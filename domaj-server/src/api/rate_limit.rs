//! Rate limiting module for brute-force protection

use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Configuration for rate limiting
pub struct RateLimiterConfig {
    /// Maximum attempts allowed in the time window
    pub max_attempts: u32,
    /// Time window duration
    pub window_duration: Duration,
    /// Lockout duration after max attempts reached
    pub lockout_duration: Duration,
}

impl Default for RateLimiterConfig {
    fn default() -> Self {
        Self {
            max_attempts: 5,
            window_duration: Duration::from_secs(60),      // 5 attempts per minute
            lockout_duration: Duration::from_secs(300),    // 5 minute lockout
        }
    }
}

/// Track attempts for a single IP
#[derive(Debug, Clone)]
struct AttemptRecord {
    attempts: u32,
    first_attempt: Instant,
    locked_until: Option<Instant>,
}

impl AttemptRecord {
    fn new() -> Self {
        Self {
            attempts: 1,
            first_attempt: Instant::now(),
            locked_until: None,
        }
    }
}

/// Rate limiter for protecting against brute-force attacks
pub struct RateLimiter {
    records: RwLock<HashMap<IpAddr, AttemptRecord>>,
    config: RateLimiterConfig,
}

impl RateLimiter {
    pub fn new(config: RateLimiterConfig) -> Self {
        Self {
            records: RwLock::new(HashMap::new()),
            config,
        }
    }

    /// Check if an IP is allowed to make a request
    /// Returns Ok(()) if allowed, Err with seconds remaining if blocked
    pub async fn check(&self, ip: IpAddr) -> Result<(), u64> {
        let now = Instant::now();
        let mut records = self.records.write().await;

        if let Some(record) = records.get_mut(&ip) {
            // Check if currently locked out
            if let Some(locked_until) = record.locked_until {
                if now < locked_until {
                    let remaining = (locked_until - now).as_secs();
                    return Err(remaining);
                } else {
                    // Lockout expired, reset
                    record.attempts = 0;
                    record.locked_until = None;
                    record.first_attempt = now;
                }
            }

            // Check if window has expired
            if now.duration_since(record.first_attempt) > self.config.window_duration {
                record.attempts = 0;
                record.first_attempt = now;
            }
        }

        Ok(())
    }

    /// Record a failed attempt for an IP
    pub async fn record_failure(&self, ip: IpAddr) {
        let now = Instant::now();
        let mut records = self.records.write().await;

        let record = records.entry(ip).or_insert_with(AttemptRecord::new);
        
        // Check if window has expired
        if now.duration_since(record.first_attempt) > self.config.window_duration {
            record.attempts = 1;
            record.first_attempt = now;
        } else {
            record.attempts += 1;
        }

        // Check if we've exceeded max attempts
        if record.attempts >= self.config.max_attempts {
            record.locked_until = Some(now + self.config.lockout_duration);
            tracing::warn!("IP {} locked out for {} seconds due to too many failed login attempts", 
                ip, self.config.lockout_duration.as_secs());
        }
    }

    /// Record a successful attempt (resets the counter)
    pub async fn record_success(&self, ip: IpAddr) {
        let mut records = self.records.write().await;
        records.remove(&ip);
    }

    /// Cleanup old records (call periodically)
    pub async fn cleanup(&self) {
        let now = Instant::now();
        let mut records = self.records.write().await;
        
        records.retain(|_, record| {
            // Keep if locked and lockout hasn't expired
            if let Some(locked_until) = record.locked_until {
                if now < locked_until {
                    return true;
                }
            }
            // Keep if within window
            now.duration_since(record.first_attempt) < self.config.window_duration
        });
    }
}

/// Create a shared rate limiter
pub fn create_rate_limiter() -> Arc<RateLimiter> {
    Arc::new(RateLimiter::new(RateLimiterConfig::default()))
}
