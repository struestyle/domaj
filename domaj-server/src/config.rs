//! Configuration module for Domaj Server
//!
//! Reads configuration from environment variables.

use anyhow::{Context, Result};

/// Server configuration
#[derive(Debug, Clone)]
pub struct Config {
    /// Database URL (SQLite path)
    pub database_url: String,
    
    /// Server port
    pub port: u16,
    
    /// Scan interval in cron format (e.g., "0 0 0 * * *" for daily at midnight - 6 fields with seconds)
    pub scan_interval: String,
    
    /// API secret for authenticating agents
    pub api_secret: String,
    
    /// JWT secret for user authentication
    pub jwt_secret: String,
    
    // Admin account configuration (optional, for initial setup)
    pub admin_username: Option<String>,
    pub admin_password: Option<String>,
    
    // SMTP Configuration
    pub smtp_host: Option<String>,
    pub smtp_port: u16,
    pub smtp_user: Option<String>,
    pub smtp_password: Option<String>,
    pub smtp_from: Option<String>,
    
    /// Comma-separated list of notification email addresses
    pub notify_emails: Vec<String>,
}

impl Config {
    /// Load configuration from environment variables
    pub fn from_env() -> Result<Self> {
        let database_url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "sqlite:./data/domaj.db?mode=rwc".to_string());
        
        let port = std::env::var("PORT")
            .unwrap_or_else(|_| "3000".to_string())
            .parse()
            .context("Invalid PORT")?;
        
        let scan_interval = std::env::var("SCAN_INTERVAL")
            .unwrap_or_else(|_| "0 0 0 * * *".to_string());
        
        let api_secret = std::env::var("API_SECRET")
            .unwrap_or_else(|_| {
                tracing::warn!("⚠️  API_SECRET not set, using default (insecure for production!)");
                "change-me-in-production".to_string()
            });
        
        let jwt_secret = std::env::var("JWT_SECRET")
            .unwrap_or_else(|_| {
                tracing::warn!("⚠️  JWT_SECRET not set, using default (insecure for production!)");
                "jwt-secret-change-me-in-production-32chars".to_string()
            });
        
        let smtp_host = std::env::var("SMTP_HOST").ok();
        let smtp_port = std::env::var("SMTP_PORT")
            .unwrap_or_else(|_| "587".to_string())
            .parse()
            .unwrap_or(587);
        let smtp_user = std::env::var("SMTP_USER").ok();
        let smtp_password = std::env::var("SMTP_PASSWORD").ok();
        let smtp_from = std::env::var("SMTP_FROM").ok();
        
        let notify_emails = std::env::var("NOTIFY_EMAILS")
            .unwrap_or_default()
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        
        // Admin account configuration (optional)
        let admin_username = std::env::var("ADMIN_USERNAME").ok();
        let admin_password = std::env::var("ADMIN_PASSWORD").ok();
        
        Ok(Self {
            database_url,
            port,
            scan_interval,
            api_secret,
            jwt_secret,
            admin_username,
            admin_password,
            smtp_host,
            smtp_port,
            smtp_user,
            smtp_password,
            smtp_from,
            notify_emails,
        })
    }
    
    /// Check if SMTP is properly configured
    pub fn is_smtp_configured(&self) -> bool {
        self.smtp_host.is_some() && self.smtp_from.is_some()
    }
}
