//! Configuration module for Domaj Server
//!
//! Reads configuration from environment variables.

use anyhow::{Context, Result};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};

/// Credentials for a private container registry
#[derive(Debug, Clone)]
pub struct RegistryCredential {
    pub host: String,
    pub username: String,
    pub password: String,
}

impl RegistryCredential {
    /// Encode credentials as Basic auth header value
    pub fn basic_auth(&self) -> String {
        let encoded = BASE64.encode(format!("{}:{}", self.username, self.password));
        format!("Basic {}", encoded)
    }
}

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
    
    // Telegram Configuration
    pub telegram_bot_token: Option<String>,
    pub telegram_chat_ids: Vec<String>,
    
    /// Private registry credentials
    pub registry_credentials: Vec<RegistryCredential>,
    
    /// Auto-rollback if container exits after update (None = not set by env, modifiable via UI)
    pub auto_rollback: Option<bool>,
    
    /// Delay in seconds before checking container health after update
    pub auto_rollback_delay_secs: u64,
    
    /// Docker Hub username (optional, for higher rate limits)
    pub docker_username: Option<String>,
    
    /// Docker Hub password/token (optional, for higher rate limits)
    pub docker_password: Option<String>,
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
        
        let telegram_bot_token = std::env::var("TELEGRAM_BOT_TOKEN").ok().filter(|s| !s.is_empty());
        let telegram_chat_ids: Vec<String> = std::env::var("TELEGRAM_CHAT_IDS")
            .unwrap_or_default()
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        
        // Admin account configuration (optional)
        let admin_username = std::env::var("ADMIN_USERNAME").ok();
        let admin_password = std::env::var("ADMIN_PASSWORD").ok();
        
        // Parse registry credentials (REGISTRY_1_HOST, REGISTRY_1_USER, REGISTRY_1_PASSWORD, ...)
        let mut registry_credentials = Vec::new();
        for i in 1..=10 {
            let host = std::env::var(format!("REGISTRY_{}_HOST", i));
            let user = std::env::var(format!("REGISTRY_{}_USER", i));
            let pass = std::env::var(format!("REGISTRY_{}_PASSWORD", i));
            
            if let (Ok(host), Ok(username), Ok(password)) = (host, user, pass) {
                if !host.is_empty() {
                    tracing::info!("🔐 Loaded credentials for registry: {}", host);
                    registry_credentials.push(RegistryCredential { host, username, password });
                }
            }
        }
        
        // Auto-rollback configuration
        let auto_rollback = std::env::var("AUTO_ROLLBACK").ok().map(|v| {
            v.to_lowercase() == "true" || v == "1"
        });
        
        let auto_rollback_delay_secs = std::env::var("AUTO_ROLLBACK_DELAY")
            .unwrap_or_else(|_| "30".to_string())
            .parse()
            .unwrap_or(30);
        
        if auto_rollback.is_some() {
            tracing::info!("🔄 Auto-rollback configured via env: enabled={}, delay={}s", 
                auto_rollback.unwrap(), auto_rollback_delay_secs);
        }
        
        // Docker Hub credentials
        let docker_username = std::env::var("DOCKER_USERNAME").ok().filter(|s| !s.is_empty());
        let docker_password = std::env::var("DOCKER_PASSWORD").ok().filter(|s| !s.is_empty());
        
        if docker_username.is_some() && docker_password.is_some() {
            tracing::info!("🐳 Docker Hub credentials loaded from environment (user: {})", docker_username.as_ref().unwrap());
        }
        
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
            telegram_bot_token,
            telegram_chat_ids,
            registry_credentials,
            auto_rollback,
            auto_rollback_delay_secs,
            docker_username,
            docker_password,
        })
    }
    
    /// Check if SMTP is properly configured
    pub fn is_smtp_configured(&self) -> bool {
        self.smtp_host.is_some() && self.smtp_from.is_some()
    }
    
    /// Check if Telegram is properly configured
    pub fn is_telegram_configured(&self) -> bool {
        self.telegram_bot_token.is_some() && !self.telegram_chat_ids.is_empty()
    }
}
