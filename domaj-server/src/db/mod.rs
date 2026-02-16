//! Database module for Domaj Server
//!
//! Handles database initialization, migrations, and models.
//! Supports both SQLite and PostgreSQL backends via sqlx::AnyPool.

mod models;

pub use models::*;

/// SQL SELECT fragments with COALESCE for nullable fields (required by sqlx::Any driver)
pub const SELECT_SERVERS: &str = "id, name, endpoint, api_key, COALESCE(agent_id, '') AS agent_id, COALESCE(last_seen, '') AS last_seen, CAST(created_at AS TEXT) AS created_at";
pub const SELECT_CONTAINERS: &str = "id, server_id, container_id, name, image, COALESCE(image_digest, '') AS image_digest, COALESCE(architecture, '') AS architecture, status, CAST(created_at AS TEXT) AS created_at, COALESCE(CAST(last_checked AS TEXT), '') AS last_checked";
pub const SELECT_UPDATE_CHECKS: &str = "id, container_id, check_type, local_digest, COALESCE(remote_digest, '') AS remote_digest, has_update, COALESCE(latest_tag, '') AS latest_tag, version_gap, CAST(checked_at AS TEXT) AS checked_at";
pub const SELECT_UPDATE_JOBS: &str = "id, container_id, container_name, server_name, image, COALESCE(target_tag, '') AS target_tag, status, COALESCE(error_message, '') AS error_message, COALESCE(previous_image, '') AS previous_image, job_type, CAST(started_at AS TEXT) AS started_at, COALESCE(CAST(completed_at AS TEXT), '') AS completed_at";
pub const SELECT_REGISTRY_CREDS: &str = "id, host, username, password, CAST(created_at AS TEXT) AS created_at";
pub const SELECT_USERS: &str = "id, username, password_hash, role, CAST(created_at AS TEXT) AS created_at";

use anyhow::Result;
use sqlx::any::AnyPoolOptions;
use sqlx::AnyPool;

/// Initialize the database connection pool and run migrations
pub async fn init_db(database_url: &str) -> Result<AnyPool> {
    // Install default drivers (sqlite + postgres)
    sqlx::any::install_default_drivers();

    // For SQLite, ensure the data directory exists
    if database_url.starts_with("sqlite:") {
        let path = database_url
            .trim_start_matches("sqlite:")
            .split('?')
            .next()
            .unwrap_or("./data/domaj.db");
        
        if let Some(parent) = std::path::Path::new(path).parent() {
            std::fs::create_dir_all(parent)?;
        }
    }

    let pool = AnyPoolOptions::new()
        .max_connections(5)
        .connect(database_url)
        .await?;
    
    // Run migrations based on detected backend
    let is_postgres = database_url.starts_with("postgres");
    run_migrations(&pool, is_postgres).await?;
    
    Ok(pool)
}

/// Run database migrations
async fn run_migrations(pool: &AnyPool, is_postgres: bool) -> Result<()> {
    if is_postgres {
        run_postgres_migrations(pool).await
    } else {
        run_sqlite_migrations(pool).await
    }
}

/// SQLite-specific migrations
async fn run_sqlite_migrations(pool: &AnyPool) -> Result<()> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS servers (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            endpoint TEXT NOT NULL,
            api_key TEXT NOT NULL,
            agent_id TEXT UNIQUE,
            last_seen TEXT,
            created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS containers (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            server_id INTEGER NOT NULL,
            container_id TEXT NOT NULL,
            name TEXT NOT NULL,
            image TEXT NOT NULL,
            image_digest TEXT,
            architecture TEXT,
            status TEXT NOT NULL,
            created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            last_checked TEXT,
            FOREIGN KEY (server_id) REFERENCES servers(id) ON DELETE CASCADE
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS update_checks (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            container_id INTEGER NOT NULL,
            check_type TEXT NOT NULL,
            local_digest TEXT NOT NULL,
            remote_digest TEXT,
            has_update INTEGER NOT NULL DEFAULT 0,
            latest_tag TEXT,
            version_gap INTEGER NOT NULL DEFAULT -1,
            checked_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (container_id) REFERENCES containers(id) ON DELETE CASCADE
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS users (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            username TEXT NOT NULL UNIQUE,
            password_hash TEXT NOT NULL,
            role TEXT NOT NULL DEFAULT 'user',
            created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_containers_server ON containers(server_id)")
        .execute(pool)
        .await?;
    
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_update_checks_container ON update_checks(container_id)")
        .execute(pool)
        .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS update_jobs (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            container_id INTEGER NOT NULL,
            container_name TEXT NOT NULL,
            server_name TEXT NOT NULL,
            image TEXT NOT NULL,
            target_tag TEXT,
            status TEXT NOT NULL DEFAULT 'pending',
            error_message TEXT,
            previous_image TEXT,
            job_type TEXT NOT NULL DEFAULT 'update',
            started_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            completed_at TEXT,
            FOREIGN KEY (container_id) REFERENCES containers(id) ON DELETE CASCADE
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_update_jobs_status ON update_jobs(status)")
        .execute(pool)
        .await?;

    // Legacy column migrations (ignored if already present)
    let _ = sqlx::query("ALTER TABLE update_jobs ADD COLUMN previous_image TEXT")
        .execute(pool)
        .await;
    let _ = sqlx::query("ALTER TABLE update_jobs ADD COLUMN job_type TEXT NOT NULL DEFAULT 'update'")
        .execute(pool)
        .await;
    let _ = sqlx::query("ALTER TABLE containers ADD COLUMN architecture TEXT")
        .execute(pool)
        .await;
    let _ = sqlx::query("ALTER TABLE update_checks ADD COLUMN version_gap INTEGER NOT NULL DEFAULT -1")
        .execute(pool)
        .await;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS registry_credentials (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            host TEXT NOT NULL UNIQUE,
            username TEXT NOT NULL,
            password TEXT NOT NULL,
            created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        )
        "#,
    )
    .execute(pool)
    .await?;

    // Insert default settings if not present
    sqlx::query("INSERT OR IGNORE INTO settings (key, value) VALUES ('auto_rollback', 'true')")
        .execute(pool)
        .await?;
    sqlx::query("INSERT OR IGNORE INTO settings (key, value) VALUES ('auto_rollback_delay', '30')")
        .execute(pool)
        .await?;

    tracing::debug!("SQLite migrations completed");
    Ok(())
}

/// PostgreSQL-specific migrations
async fn run_postgres_migrations(pool: &AnyPool) -> Result<()> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS servers (
            id BIGSERIAL PRIMARY KEY,
            name TEXT NOT NULL,
            endpoint TEXT NOT NULL,
            api_key TEXT NOT NULL,
            agent_id TEXT UNIQUE,
            last_seen TEXT,
            created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS containers (
            id BIGSERIAL PRIMARY KEY,
            server_id BIGINT NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
            container_id TEXT NOT NULL,
            name TEXT NOT NULL,
            image TEXT NOT NULL,
            image_digest TEXT,
            architecture TEXT,
            status TEXT NOT NULL,
            created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            last_checked TEXT
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_containers_server ON containers(server_id)")
        .execute(pool)
        .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS update_checks (
            id BIGSERIAL PRIMARY KEY,
            container_id BIGINT NOT NULL REFERENCES containers(id) ON DELETE CASCADE,
            check_type TEXT NOT NULL,
            local_digest TEXT NOT NULL,
            remote_digest TEXT,
            has_update INTEGER NOT NULL DEFAULT 0,
            latest_tag TEXT,
            version_gap INTEGER NOT NULL DEFAULT -1,
            checked_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_update_checks_container ON update_checks(container_id)")
        .execute(pool)
        .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS users (
            id BIGSERIAL PRIMARY KEY,
            username TEXT NOT NULL UNIQUE,
            password_hash TEXT NOT NULL,
            role TEXT NOT NULL DEFAULT 'user',
            created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS update_jobs (
            id BIGSERIAL PRIMARY KEY,
            container_id BIGINT NOT NULL REFERENCES containers(id) ON DELETE CASCADE,
            container_name TEXT NOT NULL,
            server_name TEXT NOT NULL,
            image TEXT NOT NULL,
            target_tag TEXT,
            status TEXT NOT NULL DEFAULT 'pending',
            error_message TEXT,
            previous_image TEXT,
            job_type TEXT NOT NULL DEFAULT 'update',
            started_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            completed_at TEXT
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_update_jobs_status ON update_jobs(status)")
        .execute(pool)
        .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS registry_credentials (
            id BIGSERIAL PRIMARY KEY,
            host TEXT NOT NULL UNIQUE,
            username TEXT NOT NULL,
            password TEXT NOT NULL,
            created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(pool)
    .await?;

    let _ = sqlx::query("ALTER TABLE update_checks ADD COLUMN IF NOT EXISTS version_gap INTEGER NOT NULL DEFAULT -1")
        .execute(pool)
        .await;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        )
        "#,
    )
    .execute(pool)
    .await?;

    // Insert default settings if not present
    sqlx::query("INSERT INTO settings (key, value) VALUES ('auto_rollback', 'true') ON CONFLICT (key) DO NOTHING")
        .execute(pool)
        .await?;
    sqlx::query("INSERT INTO settings (key, value) VALUES ('auto_rollback_delay', '30') ON CONFLICT (key) DO NOTHING")
        .execute(pool)
        .await?;

    tracing::debug!("PostgreSQL migrations completed");
    Ok(())
}
