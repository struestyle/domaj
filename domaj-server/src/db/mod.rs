//! Database module for Domaj Server
//!
//! Handles SQLite database initialization, migrations, and models.

mod models;

pub use models::*;

use anyhow::Result;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::SqlitePool;
use std::str::FromStr;

/// Initialize the database connection pool and run migrations
pub async fn init_db(database_url: &str) -> Result<SqlitePool> {
    // Ensure the data directory exists
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
    
    let options = SqliteConnectOptions::from_str(database_url)?
        .create_if_missing(true)
        .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal);
    
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await?;
    
    // Run migrations
    run_migrations(&pool).await?;
    
    Ok(pool)
}

/// Run database migrations
async fn run_migrations(pool: &SqlitePool) -> Result<()> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS servers (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            endpoint TEXT NOT NULL,
            api_key TEXT NOT NULL,
            agent_id TEXT UNIQUE,
            last_seen DATETIME,
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
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
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            last_checked DATETIME,
            FOREIGN KEY (server_id) REFERENCES servers(id) ON DELETE CASCADE
        )
        "#,
    )
    .execute(pool)
    .await?;

    // Add architecture column if it doesn't exist (migration for existing databases)
    sqlx::query("ALTER TABLE containers ADD COLUMN architecture TEXT")
        .execute(pool)
        .await
        .ok(); // Ignore error if column already exists

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
            checked_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
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
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(pool)
    .await?;

    // Create indices for better query performance
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_containers_server ON containers(server_id)")
        .execute(pool)
        .await?;
    
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_update_checks_container ON update_checks(container_id)")
        .execute(pool)
        .await?;

    tracing::debug!("Database migrations completed");
    Ok(())
}
