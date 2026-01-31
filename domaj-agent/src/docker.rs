//! Docker client module
//!
//! Interfaces with the local Docker daemon via the Unix socket.

use anyhow::Result;
use bollard::container::ListContainersOptions;
use bollard::Docker;
use serde::Serialize;
use std::collections::HashMap;

/// Information about a Docker container
#[derive(Debug, Clone, Serialize)]
pub struct ContainerInfo {
    /// Container ID (short form)
    pub id: String,
    /// Container name (without leading /)
    pub name: String,
    /// Image reference (e.g., "nginx:1.25")
    pub image: String,
    /// Image digest (sha256:...)
    pub image_digest: Option<String>,
    /// Container status (e.g., "running", "exited")
    pub status: String,
    /// Container state (e.g., "running", "exited")
    pub state: String,
    /// Created timestamp
    pub created: i64,
    /// Port mappings
    pub ports: Vec<PortMapping>,
    /// Labels
    pub labels: HashMap<String, String>,
}

/// Port mapping information
#[derive(Debug, Clone, Serialize)]
pub struct PortMapping {
    pub container_port: u16,
    pub host_port: Option<u16>,
    pub protocol: String,
}

/// Docker daemon client wrapper
pub struct DockerClient {
    docker: Docker,
}

impl DockerClient {
    /// Create a new Docker client
    pub async fn new() -> Result<Self> {
        let docker = Docker::connect_with_local_defaults()?;
        
        // Verify connection
        docker.ping().await?;
        
        Ok(Self { docker })
    }

    /// List all containers (running and stopped)
    pub async fn list_containers(&self) -> Result<Vec<ContainerInfo>> {
        let options = ListContainersOptions::<String> {
            all: true,
            ..Default::default()
        };

        let containers = self.docker.list_containers(Some(options)).await?;

        let infos: Vec<ContainerInfo> = containers
            .into_iter()
            .map(|c| {
                let id = c.id.unwrap_or_default();
                let short_id = if id.len() > 12 { &id[..12] } else { &id };
                
                let name = c
                    .names
                    .and_then(|n| n.first().cloned())
                    .unwrap_or_default()
                    .trim_start_matches('/')
                    .to_string();

                let image = c.image.unwrap_or_default();
                let image_digest = c.image_id;
                
                let status = c.status.unwrap_or_default();
                let state = c.state.unwrap_or_default();
                let created = c.created.unwrap_or(0);

                let ports = c
                    .ports
                    .unwrap_or_default()
                    .into_iter()
                    .map(|p| {
                        PortMapping {
                            container_port: p.private_port,
                            host_port: p.public_port,
                            protocol: p.typ.map(|t| format!("{:?}", t).to_lowercase()).unwrap_or_else(|| "tcp".to_string()),
                        }
                    })
                    .collect();

                let labels = c.labels.unwrap_or_default();

                ContainerInfo {
                    id: short_id.to_string(),
                    name,
                    image,
                    image_digest,
                    status,
                    state,
                    created,
                    ports,
                    labels,
                }
            })
            .collect();

        Ok(infos)
    }

    /// Get a specific container by ID or name
    pub async fn get_container(&self, id: &str) -> Result<Option<ContainerInfo>> {
        let containers = self.list_containers().await?;
        
        Ok(containers
            .into_iter()
            .find(|c| c.id.starts_with(id) || c.name == id))
    }
}
