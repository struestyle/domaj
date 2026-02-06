//! Database models for Domaj Server

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// A registered server with a Domaj agent
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Server {
    pub id: i64,
    pub name: String,
    pub endpoint: String,
    pub api_key: String,
    pub agent_id: Option<String>,
    pub last_seen: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// Input for creating a new server
#[derive(Debug, Deserialize)]
pub struct CreateServer {
    pub name: String,
    pub endpoint: String,
}

/// A Docker container running on a server
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Container {
    pub id: i64,
    pub server_id: i64,
    pub container_id: String,
    pub name: String,
    pub image: String,
    pub image_digest: Option<String>,
    pub architecture: Option<String>,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub last_checked: Option<DateTime<Utc>>,
}

/// Container with server information for API responses
#[derive(Debug, Clone, Serialize)]
pub struct ContainerWithServer {
    #[serde(flatten)]
    pub container: Container,
    pub server_name: String,
}

/// Result of an update check
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct UpdateCheck {
    pub id: i64,
    pub container_id: i64,
    pub check_type: String,
    pub local_digest: String,
    pub remote_digest: Option<String>,
    pub has_update: bool,
    pub latest_tag: Option<String>,
    pub checked_at: DateTime<Utc>,
}

/// Type of update check
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckType {
    /// Compare same tag between local and remote
    SameTag,
    /// Compare current tag with latest available
    Latest,
}

impl std::fmt::Display for CheckType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CheckType::SameTag => write!(f, "same_tag"),
            CheckType::Latest => write!(f, "latest"),
        }
    }
}

impl std::str::FromStr for CheckType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "same_tag" => Ok(CheckType::SameTag),
            "latest" => Ok(CheckType::Latest),
            _ => Err(anyhow::anyhow!("Invalid check type: {}", s)),
        }
    }
}

/// Summary of update status for a container
#[derive(Debug, Clone, Serialize)]
pub struct UpdateSummary {
    pub container_id: i64,
    pub container_name: String,
    pub image: String,
    pub image_digest: Option<String>,
    pub server_name: String,
    pub same_tag_update: bool,
    pub same_tag_digest: Option<String>,
    pub latest_update: bool,
    pub latest_tag: Option<String>,
    pub latest_digest: Option<String>,
    pub last_checked: Option<DateTime<Utc>>,
}

/// Criticality level for updates
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Criticality {
    /// Patch version update (x.x.PATCH)
    Low,
    /// Minor version update (x.MINOR.x)
    Medium,
    /// Major version update (MAJOR.x.x)
    High,
    /// Unknown/unversioned update
    Unknown,
}
