use crate::source_engine::{build_client, RawContent, SourceAdapter, SourceError, SourceType};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockerHubSource {
    pub namespace: String,
    pub repository: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockerHubStats {
    pub pull_count: u64,
    pub star_count: u64,
    pub pull_count_7d: Option<u64>,
    pub pull_count_30d: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockerHubTag {
    pub name: String,
    pub full_size: u64,
    pub tag_last_pushed: Option<String>,
}

pub struct DockerHubAdapter;

impl DockerHubAdapter {
    pub fn new() -> Self {
        DockerHubAdapter
    }

    pub fn matches_url(url: &str) -> bool {
        url.contains("docker.io") || url.contains("hub.docker.com") || url.contains("/docker/")
    }

    pub fn parse_dockerhub_url(url: &str) -> Option<(String, String)> {
        let url = url.trim_end_matches('/');

        if url.contains("docker.io/library/") {
            let parts: Vec<&str> = url.split("docker.io/library/").collect();
            if parts.len() > 1 {
                return Some(("library".to_string(), parts[1].to_string()));
            }
        }

        if url.contains("hub.docker.com/u/") {
            let parts: Vec<&str> = url.split("hub.docker.com/u/").collect();
            if parts.len() > 1 {
                let path = parts[1].trim_end_matches('/');
                let parts: Vec<&str> = path.split('/').collect();
                if parts.len() >= 2 {
                    return Some((parts[0].to_string(), parts[1].to_string()));
                }
            }
        }

        if url.contains("hub.docker.com/r/") {
            let parts: Vec<&str> = url.split("hub.docker.com/r/").collect();
            if parts.len() > 1 {
                let path = parts[1].trim_end_matches('/');
                let parts: Vec<&str> = path.split('/').collect();
                if parts.len() >= 2 {
                    return Some((parts[0].to_string(), parts[1].to_string()));
                }
            }
        }

        if url.contains("/docker.io/") {
            let parts: Vec<&str> = url.split("/docker.io/").collect();
            if parts.len() > 1 {
                return Some(Self::parse_path(parts[1]));
            }
        }

        if url.contains("/docker/") {
            let parts: Vec<&str> = url.split("/docker/").collect();
            if parts.len() > 1 {
                return Some(Self::parse_path(parts[1]));
            }
        }

        None
    }

    fn parse_path(path: &str) -> (String, String) {
        let parts: Vec<&str> = path.split('/').collect();
        if parts.len() >= 2 {
            (parts[0].to_string(), parts[1].to_string())
        } else if parts.len() == 1 && !parts[0].is_empty() {
            ("library".to_string(), parts[0].to_string())
        } else {
            ("library".to_string(), "unknown".to_string())
        }
    }

    pub fn build_api_url(namespace: &str, repository: &str) -> String {
        let repo = format!("{}/{}", namespace, repository);
        format!("https://hub.docker.com/v2/repositories/{}", repo)
    }

    pub async fn fetch_stats(&self, namespace: &str, repository: &str) -> Result<DockerHubStats, SourceError> {
        let client = build_client(None)?;
        let url = DockerHubAdapter::build_api_url(namespace, repository);

        let response = client.get(&url).send().await?;
        let status = response.status();

        if !status.is_success() {
            if status.as_u16() == 404 {
                return Err(SourceError::ParseError(format!(
                    "Docker image not found: {}/{}",
                    namespace, repository
                )));
            }
            return Err(SourceError::NetworkError(format!("Docker Hub API error: {}", status)));
        }

        let data: serde_json::Value = response.json().await?;

        let pull_count = data
            .get("pull_count")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        let star_count = data
            .get("star_count")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        Ok(DockerHubStats {
            pull_count,
            star_count,
            pull_count_7d: None,
            pull_count_30d: None,
        })
    }

    async fn fetch_tags(&self, namespace: &str, repository: &str) -> Result<Vec<DockerHubTag>, SourceError> {
        let client = build_client(None)?;
        let repo = format!("{}/{}", namespace, repository);
        let url = format!("https://hub.docker.com/v2/repositories/{}/tags", repo);

        let response = client.get(&url).send().await?;

        if !response.status().is_success() {
            return Ok(vec![]);
        }

        let data: serde_json::Value = response.json().await?;

        let results = data.get("results").and_then(|v| v.as_array());

        match results {
            Some(arr) => {
                let tags: Vec<DockerHubTag> = arr
                    .iter()
                    .take(10)
                    .filter_map(|tag| {
                        let name = tag.get("name")?.as_str()?.to_string();
                        let full_size = tag.get("full_size")?.as_u64().unwrap_or(0);
                        let tag_last_pushed = tag
                            .get("tag_last_pushed")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string());

                        Some(DockerHubTag {
                            name,
                            full_size,
                            tag_last_pushed,
                        })
                    })
                    .collect();
                Ok(tags)
            }
            None => Ok(vec![]),
        }
    }

    fn calculate_health_score(stats: &DockerHubStats) -> u32 {
        let mut score: u32 = 0;

        if stats.pull_count >= 100_000_000 {
            score += 25;
        } else if stats.pull_count >= 10_000_000 {
            score += 20;
        } else if stats.pull_count >= 1_000_000 {
            score += 15;
        } else if stats.pull_count >= 100_000 {
            score += 10;
        } else if stats.pull_count >= 10_000 {
            score += 5;
        } else {
            score += 2;
        }

        if stats.star_count >= 10000 {
            score += 20;
        } else if stats.star_count >= 1000 {
            score += 15;
        } else if stats.star_count >= 100 {
            score += 10;
        } else if stats.star_count >= 10 {
            score += 5;
        } else {
            score += 2;
        }

        if stats.pull_count_7d.is_some() || stats.pull_count_30d.is_some() {
            score += 10;
        }

        score.min(100)
    }
}

impl Default for DockerHubAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SourceAdapter for DockerHubAdapter {
    fn adapter_type(&self) -> SourceType {
        SourceType::DockerHub
    }

    async fn fetch(
        &mut self,
        url: &str,
        _proxy: Option<&str>,
        _time_range: Option<&str>,
    ) -> Result<Vec<RawContent>, SourceError> {
        let (namespace, repository) = Self::parse_dockerhub_url(url)
            .ok_or_else(|| SourceError::ParseError(format!("Invalid Docker Hub URL: {}", url)))?;

        let stats = self.fetch_stats(&namespace, &repository).await?;
        let tags = self.fetch_tags(&namespace, &repository).await?;

        let image_name = format!("{}/{}", namespace, repository);
        let health_score = Self::calculate_health_score(&stats);

        let tag_list: Vec<String> = tags
            .iter()
            .take(5)
            .map(|t| {
                let size_mb = t.full_size as f64 / 1024.0 / 1024.0;
                format!("{} ({:.1} MB)", t.name, size_mb)
            })
            .collect();

        let content = format!(
            "Image: {}\nPull Count: {}\nStar Count: {}\nHealth Score: {}/100\nLatest Tags: {}\n",
            image_name,
            stats.pull_count,
            stats.star_count,
            health_score,
            if tag_list.is_empty() {
                "None".to_string()
            } else {
                tag_list.join(", ")
            }
        );

        let published_at = tags.first().and_then(|t| t.tag_last_pushed.clone());

        Ok(vec![RawContent {
            title: image_name,
            url: url.to_string(),
            content: Some(content),
            published_at,
        }])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_matches_url() {
        assert!(DockerHubAdapter::matches_url("https://docker.io/library/nginx"));
        assert!(DockerHubAdapter::matches_url("https://hub.docker.com/r/nginx"));
        assert!(!DockerHubAdapter::matches_url("https://github.com/owner/repo"));
    }

    #[test]
    fn test_parse_dockerhub_url() {
        assert_eq!(
            DockerHubAdapter::parse_dockerhub_url("https://docker.io/library/nginx"),
            Some(("library".to_string(), "nginx".to_string()))
        );
        assert_eq!(
            DockerHubAdapter::parse_dockerhub_url("https://docker.io/library/postgres"),
            Some(("library".to_string(), "postgres".to_string()))
        );
        assert_eq!(
            DockerHubAdapter::parse_dockerhub_url("https://docker.io/namespace/image"),
            Some(("namespace".to_string(), "image".to_string()))
        );
    }
}
