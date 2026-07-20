use crate::source_engine::{build_client, RawContent, SourceAdapter, SourceError, SourceType};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitLabSource {
    pub base_url: String,
    pub token: Option<String>,
}

impl GitLabSource {
    pub fn new() -> Self {
        GitLabSource {
            base_url: "https://gitlab.com".to_string(),
            token: None,
        }
    }

    pub fn with_token(token: &str) -> Self {
        GitLabSource {
            base_url: "https://gitlab.com".to_string(),
            token: Some(token.to_string()),
        }
    }

    pub fn with_base_url(base_url: &str) -> Self {
        GitLabSource {
            base_url: base_url.to_string(),
            token: None,
        }
    }

    pub fn with_base_url_and_token(base_url: &str, token: &str) -> Self {
        GitLabSource {
            base_url: base_url.to_string(),
            token: Some(token.to_string()),
        }
    }
}

impl Default for GitLabSource {
    fn default() -> Self {
        Self::new()
    }
}

pub struct GitLabAdapter {
    source: GitLabSource,
}

impl GitLabAdapter {
    pub fn new() -> Self {
        GitLabAdapter {
            source: GitLabSource::new(),
        }
    }

    pub fn with_source(source: GitLabSource) -> Self {
        GitLabAdapter { source }
    }

    pub fn matches_url(url: &str) -> bool {
        url.contains("gitlab.com") || url.contains("/gitlab/")
    }

    fn parse_gitlab_url(url: &str) -> Option<String> {
        let url = url.trim_end_matches('/');

        let patterns = [
            "https://gitlab.com/",
            "http://gitlab.com/",
            "https://www.gitlab.com/",
            "http://www.gitlab.com/",
        ];

        for pattern in &patterns {
            if url.starts_with(pattern) {
                let path = &url[pattern.len()..];
                if !path.is_empty() && !path.starts_with("users") && !path.starts_with("explore") {
                    return Some(format!("gitlab.com/{}", path));
                }
            }
        }

        if url.contains("gitlab.com") {
            let parts: Vec<&str> = url.split("gitlab.com").collect();
            if parts.len() > 1 {
                let path = parts[1].trim_start_matches('/');
                if !path.is_empty() && !path.starts_with("users") && !path.starts_with("explore") {
                    return Some(format!("gitlab.com/{}", path));
                }
            }
        }

        if url.contains("/gitlab/") {
            let parts: Vec<&str> = url.split("/gitlab/").collect();
            if parts.len() > 1 {
                return Some(format!("gitlab.com/{}", parts[1].trim_end_matches('/')));
            }
        }

        None
    }

    fn build_api_url(&self, project_path: &str) -> String {
        let encoded = project_path.replace('/', "%2F");
        format!("{}/api/v4/projects/{}", self.source.base_url, encoded)
    }

    async fn fetch_project_info(&self, project_path: &str) -> Result<serde_json::Value, SourceError> {
        let client = build_client(None)?;
        let url = self.build_api_url(project_path);

        let mut request = client.get(&url);

        if let Some(ref token) = self.source.token {
            request = request.header("PRIVATE-TOKEN", token);
        }

        let response = request.send().await?;
        let status = response.status();

        if !status.is_success() {
            if status.as_u16() == 404 {
                return Err(SourceError::ParseError(format!("Project not found: {}", project_path)));
            }
            return Err(SourceError::NetworkError(format!("GitLab API error: {}", status)));
        }

        let data: serde_json::Value = response.json().await?;
        Ok(data)
    }

    async fn fetch_contributors(&self, project_path: &str) -> Result<Vec<serde_json::Value>, SourceError> {
        let client = build_client(None)?;
        let encoded = project_path.replace('/', "%2F");
        let url = format!("{}/api/v4/projects/{}/repository/contributors", self.source.base_url, encoded);

        let mut request = client.get(&url);

        if let Some(ref token) = self.source.token {
            request = request.header("PRIVATE-TOKEN", token);
        }

        let response = request.send().await?;

        if !response.status().is_success() {
            return Ok(vec![]);
        }

        let data: Vec<serde_json::Value> = response.json().await?;
        Ok(data)
    }

    async fn fetch_merge_requests(&self, project_path: &str) -> Result<(u64, u64), SourceError> {
        let client = build_client(None)?;
        let encoded = project_path.replace('/', "%2F");
        let url = format!(
            "{}/api/v4/projects/{}/merge_requests?state=all&per_page=100",
            self.source.base_url, encoded
        );

        let mut request = client.get(&url);

        if let Some(ref token) = self.source.token {
            request = request.header("PRIVATE-TOKEN", token);
        }

        let response = request.send().await?;

        if !response.status().is_success() {
            return Ok((0, 0));
        }

        let data: Vec<serde_json::Value> = response.json().await?;

        let total_mr = data.len() as u64;
        let merged_mr = data.iter().filter(|mr| {
            mr.get("state").and_then(|s| s.as_str()) == Some("merged")
        }).count() as u64;

        Ok((merged_mr, total_mr))
    }

    fn calculate_health_score(
        &self,
        project: &serde_json::Value,
        contributors: &[serde_json::Value],
        (merged_mr, total_mr): (u64, u64),
    ) -> u32 {
        let mut score: u32 = 0;

        let stars = project.get("star_count").and_then(|v| v.as_u64()).unwrap_or(0);
        if stars >= 10000 {
            score += 10;
        } else if stars >= 1000 {
            score += 8;
        } else if stars >= 100 {
            score += 5;
        } else {
            score += 2;
        }

        if total_mr > 0 {
            let merge_rate = (merged_mr as f64) / (total_mr as f64);
            if merge_rate >= 0.8 {
                score += 20;
            } else if merge_rate >= 0.6 {
                score += 15;
            } else if merge_rate >= 0.4 {
                score += 10;
            } else {
                score += 5;
            }
        } else {
            score += 10;
        }

        if !contributors.is_empty() {
            if contributors.len() >= 20 {
                score += 10;
            } else if contributors.len() >= 5 {
                score += 8;
            } else if contributors.len() >= 2 {
                score += 5;
            } else {
                score += 2;
            }
        }

        if let Some(last_activity) = project.get("last_activity_at").and_then(|v| v.as_str()) {
            if let Ok(activity) = chrono::DateTime::parse_from_rfc3339(last_activity) {
                let days_since = chrono::Utc::now().signed_duration_since(activity.with_timezone(&chrono::Utc)).num_days();
                if days_since <= 30 {
                    score += 25;
                } else if days_since <= 90 {
                    score += 20;
                } else if days_since <= 180 {
                    score += 15;
                } else if days_since <= 365 {
                    score += 8;
                } else {
                    score += 0;
                }
            }
        }

        if project.get("visibility").and_then(|v| v.as_str()) == Some("public") {
            score += 5;
        }

        score.min(100)
    }
}

impl Default for GitLabAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SourceAdapter for GitLabAdapter {
    fn adapter_type(&self) -> SourceType {
        SourceType::GitLab
    }

    async fn fetch(
        &mut self,
        url: &str,
        proxy: Option<&str>,
        _time_range: Option<&str>,
    ) -> Result<Vec<RawContent>, SourceError> {
        let project_path = Self::parse_gitlab_url(url)
            .ok_or_else(|| SourceError::ParseError(format!("Invalid GitLab URL: {}", url)))?;

        let project = self.fetch_project_info(&project_path).await?;

        let project_name = project
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown")
            .to_string();

        let project_description = project
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let web_url = project
            .get("web_url")
            .and_then(|v| v.as_str())
            .unwrap_or(url)
            .to_string();

        let language = project
            .get("primary_language")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let contributors = self.fetch_contributors(&project_path).await?;
        let mr_stats = self.fetch_merge_requests(&project_path).await?;
        let health_score = self.calculate_health_score(&project, &contributors, mr_stats);

        let topics = project
            .get("topics")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            })
            .unwrap_or_default();

        let content = format!(
            "Name: {}\nDescription: {}\nLanguage: {}\nTopics: {}\nHealth Score: {}/100\nStars: {}\nForks: {}\n",
            project_name,
            project_description,
            language.as_deref().unwrap_or("Unknown"),
            if topics.is_empty() { "None".to_string() } else { topics },
            health_score,
            project.get("star_count").and_then(|v| v.as_u64()).unwrap_or(0),
            project.get("forks_count").and_then(|v| v.as_u64()).unwrap_or(0),
        );

        let published_at = project
            .get("created_at")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        Ok(vec![RawContent {
            title: project_name,
            url: web_url,
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
        assert!(GitLabAdapter::matches_url("https://gitlab.com/owner/repo"));
        assert!(GitLabAdapter::matches_url("https://gitlab.example.com/owner/repo"));
        assert!(!GitLabAdapter::matches_url("https://github.com/owner/repo"));
    }

    #[test]
    fn test_parse_gitlab_url() {
        assert_eq!(
            GitLabAdapter::parse_gitlab_url("https://gitlab.com/owner/repo"),
            Some("gitlab.com/owner/repo".to_string())
        );
        assert_eq!(
            GitLabAdapter::parse_gitlab_url("https://gitlab.com/owner/repo/"),
            Some("gitlab.com/owner/repo".to_string())
        );
    }
}
