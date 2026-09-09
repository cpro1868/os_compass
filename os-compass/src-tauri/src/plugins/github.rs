use crate::plugins::{extract_source_id, ImportResult, Platform, ProjectData};
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
struct GitHubRepo {
    name: String,
    #[allow(dead_code)]
    full_name: String,
    description: Option<String>,
    #[serde(default)]
    language: Option<String>,
    stargazers_count: i64,
    forks_count: i64,
    open_issues_count: i64,
    license: Option<GitHubLicense>,
    homepage: Option<String>,
    pushed_at: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GitHubLicense {
    name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GitHubContent {
    name: String,
    content: Option<String>,
    #[allow(dead_code)]
    encoding: Option<String>,
}

fn build_client(proxy: Option<&str>) -> reqwest::Client {
    let mut builder = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30));
    if let Some(p) = proxy {
        if !p.trim().is_empty() {
            if let Ok(proxy) = reqwest::Proxy::all(p.trim()) {
                builder = builder.proxy(proxy);
            }
        }
    }
    builder.build().unwrap_or_else(|_| reqwest::Client::new())
}

pub async fn fetch_github_project(url: &str, token: Option<&str>, proxy: Option<&str>) -> ImportResult {
    let platform = Platform::GitHub;
    let source_id = match extract_source_id(url, platform) {
        Some(id) => id,
        None => return ImportResult {
            success: false,
            project_id: None,
            error: Some("Invalid GitHub URL".to_string()),
            project_data: None,
            readme_variants: None,
        },
    };

    let repo_url = format!("https://api.github.com/repos/{}", source_id);
    let client = build_client(proxy);
    
    let mut request = client.get(&repo_url)
        .header("User-Agent", "OS-Compass")
        .header("Accept", "application/vnd.github.v3+json");
    
    if let Some(t) = token {
        request = request.header("Authorization", format!("Bearer {}", t));
    }

    let repo: GitHubRepo = match request.send().await {
        Ok(resp) => {
            if resp.status() == 200 {
                match resp.json().await {
                    Ok(r) => r,
                    Err(e) => return ImportResult {
                        success: false,
                        project_id: None,
                        error: Some(format!("Failed to parse GitHub response: {}", e)),
                        project_data: None,
                        readme_variants: None,
                    },
                }
            } else {
                return ImportResult {
                    success: false,
                    project_id: None,
                    error: Some(format!("GitHub API error: {}", resp.status())),
                    project_data: None,
                    readme_variants: None,
                };
            }
        }
        Err(e) => return ImportResult {
            success: false,
            project_id: None,
            error: Some(format!("Network error: {}", e)),
            project_data: None,
            readme_variants: None,
        },
    };

    let readme_content = fetch_readme(&source_id, &client, token).await.ok();
    let readme_variants = fetch_readme_variants(&source_id, &client, token).await.ok();

    let languages = fetch_languages(&source_id, &client, token).await;

    let license = repo.license.as_ref().and_then(|l| l.name.clone());

    let project_data = ProjectData {
        name: repo.name,
        url: format!("https://github.com/{}", source_id),
        source: "github".to_string(),
        source_id,
        description: repo.description,
        languages,
        stars: repo.stargazers_count as i32,
        forks: repo.forks_count as i32,
        open_issues: repo.open_issues_count as i32,
        license,
        homepage: repo.homepage,
        latest_commit: repo.pushed_at,
        latest_release: None,
        readme_content,
    };

    ImportResult {
        success: true,
        project_id: None,
        error: None,
        project_data: Some(project_data),
        readme_variants,
    }
}

async fn fetch_readme(
    repo: &str,
    client: &reqwest::Client,
    token: Option<&str>,
) -> Result<String, String> {
    let readme_url = format!("https://api.github.com/repos/{}/readme", repo);
    
    let mut request = client.get(&readme_url)
        .header("User-Agent", "OS-Compass")
        .header("Accept", "application/vnd.github.v3.raw");
    
    if let Some(t) = token {
        request = request.header("Authorization", format!("Bearer {}", t));
    }

    let content = request.send()
        .await
        .map_err(|e| e.to_string())?
        .text()
        .await
        .map_err(|e| e.to_string())?;

    Ok(content)
}

pub async fn fetch_readme_variants(
    repo: &str,
    client: &reqwest::Client,
    token: Option<&str>,
) -> Result<HashMap<String, String>, String> {
    let contents_url = format!("https://api.github.com/repos/{}/contents", repo);
    
    let mut request = client.get(&contents_url)
        .header("User-Agent", "OS-Compass")
        .header("Accept", "application/vnd.github.v3+json");
    
    if let Some(t) = token {
        request = request.header("Authorization", format!("Bearer {}", t));
    }

    let contents: Vec<GitHubContent> = request.send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;

    let mut variants = HashMap::new();
    
    for item in contents {
        let name_lower = item.name.to_lowercase();
        if name_lower.starts_with("readme") && name_lower.ends_with(".md") {
            let file_url = format!("https://api.github.com/repos/{}/contents/{}", repo, item.name);
            
            let mut file_request = client.get(&file_url)
                .header("User-Agent", "OS-Compass")
                .header("Accept", "application/vnd.github.v3.raw");
            
            if let Some(t) = token {
                file_request = file_request.header("Authorization", format!("Bearer {}", t));
            }

            if let Ok(response) = file_request.send().await {
                if let Ok(text) = response.text().await {
                    variants.insert(item.name.clone(), text);
                }
            }
        }
    }

    Ok(variants)
}

async fn fetch_languages(
    repo: &str,
    client: &reqwest::Client,
    token: Option<&str>,
) -> Option<String> {
    let lang_url = format!("https://api.github.com/repos/{}/languages", repo);

    let mut request = client.get(&lang_url)
        .header("User-Agent", "OS-Compass")
        .header("Accept", "application/vnd.github.v3+json");

    if let Some(t) = token {
        request = request.header("Authorization", format!("Bearer {}", t));
    }

    let response = request.send().await.ok()?;
    if !response.status().is_success() {
        return None;
    }

    let lang_map: HashMap<String, u64> = response.json().await.ok()?;
    if lang_map.is_empty() {
        return None;
    }

    let mut langs: Vec<(String, u64)> = lang_map.into_iter().collect();
    langs.sort_by(|a, b| b.1.cmp(&a.1));

    let lang_names: Vec<String> = langs.iter().map(|(name, _)| name.clone()).take(10).collect();
    serde_json::to_string(&lang_names).ok()
}
