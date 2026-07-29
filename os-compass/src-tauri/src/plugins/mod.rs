pub mod github;
pub mod gitee;
pub mod crawler;
pub mod radar;
pub mod search;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectData {
    pub name: String,
    pub url: String,
    pub source: String,
    pub source_id: String,
    pub description: Option<String>,
    pub languages: Option<String>,
    pub stars: i32,
    pub forks: i32,
    pub open_issues: i32,
    pub license: Option<String>,
    pub homepage: Option<String>,
    pub latest_commit: Option<String>,
    pub latest_release: Option<String>,
    pub readme_content: Option<String>,
}

impl Default for ProjectData {
    fn default() -> Self {
        Self {
            name: String::new(),
            url: String::new(),
            source: String::new(),
            source_id: String::new(),
            description: None,
            languages: None,
            stars: 0,
            forks: 0,
            open_issues: 0,
            license: None,
            homepage: None,
            latest_commit: None,
            latest_release: None,
            readme_content: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportResult {
    pub success: bool,
    pub project_id: Option<i64>,
    pub error: Option<String>,
    pub project_data: Option<ProjectData>,
    #[serde(default)]
    pub readme_variants: Option<std::collections::HashMap<String, String>>,
}

impl Default for ImportResult {
    fn default() -> Self {
        Self {
            success: false,
            project_id: None,
            error: None,
            project_data: None,
            readme_variants: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Platform {
    GitHub,
    Gitee,
    Unknown,
}

pub fn identify_platform(url: &str) -> Platform {
    match crate::system_db::get_enabled_source_domains() {
        Ok(domains) => {
            let url_lower = url.to_lowercase();
            for domain in domains {
                if url_lower.contains(&domain.to_lowercase()) {
                    if domain.contains("github") {
                        return Platform::GitHub;
                    } else if domain.contains("gitee") {
                        return Platform::Gitee;
                    }
                }
            }
            Platform::Unknown
        }
        Err(_) => {
            let url_lower = url.to_lowercase();
            if url_lower.contains("github.com") {
                Platform::GitHub
            } else if url_lower.contains("gitee.com") {
                Platform::Gitee
            } else {
                Platform::Unknown
            }
        }
    }
}

pub fn get_supported_platforms() -> Vec<(String, String)> {
    let mut platforms = Vec::new();
    
    if let Ok(domains) = crate::system_db::get_enabled_source_domains() {
        for domain in domains {
            let name: String = if domain.contains("github") {
                "GitHub".to_string()
            } else if domain.contains("gitee") {
                "Gitee".to_string()
            } else if domain.contains("gitlab") {
                "GitLab".to_string()
            } else if domain.contains("npm") {
                "NPM".to_string()
            } else if domain.contains("pypi") || domain.contains("python") {
                "PyPI".to_string()
            } else if domain.contains("crates") {
                "Crates.io".to_string()
            } else {
                domain.clone()
            };
            platforms.push((domain, name));
        }
    }
    
    if platforms.is_empty() {
        platforms.push(("github.com".to_string(), "GitHub".to_string()));
        platforms.push(("gitee.com".to_string(), "Gitee".to_string()));
    }
    
    platforms
}

pub fn extract_source_id(url: &str, platform: Platform) -> Option<String> {
    match platform {
        Platform::GitHub => {
            let parts: Vec<&str> = url.trim_end_matches('/').split("/").collect();
            if parts.len() >= 5 && parts[2] == "github.com" {
                Some(format!("{}/{}", parts[3], parts[4]))
            } else {
                None
            }
        }
        Platform::Gitee => {
            let parts: Vec<&str> = url.trim_end_matches('/').split("/").collect();
            if parts.len() >= 5 && parts[2] == "gitee.com" {
                Some(format!("{}/{}", parts[3], parts[4]))
            } else {
                None
            }
        }
        Platform::Unknown => None,
    }
}
