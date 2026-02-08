//! Docker client module
//!
//! Interfaces with the local Docker daemon via the Unix socket.

use anyhow::{anyhow, Result};
use bollard::auth::DockerCredentials;
use bollard::container::{
    Config, CreateContainerOptions, ListContainersOptions, RemoveContainerOptions,
    StartContainerOptions, StopContainerOptions,
};
use bollard::image::CreateImageOptions;
use bollard::Docker;
use futures_util::StreamExt;
use serde::Serialize;
use std::collections::HashMap;

use crate::RegistryCredential;

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
    /// Image architecture (e.g., "amd64", "arm64")
    pub architecture: Option<String>,
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

/// Result of a container update operation
#[derive(Debug, Clone, Serialize)]
pub struct UpdateResult {
    pub success: bool,
    pub message: String,
    pub old_image: String,
    pub new_image: String,
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
        let mut infos = Vec::new();

        for c in containers {
            let id = c.id.clone().unwrap_or_default();
            let short_id = if id.len() > 12 { &id[..12] } else { &id };
            
            let name = c
                .names
                .as_ref()
                .and_then(|n| n.first().cloned())
                .unwrap_or_default()
                .trim_start_matches('/')
                .to_string();

            let image = c.image.clone().unwrap_or_default();
            
            // Get the RepoDigest and architecture from image inspection
            let (image_digest, architecture) = if let Some(image_id) = &c.image_id {
                match self.docker.inspect_image(image_id).await {
                    Ok(inspect) => {
                        // RepoDigests contains entries like "postgres@sha256:1090bc3a..."
                        let digest = inspect.repo_digests
                            .and_then(|digests| {
                                digests.first().and_then(|d| {
                                    d.split('@').nth(1).map(|s| s.to_string())
                                })
                            });
                        // Get architecture from image inspection
                        let arch = inspect.architecture;
                        (digest, arch)
                    }
                    Err(_) => (c.image_id.clone(), None), // Fallback
                }
            } else {
                (None, None)
            };
            
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

            infos.push(ContainerInfo {
                id: short_id.to_string(),
                name,
                image,
                image_digest,
                architecture,
                status,
                state,
                created,
                ports,
                labels,
            });
        }

        Ok(infos)
    }

    /// Get a specific container by ID or name
    pub async fn get_container(&self, id: &str) -> Result<Option<ContainerInfo>> {
        let containers = self.list_containers().await?;
        
        Ok(containers
            .into_iter()
            .find(|c| c.id.starts_with(id) || c.name == id))
    }

    /// Update a container to a new image tag
    /// 
    /// This will:
    /// 1. Pull the new image
    /// 2. Stop the existing container
    /// 3. Remove the old container
    /// 4. Create a new container with the same config but new image
    /// 5. Start the new container
    pub async fn update_container(&self, container_name: &str, target_tag: Option<&str>, credentials: Option<&RegistryCredential>) -> Result<UpdateResult> {
        // Get the container info first
        let container = self.get_container(container_name).await?
            .ok_or_else(|| anyhow!("Container '{}' not found", container_name))?;
        
        let old_image = container.image.clone();
        
        // Determine the new image
        let new_image = if let Some(tag) = target_tag {
            // Replace the tag in the image reference
            let base_image = old_image.split(':').next().unwrap_or(&old_image);
            format!("{}:{}", base_image, tag)
        } else {
            // Just re-pull the same image (for same-tag updates)
            old_image.clone()
        };
        
        tracing::info!("🔄 Updating container '{}': {} -> {}", container_name, old_image, new_image);
        
        // Get full container ID
        let full_id = self.get_full_container_id(container_name).await?;
        
        // Get the container's current config for recreation
        let inspect = self.docker.inspect_container(&full_id, None).await?;
        let was_running = inspect.state.as_ref().and_then(|s| s.running).unwrap_or(false);
        
        // Pull the new image
        tracing::info!("📥 Pulling image: {}", new_image);
        let pull_options = CreateImageOptions {
            from_image: new_image.clone(),
            ..Default::default()
        };
        
        // Build Docker credentials if we have registry credentials
        let docker_creds = credentials.map(|cred| DockerCredentials {
            username: Some(cred.username.clone()),
            password: Some(cred.password.clone()),
            serveraddress: Some(cred.host.clone()),
            ..Default::default()
        });
        
        let mut pull_stream = self.docker.create_image(Some(pull_options), None, docker_creds);
        while let Some(result) = pull_stream.next().await {
            match result {
                Ok(info) => {
                    if let Some(status) = info.status {
                        tracing::debug!("Pull: {}", status);
                    }
                }
                Err(e) => {
                    return Ok(UpdateResult {
                        success: false,
                        message: format!("Failed to pull image: {}", e),
                        old_image,
                        new_image,
                    });
                }
            }
        }
        tracing::info!("✅ Image pulled successfully");
        
        // Stop the container if running
        if was_running {
            tracing::info!("⏹️ Stopping container...");
            self.docker.stop_container(&full_id, Some(StopContainerOptions { t: 30 })).await?;
        }
        
        // Get the container config for recreation
        let config = inspect.config.ok_or_else(|| anyhow!("No config found for container"))?;
        let host_config = inspect.host_config;
        let network_settings = inspect.network_settings;
        
        // Remove the old container
        tracing::info!("🗑️ Removing old container...");
        self.docker.remove_container(&full_id, Some(RemoveContainerOptions {
            force: true,
            ..Default::default()
        })).await?;
        
        // Create new container with updated image
        tracing::info!("🆕 Creating new container with image: {}", new_image);
        let create_options = CreateContainerOptions {
            name: container_name,
            platform: None,
        };
        
        let new_config = Config {
            image: Some(new_image.clone()),
            hostname: config.hostname,
            domainname: config.domainname,
            user: config.user,
            attach_stdin: config.attach_stdin,
            attach_stdout: config.attach_stdout,
            attach_stderr: config.attach_stderr,
            tty: config.tty,
            open_stdin: config.open_stdin,
            stdin_once: config.stdin_once,
            env: config.env,
            cmd: config.cmd,
            healthcheck: config.healthcheck,
            args_escaped: config.args_escaped,
            volumes: config.volumes,
            working_dir: config.working_dir,
            entrypoint: config.entrypoint,
            network_disabled: config.network_disabled,
            mac_address: config.mac_address,
            on_build: config.on_build,
            labels: config.labels,
            stop_signal: config.stop_signal,
            stop_timeout: config.stop_timeout,
            shell: config.shell,
            host_config,
            networking_config: network_settings.and_then(|ns| {
                ns.networks.map(|networks| {
                    bollard::container::NetworkingConfig {
                        endpoints_config: networks,
                    }
                })
            }),
            exposed_ports: config.exposed_ports,
        };
        
        self.docker.create_container(Some(create_options), new_config).await?;
        
        // Start the new container if it was running before
        if was_running {
            tracing::info!("▶️ Starting new container...");
            self.docker.start_container(container_name, None::<StartContainerOptions<String>>).await?;
        }
        
        tracing::info!("✅ Container '{}' updated successfully", container_name);
        
        Ok(UpdateResult {
            success: true,
            message: format!("Container updated from {} to {}", old_image, new_image),
            old_image,
            new_image,
        })
    }
    
    /// Get the full container ID from a name or short ID
    async fn get_full_container_id(&self, name_or_id: &str) -> Result<String> {
        let options = ListContainersOptions::<String> {
            all: true,
            ..Default::default()
        };
        
        let containers = self.docker.list_containers(Some(options)).await?;
        
        for c in containers {
            let id = c.id.unwrap_or_default();
            let names = c.names.unwrap_or_default();
            
            // Check if name matches (with or without leading /)
            let name_match = names.iter().any(|n| {
                let clean_name = n.trim_start_matches('/');
                clean_name == name_or_id
            });
            
            // Check if ID matches (prefix match)
            let id_match = id.starts_with(name_or_id);
            
            if name_match || id_match {
                return Ok(id);
            }
        }
        
        Err(anyhow!("Container '{}' not found", name_or_id))
    }
}
