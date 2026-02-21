//! Database models for Domaj Server

use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// A registered user
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct User {
    pub id: i64,
    pub username: String,
    #[serde(skip_serializing)]
    pub password_hash: String,
    pub role: String,
    pub created_at: String,
}

/// User response without password hash
#[derive(Debug, Serialize)]
pub struct UserResponse {
    pub id: i64,
    pub username: String,
    pub role: String,
    pub created_at: String,
}

impl From<User> for UserResponse {
    fn from(user: User) -> Self {
        Self {
            id: user.id,
            username: user.username,
            role: user.role,
            created_at: user.created_at,
        }
    }
}

/// A registered server with a Domaj agent
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Server {
    pub id: i64,
    pub name: String,
    pub endpoint: String,
    pub api_key: String,
    pub agent_id: String,
    pub last_seen: String,
    pub created_at: String,
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
    pub image_digest: String,
    pub architecture: String,
    pub status: String,
    pub created_at: String,
    pub last_checked: String,
}

/// Container with server information for API responses
#[derive(Debug, Clone, Serialize)]
#[allow(dead_code)]
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
    pub remote_digest: String,
    pub has_update: i32,
    pub latest_tag: String,
    pub version_gap: i32,
    pub checked_at: String,
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
    pub image_digest: String,
    pub server_name: String,
    pub same_tag_update: bool,
    pub same_tag_digest: String,
    pub latest_update: bool,
    pub latest_tag: String,
    pub latest_digest: String,
    pub versions_behind: i32,
    pub last_checked: String,
}

/// Criticality level for updates
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[allow(dead_code)]
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

/// An update job tracking entry
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct UpdateJob {
    pub id: i64,
    pub container_id: i64,
    pub container_name: String,
    pub server_name: String,
    pub image: String,
    pub target_tag: String,
    pub status: String,
    pub error_message: String,
    pub previous_image: String,
    pub job_type: String,
    pub started_at: String,
    pub completed_at: String,
}

/// Registry credential stored in the database (managed via UI)
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct DbRegistryCredential {
    pub id: i64,
    pub host: String,
    pub username: String,
    #[serde(skip_serializing)]
    pub password: String,
    pub created_at: String,
}

/// An audit log entry
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AuditLog {
    pub id: i64,
    pub username: String,
    pub action: String,
    pub details: String,
    pub created_at: String,
}
