use crate::plugins::{extract_source_id, ImportResult, Platform, ProjectData};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct NpmPackage {
    name: String,
    description: Option<String>,
    homepage: Option<String>,
    license: Option<String>,
    versions: serde_json::Value,
    #[serde(rename = "dist-tags")]
    dist_tags: Option<NpmDistTags>,
}

#[derive(Debug, Deserialize)]
struct NpmDistTags {
    latest: Option<String>,
}

#[derive(Debug, Deserialize)]
struct NpmDownloads {
    downloads: i64,
}

#[derive(Debug, Deserialize)]
struct NpmSearchResult {
    package: NpmSearchPackage,
}

#[derive(Debug, Deserialize)]
struct NpmSearchPackage {
    name: String,
    version: Option<String>,
    description: Option<String>,
    links: Option<NpmLinks>,
    score: Option<NpmScore>,
}

#[derive(Debug, Deserialize)]
struct NpmLinks {
    homepage: Option<String>,
    repository: Option<String>,
}

#[derive(Debug, Deserialize)]
struct NpmScore {
    #[serde(rename = "final")]
    final_score: f64,
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

pub async fn fetch_npm_project(url: &str, proxy: Option<&str>) -> ImportResult {
    let source_id = match extract_source_id(url, Platform::NPM) {
        Some(id) => id,
        None => return ImportResult {
            success: false,
            project_id: None,
            error: Some("Invalid NPM package URL or name".to_string()),
            project_data: None,
            readme_variants: None,
        },
    };

    let client = build_client(proxy);
    let package_name = source_id.trim();

    let package_url = format!("https://registry.npmjs.org/{}", urlencoding::encode(package_name));
    
    let package: NpmPackage = match client.get(&package_url).send().await {
        Ok(resp) => {
            if resp.status().is_success() {
                match resp.json().await {
                    Ok(p) => p,
                    Err(e) => return ImportResult {
                        success: false,
                        project_id: None,
                        error: Some(format!("Failed to parse NPM response: {}", e)),
                        project_data: None,
                        readme_variants: None,
                    },
                }
            } else if resp.status().as_u16() == 404 {
                return ImportResult {
                    success: false,
                    project_id: None,
                    error: Some(format!("Package '{}' not found", package_name)),
                    project_data: None,
                    readme_variants: None,
                };
            } else {
                return ImportResult {
                    success: false,
                    project_id: None,
                    error: Some(format!("NPM API error: {}", resp.status())),
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

    let downloads = fetch_npm_downloads(package_name, &client).await;
    let latest_version = package.dist_tags.as_ref()
        .and_then(|t| t.latest.as_ref())
        .cloned()
        .unwrap_or_else(|| "unknown".to_string());

    let homepage = package.homepage.clone();

    let repository_url = extract_repository_from_versions(&package.versions, &latest_version);

    let dependency_count = count_dependencies(&package.versions, &latest_version);

    let project_data = ProjectData {
        name: package.name.clone(),
        url: format!("https://www.npmjs.com/package/{}", package_name),
        source: "npm".to_string(),
        source_id: package_name.to_string(),
        description: package.description,
        languages: Some("JavaScript/TypeScript".to_string()),
        stars: 0,
        forks: 0,
        open_issues: 0,
        license: package.license,
        homepage,
        latest_commit: None,
        latest_release: Some(latest_version),
        readme_content: None,
    };

    ImportResult {
        success: true,
        project_id: None,
        error: None,
        project_data: Some(project_data),
        readme_variants: None,
    }
}

async fn fetch_npm_downloads(package_name: &str, client: &reqwest::Client) -> i64 {
    let downloads_url = format!(
        "https://api.npmjs.org/downloads/point/last-month/{}",
        urlencoding::encode(package_name)
    );

    match client.get(&downloads_url).send().await {
        Ok(resp) => {
            if resp.status().is_success() {
                if let Ok(downloads) = resp.json::<NpmDownloads>().await {
                    return downloads.downloads;
                }
            }
        }
        Err(_) => {}
    }
    0
}

fn count_dependencies(versions: &serde_json::Value, latest_version: &str) -> usize {
    if let Some(obj) = versions.get(latest_version).and_then(|v| v.get("dependencies")) {
        if let Some(deps) = obj.as_object() {
            return deps.len();
        }
    }
    0
}

fn extract_repository_from_versions(versions: &serde_json::Value, latest_version: &str) -> Option<String> {
    if let Some(obj) = versions.get(latest_version) {
        if let Some(repository) = obj.get("repository").and_then(|r| r.get("url")).and_then(|u| u.as_str()) {
            let url = repository.to_string();
            if url.starts_with("git+") {
                return Some(url[4..].to_string());
            }
            return Some(url);
        }
    }
    None
}

pub fn calculate_npm_health_score(
    monthly_downloads: i64,
    version_count: usize,
    dependency_count: usize,
    has_homepage: bool,
    has_repository: bool,
    license: Option<&str>,
) -> f64 {
    let mut score = 0.0;

    let download_score = match monthly_downloads {
        d if d >= 10_000_000 => 100.0,
        d if d >= 1_000_000 => 80.0,
        d if d >= 100_000 => 60.0,
        d if d >= 10_000 => 40.0,
        _ => 20.0,
    };
    score += download_score * 0.30;

    let version_score = match version_count {
        v if v >= 100 => 100.0,
        v if v >= 50 => 80.0,
        v if v >= 20 => 60.0,
        v if v >= 10 => 40.0,
        _ => 20.0,
    };
    score += version_score * 0.25;

    let dep_score = match dependency_count {
        d if d <= 10 => 100.0,
        d if d <= 30 => 80.0,
        d if d <= 50 => 60.0,
        d if d <= 100 => 40.0,
        _ => 20.0,
    };
    score += dep_score * 0.20;

    let homepage_score = if has_homepage { 100.0 } else { 30.0 };
    score += homepage_score * 0.10;

    let repo_score = if has_repository { 100.0 } else { 30.0 };
    score += repo_score * 0.10;

    let license_score = license
        .map(|l| {
            let l_upper = l.to_uppercase();
            if l_upper.contains("MIT") || l_upper.contains("APACHE") || l_upper.contains("BSD") || l_upper.contains("ISC") {
                100.0
            } else if l_upper.contains("GPL") || l_upper.contains("LGPL") {
                80.0
            } else if l_upper.contains("PROPRIETARY") || l_upper.contains("UNLICENSED") {
                0.0
            } else {
                60.0
            }
        })
        .unwrap_or(50.0);
    score += license_score * 0.05;

    score
}
