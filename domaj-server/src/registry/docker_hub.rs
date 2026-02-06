//! Docker Hub registry client
//!
//! Handles authentication and API calls to Docker Hub.

use anyhow::{Context, Result};
use serde::Deserialize;

use super::RegistryClientDyn;

/// Docker Hub API client
pub struct DockerHubClient {
    client: reqwest::Client,
}

impl DockerHubClient {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }

    /// Get an anonymous authentication token for Docker Hub
    async fn get_token(&self, repository: &str) -> Result<String> {
        #[derive(Deserialize)]
        struct TokenResponse {
            token: String,
        }

        let url = format!(
            "https://auth.docker.io/token?service=registry.docker.io&scope=repository:{}:pull",
            repository
        );

        let resp: TokenResponse = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to get Docker Hub token")?
            .json()
            .await
            .context("Failed to parse token response")?;

        Ok(resp.token)
    }
}

impl Default for DockerHubClient {
    fn default() -> Self {
        Self::new()
    }
}

impl RegistryClientDyn for DockerHubClient {
    fn get_digest_dyn(
        &self,
        repository: &str,
        tag: &str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<String>> + Send + '_>> {
        let repo = repository.to_string();
        let tag = tag.to_string();
        
        Box::pin(async move {
            let token = self.get_token(&repo).await?;
            
            let url = format!(
                "https://registry-1.docker.io/v2/{}/manifests/{}",
                repo, tag
            );
            
            // Request manifest list (multi-arch) or fallback to single manifest
            let resp = self
                .client
                .get(&url)
                .header("Authorization", format!("Bearer {}", token))
                .header(
                    "Accept",
                    "application/vnd.oci.image.index.v1+json, application/vnd.docker.distribution.manifest.list.v2+json, application/vnd.docker.distribution.manifest.v2+json",
                )
                .send()
                .await
                .context("Failed to get manifest")?;

            if !resp.status().is_success() {
                let status = resp.status();
                let body = resp.text().await.unwrap_or_default();
                anyhow::bail!("Docker Hub returned {}: {}", status, body);
            }

            let content_type = resp.headers()
                .get("content-type")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("");
            
            // Check if this is a manifest list (multi-arch)
            if content_type.contains("manifest.list") || content_type.contains("image.index") {
                // Parse the manifest list and find the amd64/linux digest
                let body: serde_json::Value = resp.json().await?;
                
                if let Some(manifests) = body["manifests"].as_array() {
                    // Try to find linux/amd64 manifest (most common)
                    for manifest in manifests {
                        let platform = &manifest["platform"];
                        let os = platform["os"].as_str().unwrap_or("");
                        let arch = platform["architecture"].as_str().unwrap_or("");
                        
                        if os == "linux" && arch == "amd64" {
                            if let Some(digest) = manifest["digest"].as_str() {
                                return Ok(digest.to_string());
                            }
                        }
                    }
                    
                    // Fallback: return first manifest digest
                    if let Some(first) = manifests.first() {
                        if let Some(digest) = first["digest"].as_str() {
                            return Ok(digest.to_string());
                        }
                    }
                }
                
                anyhow::bail!("No suitable manifest found in manifest list");
            } else {
                // Single architecture manifest - use Docker-Content-Digest header
                let digest = resp
                    .headers()
                    .get("docker-content-digest")
                    .and_then(|v| v.to_str().ok())
                    .map(|s| s.to_string())
                    .ok_or_else(|| anyhow::anyhow!("Docker-Content-Digest header not found"))?;

                Ok(digest)
            }
        })
    }

    fn list_tags_dyn(
        &self,
        repository: &str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Vec<String>>> + Send + '_>> {
        let repo = repository.to_string();
        
        Box::pin(async move {
            let token = self.get_token(&repo).await?;
            
            let url = format!(
                "https://registry-1.docker.io/v2/{}/tags/list",
                repo
            );
            
            #[derive(Deserialize)]
            struct TagsResponse {
                tags: Option<Vec<String>>,
            }
            
            let resp: TagsResponse = self
                .client
                .get(&url)
                .header("Authorization", format!("Bearer {}", token))
                .send()
                .await
                .context("Failed to list tags")?
                .json()
                .await
                .context("Failed to parse tags response")?;

            Ok(resp.tags.unwrap_or_default())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_parse_official_image() {
        use crate::registry::ImageReference;
        
        let img = ImageReference::parse("nginx");
        assert_eq!(img.registry, "docker.io");
        assert_eq!(img.repository, "library/nginx");
        assert_eq!(img.tag, "latest");
    }

    #[tokio::test]
    async fn test_parse_tagged_image() {
        use crate::registry::ImageReference;
        
        let img = ImageReference::parse("nginx:1.25");
        assert_eq!(img.registry, "docker.io");
        assert_eq!(img.repository, "library/nginx");
        assert_eq!(img.tag, "1.25");
    }

    #[tokio::test]
    async fn test_parse_user_image() {
        use crate::registry::ImageReference;
        
        let img = ImageReference::parse("myuser/myapp:v1");
        assert_eq!(img.registry, "docker.io");
        assert_eq!(img.repository, "myuser/myapp");
        assert_eq!(img.tag, "v1");
    }

    #[tokio::test]
    async fn test_parse_custom_registry() {
        use crate::registry::ImageReference;
        
        let img = ImageReference::parse("ghcr.io/owner/repo:latest");
        assert_eq!(img.registry, "ghcr.io");
        assert_eq!(img.repository, "owner/repo");
        assert_eq!(img.tag, "latest");
    }
}
