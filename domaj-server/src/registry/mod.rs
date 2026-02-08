//! Docker Registry client module
//!
//! Handles communication with Docker registries (Docker Hub, Quay.io, GHCR)
//! to fetch image manifests and compare digests.

mod docker_hub;

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::config::RegistryCredential;

pub use docker_hub::DockerHubClient;

/// Information about a remote image
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteImageInfo {
    /// Full image reference (e.g., "nginx:1.25")
    pub image: String,
    /// Registry (e.g., "docker.io", "quay.io")
    pub registry: String,
    /// Repository (e.g., "library/nginx", "user/app")
    pub repository: String,
    /// Tag (e.g., "1.25", "latest")
    pub tag: String,
    /// Image digest (sha256:...)
    pub digest: Option<String>,
}

/// Parsed image reference
#[derive(Debug, Clone)]
pub struct ImageReference {
    pub registry: String,
    pub repository: String,
    pub tag: String,
}

impl ImageReference {
    /// Parse an image string into its components
    pub fn parse(image: &str) -> Self {
        let (image_part, tag) = if let Some(idx) = image.rfind(':') {
            // Check if ':' is part of a port number or a tag
            let after_colon = &image[idx + 1..];
            if after_colon.contains('/') {
                // It's a port, not a tag
                (image, "latest".to_string())
            } else {
                (&image[..idx], after_colon.to_string())
            }
        } else {
            (image, "latest".to_string())
        };

        let (registry, repository) = if image_part.contains('/') {
            let parts: Vec<&str> = image_part.splitn(2, '/').collect();
            if parts[0].contains('.') || parts[0].contains(':') {
                // Has registry
                (parts[0].to_string(), parts[1].to_string())
            } else {
                // Docker Hub with user
                ("docker.io".to_string(), image_part.to_string())
            }
        } else {
            // Official Docker Hub image
            ("docker.io".to_string(), format!("library/{}", image_part))
        };

        Self {
            registry,
            repository,
            tag,
        }
    }
}

/// Trait for registry clients
pub trait RegistryClient: Send + Sync {
    /// Get the digest for a specific image tag
    fn get_digest(
        &self,
        repository: &str,
        tag: &str,
    ) -> impl std::future::Future<Output = Result<String>> + Send;

    /// List available tags for a repository
    fn list_tags(
        &self,
        repository: &str,
    ) -> impl std::future::Future<Output = Result<Vec<String>>> + Send;
}

/// Get the appropriate registry client for an image
pub fn get_registry_client(registry: &str, credentials: Option<&RegistryCredential>) -> Box<dyn RegistryClientDyn> {
    match registry {
        "docker.io" | "registry-1.docker.io" => Box::new(DockerHubClient::new()),
        "quay.io" => Box::new(QuayClient::new()),
        "ghcr.io" => Box::new(GhcrClient::new()),
        _ => Box::new(GenericClient::new(registry, credentials.cloned())),
    }
}

/// Dynamic dispatch trait for registry clients
pub trait RegistryClientDyn: Send + Sync {
    fn get_digest_dyn(
        &self,
        repository: &str,
        tag: &str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<String>> + Send + '_>>;

    fn list_tags_dyn(
        &self,
        repository: &str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Vec<String>>> + Send + '_>>;
}

/// Quay.io registry client
pub struct QuayClient {
    client: reqwest::Client,
}

impl QuayClient {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }
}

impl RegistryClientDyn for QuayClient {
    fn get_digest_dyn(
        &self,
        repository: &str,
        tag: &str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<String>> + Send + '_>> {
        let repo = repository.to_string();
        let tag = tag.to_string();
        Box::pin(async move {
            let url = format!(
                "https://quay.io/api/v1/repository/{}/tag/?specificTag={}",
                repo, tag
            );
            let resp: serde_json::Value = self.client.get(&url).send().await?.json().await?;
            
            let digest = resp["tags"]
                .as_array()
                .and_then(|tags| tags.first())
                .and_then(|t| t["manifest_digest"].as_str())
                .map(|s| s.to_string())
                .ok_or_else(|| anyhow::anyhow!("Digest not found"))?;
            
            Ok(digest)
        })
    }

    fn list_tags_dyn(
        &self,
        repository: &str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Vec<String>>> + Send + '_>> {
        let repo = repository.to_string();
        Box::pin(async move {
            let url = format!("https://quay.io/api/v1/repository/{}/tag/", repo);
            let resp: serde_json::Value = self.client.get(&url).send().await?.json().await?;
            
            let tags = resp["tags"]
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .filter_map(|t| t["name"].as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default();
            
            Ok(tags)
        })
    }
}

/// GitHub Container Registry client
pub struct GhcrClient {
    client: reqwest::Client,
}

impl GhcrClient {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }
}

impl RegistryClientDyn for GhcrClient {
    fn get_digest_dyn(
        &self,
        repository: &str,
        tag: &str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<String>> + Send + '_>> {
        let repo = repository.to_string();
        let tag = tag.to_string();
        Box::pin(async move {
            // GHCR uses Docker Registry API v2
            let url = format!("https://ghcr.io/v2/{}/manifests/{}", repo, tag);
            let resp = self
                .client
                .get(&url)
                .header(
                    "Accept",
                    "application/vnd.docker.distribution.manifest.v2+json",
                )
                .send()
                .await?;
            
            let digest = resp
                .headers()
                .get("docker-content-digest")
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string())
                .ok_or_else(|| anyhow::anyhow!("Digest header not found"))?;
            
            Ok(digest)
        })
    }

    fn list_tags_dyn(
        &self,
        repository: &str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Vec<String>>> + Send + '_>> {
        let repo = repository.to_string();
        Box::pin(async move {
            let url = format!("https://ghcr.io/v2/{}/tags/list", repo);
            let resp: serde_json::Value = self.client.get(&url).send().await?.json().await?;
            
            let tags = resp["tags"]
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .filter_map(|t| t.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default();
            
            Ok(tags)
        })
    }
}

/// Generic OCI registry client with optional authentication
pub struct GenericClient {
    registry: String,
    credentials: Option<RegistryCredential>,
    client: reqwest::Client,
}

impl GenericClient {
    pub fn new(registry: &str, credentials: Option<RegistryCredential>) -> Self {
        Self {
            registry: registry.to_string(),
            credentials,
            client: reqwest::Client::new(),
        }
    }

    /// Parse WWW-Authenticate header to extract realm, service, scope
    fn parse_www_authenticate(header: &str) -> Option<(String, String, String)> {
        // Format: Bearer realm="https://...",service="...",scope="..."
        let header = header.strip_prefix("Bearer ")?;
        
        let mut realm = None;
        let mut service = None;
        let mut scope = None;
        
        for part in header.split(',') {
            let part = part.trim();
            if let Some(val) = part.strip_prefix("realm=") {
                realm = Some(val.trim_matches('"').to_string());
            } else if let Some(val) = part.strip_prefix("service=") {
                service = Some(val.trim_matches('"').to_string());
            } else if let Some(val) = part.strip_prefix("scope=") {
                scope = Some(val.trim_matches('"').to_string());
            }
        }
        
        Some((realm?, service.unwrap_or_default(), scope.unwrap_or_default()))
    }

    /// Obtain a Bearer token from the registry's auth service
    async fn get_bearer_token(&self, realm: &str, service: &str, scope: &str) -> Result<String> {
        let mut url = format!("{}?service={}&scope={}", realm, service, scope);
        
        // If service is empty, omit it
        if service.is_empty() {
            url = format!("{}?scope={}", realm, scope);
        }
        
        let mut request = self.client.get(&url);
        
        if let Some(cred) = &self.credentials {
            request = request.header("Authorization", cred.basic_auth());
        }
        
        let resp = request.send().await?;
        
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("Token service returned {}: {}", status, body);
        }
        
        // Harbor returns {"token": "..."} or {"access_token": "..."}
        let json: serde_json::Value = resp.json().await?;
        let token = json["token"]
            .as_str()
            .or_else(|| json["access_token"].as_str())
            .ok_or_else(|| anyhow::anyhow!("No token in auth response"))?;
        
        Ok(token.to_string())
    }

    /// Make an authenticated request to the registry, handling the OCI v2 auth challenge
    async fn authenticated_request(&self, url: &str, accept_header: &str) -> Result<reqwest::Response> {
        // First attempt — maybe the registry allows anonymous access
        let resp = self.client
            .get(url)
            .header("Accept", accept_header)
            .send()
            .await?;
        
        if resp.status() != reqwest::StatusCode::UNAUTHORIZED {
            return Ok(resp);
        }
        
        // Got 401 — parse WWW-Authenticate and get a token
        let www_auth = resp
            .headers()
            .get("www-authenticate")
            .and_then(|v| v.to_str().ok())
            .unwrap_or_default()
            .to_string();
        
        tracing::debug!("Registry auth challenge: {}", www_auth);
        
        let (realm, service, scope) = Self::parse_www_authenticate(&www_auth)
            .ok_or_else(|| anyhow::anyhow!(
                "Cannot parse WWW-Authenticate header: {}", www_auth
            ))?;
        
        let token = self.get_bearer_token(&realm, &service, &scope).await?;
        
        // Retry with the Bearer token
        let resp = self.client
            .get(url)
            .header("Accept", accept_header)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await?;
        
        Ok(resp)
    }
}

impl RegistryClientDyn for GenericClient {
    fn get_digest_dyn(
        &self,
        repository: &str,
        tag: &str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<String>> + Send + '_>> {
        let registry = self.registry.clone();
        let repo = repository.to_string();
        let tag = tag.to_string();
        Box::pin(async move {
            let url = format!("https://{}/v2/{}/manifests/{}", registry, repo, tag);
            let accept = "application/vnd.docker.distribution.manifest.list.v2+json, application/vnd.oci.image.index.v1+json, application/vnd.docker.distribution.manifest.v2+json";
            
            let resp = self.authenticated_request(&url, accept).await?;
            
            if !resp.status().is_success() {
                let status = resp.status();
                let body = resp.text().await.unwrap_or_default();
                anyhow::bail!("Registry {} returned {}: {}", registry, status, body);
            }
            
            let digest = resp
                .headers()
                .get("docker-content-digest")
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string())
                .ok_or_else(|| anyhow::anyhow!("Digest header not found"))?;
            
            Ok(digest)
        })
    }

    fn list_tags_dyn(
        &self,
        repository: &str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Vec<String>>> + Send + '_>> {
        let registry = self.registry.clone();
        let repo = repository.to_string();
        Box::pin(async move {
            let url = format!("https://{}/v2/{}/tags/list", registry, repo);
            
            let resp = self.authenticated_request(&url, "application/json").await?;
            
            if !resp.status().is_success() {
                let status = resp.status();
                let body = resp.text().await.unwrap_or_default();
                anyhow::bail!("Registry {} returned {}: {}", registry, status, body);
            }
            
            let json: serde_json::Value = resp.json().await?;
            let tags = json["tags"]
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .filter_map(|t| t.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default();
            
            Ok(tags)
        })
    }
}
