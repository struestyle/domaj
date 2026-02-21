//! Scheduler module for periodic scans
//!
//! Handles cron-based scheduling of container update checks.

use std::collections::HashMap;
use std::sync::Arc;
use anyhow::Result;
use tokio_cron_scheduler::{Job, JobScheduler};

use crate::db::{CheckType, Container, Server};
use crate::registry::{get_registry_client, ImageReference, RegistryClientDyn};
use crate::AppState;

/// Cache for digest lookups: (repository, tag) -> digest
/// Shared across all container checks during a single scan to avoid
/// redundant API calls and Docker Hub rate limiting (429).
type DigestCache = HashMap<(String, String), String>;

/// Scheduler wrapper
pub struct Scheduler {
    inner: Option<JobScheduler>,
}

impl Scheduler {
    pub fn new() -> Self {
        Self { inner: None }
    }

    /// Start the scheduler with the configured interval
    pub async fn start(&mut self, state: Arc<AppState>) -> Result<()> {
        let sched = JobScheduler::new().await?;
        
        let cron_expr = &state.config.scan_interval;
        let state_clone = state.clone();
        
        let job = Job::new_async(cron_expr.as_str(), move |_uuid, _lock| {
            let state = state_clone.clone();
            Box::pin(async move {
                tracing::info!("🔍 Starting scheduled scan...");
                if let Err(e) = run_scan(&state).await {
                    tracing::error!("Scheduled scan failed: {}", e);
                }
            })
        })?;

        sched.add(job).await?;
        sched.start().await?;
        
        self.inner = Some(sched);
        
        tracing::info!("📅 Scheduler started with cron expression: {}", cron_expr);
        Ok(())
    }
}

impl Default for Scheduler {
    fn default() -> Self {
        Self::new()
    }
}

/// Run a full scan of all servers and containers
pub async fn run_scan(state: &AppState) -> Result<()> {
    let scan_start = std::time::Instant::now();
    tracing::info!("Starting full scan...");
    
    // Get all servers
    let servers: Vec<Server> = sqlx::query_as(&format!("SELECT {} FROM servers", crate::db::SELECT_SERVERS))
        .fetch_all(&state.db)
        .await?;

    tracing::info!("Found {} servers to scan", servers.len());

    let mut total_updates = 0;
    let mut digest_cache = DigestCache::new();

    for server in &servers {
        let server_start = std::time::Instant::now();
        tracing::info!("Scanning server: {}", server.name);
        
        // Sync containers from agent first
        let sync_start = std::time::Instant::now();
        if let Err(e) = sync_server_containers(state, server).await {
            tracing::warn!("Failed to sync server {}: {} (took {:?})", server.name, e, sync_start.elapsed());
            continue;
        }
        tracing::info!("⏱️  Sync {} took {:?}", server.name, sync_start.elapsed());

        // Get containers for this server
        let containers: Vec<Container> = sqlx::query_as(
            &format!("SELECT {} FROM containers WHERE server_id = $1", crate::db::SELECT_CONTAINERS),
        )
        .bind(server.id)
        .fetch_all(&state.db)
        .await?;

        for container in &containers {
            let container_start = std::time::Instant::now();
            match check_container_updates(state, container, &mut digest_cache).await {
                Ok(has_update) => {
                    let elapsed = container_start.elapsed();
                    if elapsed.as_millis() > 500 {
                        tracing::warn!("⏱️  Check {} took {:?} (SLOW)", container.name, elapsed);
                    } else {
                        tracing::info!("⏱️  Check {} took {:?}", container.name, elapsed);
                    }
                    if has_update {
                        total_updates += 1;
                    }
                }
                Err(e) => {
                    let err_msg = e.to_string();
                    if err_msg.contains("429") || err_msg.contains("TOOMANYREQUESTS") {
                        tracing::error!("🚫 Docker Hub rate limit hit — skipping remaining containers for {}", server.name);
                        break;
                    }
                    tracing::warn!(
                        "Failed to check updates for {}/{}: {} (took {:?})",
                        server.name,
                        container.name,
                        e,
                        container_start.elapsed()
                    );
                }
            }
        }
        tracing::info!("⏱️  Server {} total: {:?} ({} containers)", server.name, server_start.elapsed(), containers.len());
    }

    tracing::info!("⏱️  SCAN COMPLETE in {:?}. Found {} containers with updates. Cache: {} entries", scan_start.elapsed(), total_updates, digest_cache.len());

    // Broadcast scan completed event
    let _ = state.broadcast_tx.send(serde_json::json!({
        "type": "scan_completed",
        "total_updates": total_updates
    }).to_string());

    // Send notification if there are updates
    if total_updates > 0 {
        if let Err(e) = crate::notifier::send_update_report(state).await {
            tracing::error!("Failed to send notification: {}", e);
        }
    }

    Ok(())
}

/// Sync containers from a server's agent
async fn sync_server_containers(state: &AppState, server: &Server) -> Result<()> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()?;
    
    let agent_url = format!("{}/containers", server.endpoint.trim_end_matches('/'));
    
    let response = client
        .get(&agent_url)
        .header("X-API-Key", &state.config.api_secret)
        .send()
        .await?;

    if !response.status().is_success() {
        anyhow::bail!("Agent returned status: {}", response.status());
    }

    #[derive(serde::Deserialize)]
    struct AgentContainer {
        id: String,
        name: String,
        image: String,
        image_digest: Option<String>,
        architecture: Option<String>,
        status: String,
    }

    let agent_containers: Vec<AgentContainer> = response.json().await?;

    // Upsert containers (preserve existing IDs to keep update_jobs/update_checks)
    let agent_container_ids: Vec<&str> = agent_containers.iter().map(|c| c.id.as_str()).collect();

    for c in &agent_containers {
        // Try to update existing container first
        let updated = sqlx::query(
            "UPDATE containers SET name = $1, image = $2, image_digest = $3, architecture = $4, status = $5 WHERE server_id = $6 AND container_id = $7",
        )
        .bind(&c.name)
        .bind(&c.image)
        .bind(c.image_digest.as_deref().unwrap_or(""))
        .bind(c.architecture.as_deref().unwrap_or(""))
        .bind(&c.status)
        .bind(server.id)
        .bind(&c.id)
        .execute(&state.db)
        .await?;

        if updated.rows_affected() == 0 {
            // Container doesn't exist yet, insert it
            sqlx::query(
                "INSERT INTO containers (server_id, container_id, name, image, image_digest, architecture, status) VALUES ($1, $2, $3, $4, $5, $6, $7)",
            )
            .bind(server.id)
            .bind(&c.id)
            .bind(&c.name)
            .bind(&c.image)
            .bind(c.image_digest.as_deref().unwrap_or(""))
            .bind(c.architecture.as_deref().unwrap_or(""))
            .bind(&c.status)
            .execute(&state.db)
            .await?;
        }
    }

    // Remove containers that no longer exist on the agent
    let existing: Vec<(String,)> = sqlx::query_as(
        "SELECT container_id FROM containers WHERE server_id = $1",
    )
    .bind(server.id)
    .fetch_all(&state.db)
    .await?;

    for (cid,) in &existing {
        if !agent_container_ids.contains(&cid.as_str()) {
            sqlx::query("DELETE FROM containers WHERE server_id = $1 AND container_id = $2")
                .bind(server.id)
                .bind(cid)
                .execute(&state.db)
                .await?;
        }
    }

    // Update last_seen
    sqlx::query("UPDATE servers SET last_seen = CURRENT_TIMESTAMP WHERE id = $1")
        .bind(server.id)
        .execute(&state.db)
        .await?;

    Ok(())
}

/// Parse a tag string into a semver-like tuple (major, minor, patch)
/// Handles formats: "1", "1.2", "1.2.3", with optional prefix like "v1.2.3"
fn parse_semver(tag: &str) -> Option<(u64, u64, u64)> {
    let tag = tag.strip_prefix('v').unwrap_or(tag);
    // Strip suffixes like "-alpine", "-slim", "-bullseye" for comparison
    let version_part = tag.split('-').next().unwrap_or(tag);
    let parts: Vec<&str> = version_part.split('.').collect();
    
    match parts.len() {
        1 => {
            let major = parts[0].parse().ok()?;
            Some((major, 0, 0))
        }
        2 => {
            let major = parts[0].parse().ok()?;
            let minor = parts[1].parse().ok()?;
            Some((major, minor, 0))
        }
        3 => {
            let major = parts[0].parse().ok()?;
            let minor = parts[1].parse().ok()?;
            let patch = parts[2].parse().ok()?;
            Some((major, minor, patch))
        }
        _ => None,
    }
}

/// Count how many versions are newer than `current_tag` in the given tag list.
/// Only considers tags with the same "family" (depth + suffix pattern).
fn count_newer_versions(current_tag: &str, all_tags: &[String]) -> i32 {
    let current = match parse_semver(current_tag) {
        Some(v) => v,
        None => return -1, // Not a semver tag
    };

    // Normalize suffix: hex-only suffixes (commit hashes) are treated as no suffix
    let normalize_suffix = |s: &str| -> String {
        let stripped = s.strip_prefix('v').unwrap_or(s);
        let suffix = stripped.split_once('-').map(|(_, s)| s).unwrap_or("");
        // If suffix is all hex characters, treat it as build metadata (no family)
        if !suffix.is_empty() && suffix.chars().all(|c| c.is_ascii_hexdigit()) {
            String::new()
        } else {
            suffix.to_string()
        }
    };

    let current_suffix = normalize_suffix(current_tag);
    let stripped = current_tag.strip_prefix('v').unwrap_or(current_tag);
    let depth = stripped.split('-').next().unwrap_or("").split('.').count();

    let mut newer_versions: Vec<(u64, u64, u64)> = Vec::new();

    for tag in all_tags {
        let t = tag.strip_prefix('v').unwrap_or(tag);
        
        // Check normalized suffix matches
        let tag_suffix = normalize_suffix(tag);
        if tag_suffix != current_suffix {
            continue;
        }

        // Check depth matches
        let tag_depth = t.split('-').next().unwrap_or("").split('.').count();
        if tag_depth != depth {
            continue;
        }

        if let Some(ver) = parse_semver(tag) {
            if ver > current && !newer_versions.contains(&ver) {
                newer_versions.push(ver);
            }
        }
    }

    newer_versions.len() as i32
}

/// Resolve the effective semver tag for a given digest.
/// When the container's tag is not semver-parseable (e.g. "latest", "alpine"),
/// find other tags in the repo that share the same digest and pick the most precise one.
async fn resolve_effective_tag(
    client: &dyn RegistryClientDyn,
    repository: &str,
    current_tag: &str,
    digest: &str,
    all_tags: &[String],
    digest_cache: &mut DigestCache,
) -> String {
    // If current tag is already semver-parseable, use it directly
    if parse_semver(current_tag).is_some() {
        return current_tag.to_string();
    }

    if digest.is_empty() {
        return current_tag.to_string();
    }

    // Determine the suffix pattern to match.
    // For "alpine" -> look for tags ending with "-alpine" (like "7.4.2-alpine")
    // For "latest" -> look for tags without suffix (like "7.4.2")
    let suffix_pattern = if current_tag == "latest" {
        None // match tags without any suffix
    } else {
        Some(format!("-{}", current_tag)) // e.g. "-alpine"
    };

    // Filter semver-parseable tags that match the suffix pattern
    let mut semver_tags: Vec<&String> = all_tags
        .iter()
        .filter(|t| {
            if parse_semver(t).is_none() {
                return false;
            }
            match &suffix_pattern {
                Some(suffix) => t.ends_with(suffix),
                None => {
                    // For "latest" resolution: include tags without suffix,
                    // AND tags with build-metadata suffixes (hex commit hashes like "2026.2.14-39ac4d438")
                    // Exclude variant suffixes like "-alpine", "-slim", "-bullseye"
                    if !t.contains('-') {
                        return true; // no suffix at all
                    }
                    // Check if the suffix after the version part is a build identifier (all hex)
                    let stripped = t.strip_prefix('v').unwrap_or(t);
                    if let Some((_, suffix)) = stripped.split_once('-') {
                        suffix.chars().all(|c| c.is_ascii_hexdigit())
                    } else {
                        true
                    }
                }
            }
        })
        .collect();

    // Sort by version descending so we check newest first
    semver_tags.sort_by(|a, b| {
        let va = parse_semver(a).unwrap_or((0, 0, 0));
        let vb = parse_semver(b).unwrap_or((0, 0, 0));
        vb.cmp(&va)
    });

    // Limit to avoid too many API calls — since tags are sorted by version
    // descending, the first match will be the newest, which is what we want.
    let check_limit = 15;
    let tags_to_check: Vec<&String> = semver_tags.into_iter().take(check_limit).collect();

    tracing::debug!(
        "🔍 Resolving '{}': checking {} candidate tags against digest {}...",
        current_tag,
        tags_to_check.len(),
        &digest[..20.min(digest.len())]
    );

    // Check digests using cache — stop at first match (best version since sorted desc)
    let mut best_match: Option<String> = None;
    for tag in &tags_to_check {
        match cached_get_digest(client, repository, tag, digest_cache).await {
            Ok(tag_digest) => {
                if tag_digest == digest {
                    best_match = Some(tag.to_string());
                    break; // First match = newest version, no need to continue
                }
            }
            Err(_) => {} // skip tags we can't resolve
        }
    }

    // Return the best match or fall back to current tag
    best_match.unwrap_or_else(|| current_tag.to_string())
}

/// Get a digest from cache, or fetch from registry and cache the result.
async fn cached_get_digest(
    client: &dyn RegistryClientDyn,
    repository: &str,
    tag: &str,
    cache: &mut DigestCache,
) -> Result<String> {
    let key = (repository.to_string(), tag.to_string());
    if let Some(cached) = cache.get(&key) {
        return Ok(cached.clone());
    }
    
    let digest = client.get_digest_dyn(repository, tag).await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    cache.insert(key, digest.clone());
    Ok(digest)
}

/// Check for updates for a single container
async fn check_container_updates(state: &AppState, container: &Container, digest_cache: &mut DigestCache) -> Result<bool> {
    let image_ref = ImageReference::parse(&container.image);

    // Skip local/compose images that don't exist on any registry
    // These are images without a '/' in the name that get wrongly parsed as library/... on Docker Hub
    if image_ref.registry == "docker.io" && image_ref.repository.starts_with("library/") {
        let short_name = image_ref.repository.strip_prefix("library/").unwrap_or(&image_ref.repository);
        // Official Docker Hub images have simple names (nginx, redis, postgres, etc.)
        // Local/compose images typically contain underscores or hyphens from project names
        if short_name.contains('_') {
            tracing::debug!("⏭️  Skipping local/compose image: {} (parsed as {})", container.image, image_ref.repository);
            return Ok(false);
        }
    }

    // Look up credentials for this registry (merged env + DB)
    let all_credentials = crate::api::registries::get_all_credentials(state).await;
    let credentials = all_credentials
        .iter()
        .find(|c| c.host == image_ref.registry);
    
    let client = get_registry_client(&image_ref.registry, credentials);
    
    let mut has_any_update = false;

    // Fetch all tags for version gap calculation
    let t = std::time::Instant::now();
    let all_tags = match client.list_tags_dyn(&image_ref.repository).await {
        Ok(tags) => {
            tracing::debug!("  📋 list_tags {} ({} tags) took {:?}", container.image, tags.len(), t.elapsed());
            tags
        }
        Err(e) => {
            tracing::debug!("  📋 list_tags {} failed ({:?}): {}", container.image, t.elapsed(), e);
            Vec::new()
        }
    };

    // Resolve effective tag: if current tag isn't semver (e.g. "latest"),
    // find the most precise semver tag that shares the same digest
    // We need TWO resolutions:
    // 1. LOCAL effective tag (using local digest) - for version gap calculation
    // 2. REMOTE effective tag (using remote digest) - for display purposes
    let t = std::time::Instant::now();
    let local_effective_tag = resolve_effective_tag(
        client.as_ref(),
        &image_ref.repository,
        &image_ref.tag,
        &container.image_digest,
        &all_tags,
        digest_cache,
    ).await;
    let resolve_elapsed = t.elapsed();
    if resolve_elapsed.as_millis() > 500 {
        tracing::warn!("  🏷️  resolve_effective_tag {} took {:?} (SLOW)", container.name, resolve_elapsed);
    }

    // Calculate version gap using the LOCAL effective tag
    let version_gap = if all_tags.is_empty() {
        -1
    } else {
        count_newer_versions(&local_effective_tag, &all_tags)
    };

    if version_gap > 0 {
        tracing::info!(
            "📊 {} is {} version(s) behind (local effective: {}, original: {})",
            container.name,
            version_gap,
            local_effective_tag,
            image_ref.tag
        );
    } else if version_gap == 0 && parse_semver(&local_effective_tag).is_some() {
        tracing::info!(
            "✅ {} is up to date (local effective: {})",
            container.name,
            local_effective_tag
        );
    }

    // Check 1: Same tag comparison
    let t = std::time::Instant::now();
    match cached_get_digest(client.as_ref(), &image_ref.repository, &image_ref.tag, digest_cache).await {
        Ok(remote_digest) => {
            let local_digest = container.image_digest.clone();
            let has_update = !local_digest.is_empty() && local_digest != remote_digest;
            
            sqlx::query(
                "INSERT INTO update_checks (container_id, check_type, local_digest, remote_digest, has_update, version_gap) VALUES ($1, $2, $3, $4, $5, $6)",
            )
            .bind(container.id)
            .bind(CheckType::SameTag.to_string())
            .bind(&local_digest)
            .bind(&remote_digest)
            .bind(has_update as i32)
            .bind(version_gap)
            .execute(&state.db)
            .await?;
            tracing::debug!("  🔍 same_tag digest {} took {:?}", container.name, t.elapsed());

            if has_update {
                tracing::info!(
                    "🔄 Update available for {} (same tag): {} -> {}",
                    container.name,
                    &local_digest[..12.min(local_digest.len())],
                    &remote_digest[..12.min(remote_digest.len())]
                );
                has_any_update = true;
            }
        }
        Err(e) => {
            tracing::warn!("Failed to get digest for {}: {}", container.image, e);
            // Fallback: insert a row based on version_gap alone so the container
            // still appears in the dashboard even if digest fetch is rate-limited
            if version_gap > 0 {
                let local_digest = container.image_digest.clone();
                // Try to get the remote digest from cache — we want the NEWEST tag's digest
                // (resolve_effective_tag may have cached some tags during its search)
                let mut remote_digest = String::new();
                // Look for the newest semver tag's digest in the cache
                for tag in &all_tags {
                    if parse_semver(tag).is_some() {
                        let cache_key = (image_ref.repository.clone(), tag.clone());
                        if let Some(cached) = digest_cache.get(&cache_key) {
                            if *cached != local_digest {
                                remote_digest = cached.clone();
                                break;
                            }
                        }
                    }
                }
                // Fall back to last known digest from a previous scan
                if remote_digest.is_empty() {
                    let previous_digest: Option<(String,)> = sqlx::query_as(
                        "SELECT remote_digest FROM update_checks WHERE container_id = $1 AND check_type = $2 AND remote_digest != '' ORDER BY checked_at DESC LIMIT 1",
                    )
                    .bind(container.id)
                    .bind(CheckType::SameTag.to_string())
                    .fetch_optional(&state.db)
                    .await
                    .unwrap_or(None);
                    remote_digest = previous_digest.map(|(d,)| d).unwrap_or_default();
                }

                sqlx::query(
                    "INSERT INTO update_checks (container_id, check_type, local_digest, remote_digest, has_update, version_gap) VALUES ($1, $2, $3, $4, $5, $6)",
                )
                .bind(container.id)
                .bind(CheckType::SameTag.to_string())
                .bind(&local_digest)
                .bind(&remote_digest)
                .bind(1)
                .bind(version_gap)
                .execute(&state.db)
                .await?;
                has_any_update = true;
            }
        }
    }

    // Check 2: Latest tag comparison (if not already using latest)
    if image_ref.tag != "latest" {
        match cached_get_digest(client.as_ref(), &image_ref.repository, "latest", digest_cache).await {
            Ok(latest_digest) => {
                let local_digest = container.image_digest.clone();
                let has_update = !local_digest.is_empty() && local_digest != latest_digest;
                
                sqlx::query(
                    "INSERT INTO update_checks (container_id, check_type, local_digest, remote_digest, has_update, latest_tag, version_gap) VALUES ($1, $2, $3, $4, $5, $6, $7)",
                )
                .bind(container.id)
                .bind(CheckType::Latest.to_string())
                .bind(&local_digest)
                .bind(&latest_digest)
                .bind(has_update as i32)
                .bind("latest")
                .bind(version_gap)
                .execute(&state.db)
                .await?;

                if has_update {
                    tracing::info!(
                        "🆕 Newer 'latest' available for {} (current: {})",
                        container.name,
                        image_ref.tag
                    );
                    has_any_update = true;
                }
            }
            Err(e) => {
                tracing::warn!("Failed to get latest digest for {}: {}", container.image, e);
                // Try to reuse the last known remote digest for latest check
                let previous_latest: Option<(String, String)> = sqlx::query_as(
                    "SELECT remote_digest, COALESCE(latest_tag, 'latest') FROM update_checks WHERE container_id = $1 AND check_type = $2 AND remote_digest != '' ORDER BY checked_at DESC LIMIT 1",
                )
                .bind(container.id)
                .bind(CheckType::Latest.to_string())
                .fetch_optional(&state.db)
                .await
                .unwrap_or(None);

                if let Some((prev_digest, prev_tag)) = previous_latest {
                    let local_digest = container.image_digest.clone();
                    let has_update = !local_digest.is_empty() && local_digest != prev_digest;
                    sqlx::query(
                        "INSERT INTO update_checks (container_id, check_type, local_digest, remote_digest, has_update, latest_tag, version_gap) VALUES ($1, $2, $3, $4, $5, $6, $7)",
                    )
                    .bind(container.id)
                    .bind(CheckType::Latest.to_string())
                    .bind(&local_digest)
                    .bind(&prev_digest)
                    .bind(has_update as i32)
                    .bind(&prev_tag)
                    .bind(version_gap)
                    .execute(&state.db)
                    .await?;

                    if has_update {
                        has_any_update = true;
                    }
                }
            }
        }
    }

    // Update last_checked
    sqlx::query("UPDATE containers SET last_checked = CURRENT_TIMESTAMP WHERE id = $1")
        .bind(container.id)
        .execute(&state.db)
        .await?;

    Ok(has_any_update)
}
