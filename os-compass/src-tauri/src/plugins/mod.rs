pub mod github;
pub mod gitee;
pub mod gitlab;
pub mod npm;
pub mod pypi;
pub mod crates;
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
    #[serde(default)]
    pub health_score: Option<f64>,
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
            health_score: None,
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
    GitLab,
    NPM,
    PyPI,
    Crates,
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
                    } else if domain.contains("gitlab") {
                        return Platform::GitLab;
                    } else if domain.contains("npm") {
                        return Platform::NPM;
                    } else if domain.contains("pypi") {
                        return Platform::PyPI;
                    } else if domain.contains("crates") {
                        return Platform::Crates;
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
            } else if url_lower.contains("gitlab.com") {
                Platform::GitLab
            } else if url_lower.contains("npmjs.com") || url_lower.contains("registry.npmjs") {
                Platform::NPM
            } else if url_lower.contains("pypi.org") {
                Platform::PyPI
            } else if url_lower.contains("crates.io") {
                Platform::Crates
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
        Platform::GitLab => {
            // 支持多级 group：gitlab.com/group/sub/repo，截掉 /-/ 之后的路径段
            let path_part = url.split(['?', '#']).next().unwrap_or("").trim_end_matches('/');
            let parts: Vec<&str> = path_part.split('/').collect();
            if parts.len() >= 5 && parts[2].contains("gitlab") {
                let joined = parts[3..].join("/");
                let cut = joined.split("/-/").next().unwrap_or("").trim_end_matches('/');
                if !cut.is_empty() {
                    return Some(cut.to_string());
                }
            }
            None
        }
        Platform::NPM => {
            let url_lower = url.to_lowercase();
            if url_lower.contains("npmjs.com") || url_lower.contains("registry.npmjs") {
                if let Some(idx) = url_lower.rfind("package/") {
                    // 保留 scoped 包完整名（@scope/name），剥查询串与尾斜杠
                    let name = url[idx + 8..].split(['?', '#']).next().unwrap_or("").trim_end_matches('/');
                    if !name.is_empty() {
                        return Some(name.to_string());
                    }
                }
                // registry 直链：https://registry.npmjs.org/<name>
                if let Some(idx) = url_lower.find("registry.npmjs.org/") {
                    let name = url[idx + 19..].split(['?', '#']).next().unwrap_or("").trim_end_matches('/');
                    if !name.is_empty() {
                        return Some(name.to_string());
                    }
                }
            }
            // 裸包名（含 @scope/name）：不含 "://" 即视为包名
            if !url.contains("://") {
                let name = url.trim().trim_matches('/');
                if !name.is_empty() {
                    return Some(name.to_string());
                }
            }
            None
        }
        Platform::PyPI => {
            let url_lower = url.to_lowercase();
            if url_lower.contains("pypi.org") {
                if let Some(idx) = url_lower.rfind("project/") {
                    let name = &url[idx + 8..];
                    let name = name.trim_end_matches('/');
                    if !name.is_empty() {
                        return Some(name.to_string());
                    }
                }
            }
            if !url.contains('/') {
                return Some(url.trim().to_string());
            }
            None
        }
        Platform::Crates => {
            let url_lower = url.to_lowercase();
            if url_lower.contains("crates.io") {
                if let Some(idx) = url_lower.rfind("crates/") {
                    let name = &url[idx + 7..];
                    let name = name.trim_end_matches('/');
                    if !name.is_empty() {
                        return Some(name.to_string());
                    }
                }
            }
            if !url.contains('/') {
                return Some(url.trim().to_string());
            }
            None
        }
        Platform::Unknown => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn github_url() {
        assert_eq!(
            extract_source_id("https://github.com/tauri-apps/tauri", Platform::GitHub),
            Some("tauri-apps/tauri".to_string())
        );
        assert_eq!(
            extract_source_id("https://github.com/tauri-apps/tauri/", Platform::GitHub),
            Some("tauri-apps/tauri".to_string())
        );
    }

    #[test]
    fn gitee_url() {
        assert_eq!(
            extract_source_id("https://gitee.com/oschina/git-osc", Platform::Gitee),
            Some("oschina/git-osc".to_string())
        );
    }

    #[test]
    fn npm_plain_package() {
        assert_eq!(
            extract_source_id("https://www.npmjs.com/package/left-pad", Platform::NPM),
            Some("left-pad".to_string())
        );
    }

    #[test]
    fn npm_scoped_package() {
        assert_eq!(
            extract_source_id("https://www.npmjs.com/package/@types/node", Platform::NPM),
            Some("@types/node".to_string())
        );
        // 裸 scoped 包名
        assert_eq!(
            extract_source_id("@types/node", Platform::NPM),
            Some("@types/node".to_string())
        );
        // 带查询串
        assert_eq!(
            extract_source_id("https://www.npmjs.com/package/@types/node?activeTab=readme", Platform::NPM),
            Some("@types/node".to_string())
        );
    }

    #[test]
    fn npm_registry_direct_link() {
        assert_eq!(
            extract_source_id("https://registry.npmjs.org/react", Platform::NPM),
            Some("react".to_string())
        );
    }

    #[test]
    fn npm_bare_name() {
        assert_eq!(
            extract_source_id("lodash", Platform::NPM),
            Some("lodash".to_string())
        );
    }

    #[test]
    fn pypi_url_and_bare_name() {
        assert_eq!(
            extract_source_id("https://pypi.org/project/requests/", Platform::PyPI),
            Some("requests".to_string())
        );
        assert_eq!(
            extract_source_id("requests", Platform::PyPI),
            Some("requests".to_string())
        );
    }

    #[test]
    fn crates_url_and_bare_name() {
        assert_eq!(
            extract_source_id("https://crates.io/crates/serde", Platform::Crates),
            Some("serde".to_string())
        );
        assert_eq!(
            extract_source_id("serde", Platform::Crates),
            Some("serde".to_string())
        );
    }

    #[test]
    fn invalid_input_returns_none() {
        assert_eq!(extract_source_id("", Platform::NPM), None);
        assert_eq!(extract_source_id("https://example.com/foo", Platform::GitHub), None);
        assert_eq!(extract_source_id("anything", Platform::Unknown), None);
    }
}
