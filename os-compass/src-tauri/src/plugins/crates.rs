use crate::plugins::{extract_source_id, ImportResult, Platform, ProjectData};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct CratesCrate {
    crate_info: CratesCrateInfo,
    versions: Option<Vec<CratesVersion>>,
}

#[derive(Debug, Deserialize)]
struct CratesCrateInfo {
    name: String,
    description: Option<String>,
    license: Option<String>,
    repository: Option<String>,
    homepage: Option<String>,
    documentation: Option<String>,
    max_version: String,
    downloads: i64,
    recent_downloads: Option<i64>,
    categories: Option<Vec<String>>,
    keywords: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct CratesVersion {
    num: String,
    license: Option<String>,
    rust_version: Option<String>,
    yanked: Option<bool>,
    downloads: i64,
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

pub async fn fetch_crates_project(url: &str, proxy: Option<&str>) -> ImportResult {
    let source_id = match extract_source_id(url, Platform::Crates) {
        Some(id) => id,
        None => return ImportResult {
            success: false,
            project_id: None,
            error: Some("Invalid Crates.io crate URL or name".to_string()),
            project_data: None,
            readme_variants: None,
        },
    };

    let client = build_client(proxy);
    let crate_name = source_id.trim();

    let crate_url = format!(
        "https://crates.io/api/v1/crates/{}",
        urlencoding::encode(crate_name)
    );

    let crate_data: CratesCrate = match client.get(&crate_url).send().await {
        Ok(resp) => {
            if resp.status().is_success() {
                match resp.json().await {
                    Ok(c) => c,
                    Err(e) => return ImportResult {
                        success: false,
                        project_id: None,
                        error: Some(format!("Failed to parse Crates.io response: {}", e)),
                        project_data: None,
                        readme_variants: None,
                    },
                }
            } else if resp.status().as_u16() == 404 {
                return ImportResult {
                    success: false,
                    project_id: None,
                    error: Some(format!("Crate '{}' not found", crate_name)),
                    project_data: None,
                    readme_variants: None,
                };
            } else {
                return ImportResult {
                    success: false,
                    project_id: None,
                    error: Some(format!("Crates.io API error: {}", resp.status())),
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

    let homepage = crate_data.crate_info.homepage.clone()
        .or_else(|| crate_data.crate_info.documentation.clone());
    let repository_url = crate_data.crate_info.repository.clone();
    let downloads = crate_data.crate_info.downloads;
    let recent_downloads = crate_data.crate_info.recent_downloads.unwrap_or(0);
    let category_count = crate_data.crate_info.categories.as_ref().map(|c| c.len()).unwrap_or(0)
        + crate_data.crate_info.keywords.as_ref().map(|k| k.len()).unwrap_or(0);

    let latest_version = crate_data.crate_info.max_version.clone();
    let rust_version = get_latest_rust_version(&crate_data.versions);

    let project_data = ProjectData {
        name: crate_data.crate_info.name.clone(),
        url: format!("https://crates.io/crates/{}", crate_name),
        source: "crates".to_string(),
        source_id: crate_name.to_string(),
        description: crate_data.crate_info.description.clone(),
        languages: Some("Rust".to_string()),
        stars: downloads as i32,
        forks: 0,
        open_issues: 0,
        license: crate_data.crate_info.license.clone(),
        homepage,
        latest_commit: rust_version,
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

fn get_latest_rust_version(versions: &Option<Vec<CratesVersion>>) -> Option<String> {
    versions
        .as_ref()
        .and_then(|v| v.first())
        .and_then(|ver| ver.rust_version.clone())
}

pub fn calculate_crates_health_score(
    downloads: i64,
    recent_downloads: i64,
    has_documentation: bool,
    has_repository: bool,
    category_count: usize,
    license: Option<&str>,
) -> f64 {
    let mut score = 0.0;

    let download_score = match downloads {
        d if d >= 10_000_000 => 100.0,
        d if d >= 1_000_000 => 80.0,
        d if d >= 100_000 => 60.0,
        d if d >= 10_000 => 40.0,
        _ => 20.0,
    };
    score += download_score * 0.25;

    let recent_score = match recent_downloads {
        d if d >= 100_000 => 100.0,
        d if d >= 10_000 => 80.0,
        d if d >= 1_000 => 60.0,
        d if d >= 100 => 40.0,
        _ => 20.0,
    };
    score += recent_score * 0.20;

    let docs_score = if has_documentation { 100.0 } else if has_repository { 60.0 } else { 20.0 };
    score += docs_score * 0.20;

    let repo_score = if has_repository { 100.0 } else { 30.0 };
    score += repo_score * 0.10;

    let category_score = match category_count {
        c if c >= 5 => 100.0,
        c if c >= 3 => 80.0,
        c if c >= 1 => 60.0,
        _ => 30.0,
    };
    score += category_score * 0.15;

    let license_score = license
        .map(|l| {
            let l_upper = l.to_uppercase();
            if l_upper.contains("MIT") || l_upper.contains("APACHE") || l_upper.contains("BSD") || l_upper.contains("ISC") {
                100.0
            } else if l_upper.contains("GPL") {
                80.0
            } else if l_upper.contains("PROPRIETARY") || l_upper.contains("UNLICENSE") {
                20.0
            } else {
                60.0
            }
        })
        .unwrap_or(50.0);
    score += license_score * 0.10;

    score
}
