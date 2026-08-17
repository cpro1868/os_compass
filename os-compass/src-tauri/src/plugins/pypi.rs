use crate::plugins::{extract_source_id, ImportResult, Platform, ProjectData};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct PyPIPackage {
    info: PyPIInfo,
    releases: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct PyPIInfo {
    name: String,
    version: String,
    summary: Option<String>,
    home_page: Option<String>,
    project_urls: Option<serde_json::Value>,
    license: Option<String>,
    classifiers: Option<Vec<String>>,
    #[serde(rename = "classifiers")]
    _classifiers: Option<Vec<String>>,
    #[serde(rename = "requires_python")]
    requires_python: Option<String>,
    #[serde(rename = "yanked")]
    yanked: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct PyPIProjectUrls {
    Homepage: Option<String>,
    Documentation: Option<String>,
    Repository: Option<String>,
    #[serde(flatten)]
    extra: serde_json::Map<String, serde_json::Value>,
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

pub async fn fetch_pypi_project(url: &str, proxy: Option<&str>) -> ImportResult {
    let source_id = match extract_source_id(url, Platform::PyPI) {
        Some(id) => id,
        None => return ImportResult {
            success: false,
            project_id: None,
            error: Some("Invalid PyPI package URL or name".to_string()),
            project_data: None,
            readme_variants: None,
        },
    };

    let client = build_client(proxy);
    let package_name = source_id.trim();

    let package_url = format!(
        "https://pypi.org/pypi/{}/json",
        urlencoding::encode(package_name)
    );

    let package: PyPIPackage = match client.get(&package_url).send().await {
        Ok(resp) => {
            if resp.status().is_success() {
                match resp.json().await {
                    Ok(p) => p,
                    Err(e) => return ImportResult {
                        success: false,
                        project_id: None,
                        error: Some(format!("Failed to parse PyPI response: {}", e)),
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
                    error: Some(format!("PyPI API error: {}", resp.status())),
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

    let homepage = extract_homepage(&package.info);
    let repository_url = extract_repository(&package.info);
    let release_count = count_releases(&package.releases);
    let classifier_count = package.info.classifiers.as_ref().map(|c| c.len()).unwrap_or(0);
    let python_version = package.info.requires_python.clone();

    let yanked = package.info.yanked.unwrap_or(false);

    let project_data = ProjectData {
        name: package.info.name.clone(),
        url: format!("https://pypi.org/project/{}", package_name),
        source: "pypi".to_string(),
        source_id: package_name.to_string(),
        description: package.info.summary.clone(),
        languages: Some("Python".to_string()),
        stars: 0,
        forks: 0,
        open_issues: 0,
        license: package.info.license.clone(),
        homepage,
        latest_commit: None,
        latest_release: Some(package.info.version.clone()),
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

fn extract_homepage(info: &PyPIInfo) -> Option<String> {
    if let Some(ref urls) = info.project_urls {
        if let Ok(homepage) = serde_json::from_value::<PyPIProjectUrls>(urls.clone()) {
            if homepage.Homepage.is_some() {
                return homepage.Homepage;
            }
            if homepage.Documentation.is_some() {
                return homepage.Documentation;
            }
            if homepage.Repository.is_some() {
                return homepage.Repository;
            }
        }
    }
    info.home_page.clone()
}

fn extract_repository(info: &PyPIInfo) -> Option<String> {
    if let Some(ref urls) = info.project_urls {
        if let Ok(project_urls) = serde_json::from_value::<PyPIProjectUrls>(urls.clone()) {
            if project_urls.Repository.is_some() {
                return project_urls.Repository;
            }
        }
        if let Some(obj) = urls.as_object() {
            for (_, value) in obj {
                if let Some(key) = value.get("name").and_then(|n| n.as_str()) {
                    let key_lower = key.to_lowercase();
                    if key_lower.contains("repo") || key_lower.contains("source") || key_lower.contains("code") {
                        if let Some(url) = value.get("url").and_then(|u| u.as_str()) {
                            return Some(url.to_string());
                        }
                    }
                }
            }
        }
    }
    None
}

fn count_releases(releases: &serde_json::Value) -> usize {
    if let Some(obj) = releases.as_object() {
        obj.len()
    } else {
        0
    }
}

pub fn calculate_pypi_health_score(
    release_count: usize,
    classifier_count: usize,
    has_documentation: bool,
    has_repository: bool,
    license: Option<&str>,
    yanked: bool,
) -> f64 {
    if yanked {
        return 10.0;
    }

    let mut score = 0.0;

    let release_score = match release_count {
        r if r >= 50 => 100.0,
        r if r >= 20 => 80.0,
        r if r >= 10 => 60.0,
        r if r >= 5 => 40.0,
        _ => 20.0,
    };
    score += release_score * 0.25;

    let classifier_score = match classifier_count {
        c if c >= 10 => 100.0,
        c if c >= 5 => 80.0,
        c if c >= 3 => 60.0,
        _ => 40.0,
    };
    score += classifier_score * 0.15;

    let docs_score = if has_documentation { 100.0 } else { 50.0 };
    score += docs_score * 0.15;

    let repo_score = if has_repository { 100.0 } else { 30.0 };
    score += repo_score * 0.25;

    let license_score = license
        .map(|l| {
            let l_upper = l.to_uppercase();
            if l_upper.contains("MIT") || l_upper.contains("APACHE") || l_upper.contains("BSD") || l_upper.contains("ISC") {
                100.0
            } else if l_upper.contains("GPL") {
                80.0
            } else if l_upper.contains("PROPRIETARY") || l_upper.contains("UNKNOWN") {
                20.0
            } else {
                60.0
            }
        })
        .unwrap_or(50.0);
    score += license_score * 0.15;

    let yanked_score = if yanked { 0.0 } else { 100.0 };
    score += yanked_score * 0.05;

    score
}
