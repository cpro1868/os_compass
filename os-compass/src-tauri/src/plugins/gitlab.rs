//! GitLab 项目导入插件（GitLab API v4）
//! 匿名可读公开项目（约 500 次/窗口），配置 gitlab_token 可提高限额并访问私有项目。

use crate::plugins::{extract_source_id, ImportResult, Platform, ProjectData};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct GitLabProject {
    id: i64,
    name: String,
    web_url: String,
    description: Option<String>,
    star_count: Option<i64>,
    forks_count: Option<i64>,
    open_issues_count: Option<i64>,
    last_activity_at: Option<String>,
}

fn build_client(proxy: Option<&str>) -> reqwest::Client {
    let mut builder = reqwest::Client::builder()
        .user_agent(concat!("os-compass/", env!("CARGO_PKG_VERSION")))
        .timeout(std::time::Duration::from_secs(30));
    if let Some(p) = proxy {
        if !p.trim().is_empty() {
            if let Ok(proxy_obj) = reqwest::Proxy::all(p.trim()) {
                builder = builder.proxy(proxy_obj);
            }
        }
    }
    builder.build().unwrap_or_else(|_| reqwest::Client::new())
}

fn err_result(msg: String) -> ImportResult {
    ImportResult {
        success: false,
        project_id: None,
        error: Some(msg),
        project_data: None,
        readme_variants: None,
    }
}

pub async fn fetch_gitlab_project(url: &str, token: Option<&str>, proxy: Option<&str>) -> ImportResult {
    let source_id = match extract_source_id(url, Platform::GitLab) {
        Some(id) => id,
        None => return err_result("Invalid GitLab URL".to_string()),
    };

    let client = build_client(proxy);
    let api_url = format!(
        "https://gitlab.com/api/v4/projects/{}",
        urlencoding::encode(&source_id)
    );

    let mut request = client.get(&api_url);
    if let Some(t) = token {
        if !t.trim().is_empty() {
            request = request.header("PRIVATE-TOKEN", t.trim());
        }
    }

    let project: GitLabProject = match request.send().await {
        Ok(resp) => {
            if resp.status().is_success() {
                match resp.json().await {
                    Ok(p) => p,
                    Err(e) => return err_result(format!("Failed to parse GitLab response: {}", e)),
                }
            } else if resp.status().as_u16() == 404 {
                return err_result(format!("GitLab project '{}' not found", source_id));
            } else {
                return err_result(format!("GitLab API error: {}", resp.status()));
            }
        }
        Err(e) => return err_result(format!("Network error: {}", e)),
    };

    let readme_content = fetch_gitlab_readme(project.id, &client, token).await;

    let project_data = ProjectData {
        name: project.name,
        url: project.web_url,
        source: "gitlab".to_string(),
        source_id,
        description: project.description,
        languages: None,
        stars: project.star_count.unwrap_or(0) as i32,
        forks: project.forks_count.unwrap_or(0) as i32,
        open_issues: project.open_issues_count.unwrap_or(0) as i32,
        license: None,
        homepage: None,
        latest_commit: project.last_activity_at,
        latest_release: None,
        readme_content,
        health_score: None,
    };

    ImportResult {
        success: true,
        project_id: None,
        error: None,
        project_data: Some(project_data),
        readme_variants: None,
    }
}

/// 依次尝试常见 README 文件名，取第一个成功命中的
async fn fetch_gitlab_readme(project_id: i64, client: &reqwest::Client, token: Option<&str>) -> Option<String> {
    for file in ["README.md", "readme.md", "README.rst", "README"] {
        let api_url = format!(
            "https://gitlab.com/api/v4/projects/{}/repository/files/{}/raw?ref=HEAD",
            project_id,
            urlencoding::encode(file)
        );
        let mut request = client.get(&api_url);
        if let Some(t) = token {
            if !t.trim().is_empty() {
                request = request.header("PRIVATE-TOKEN", t.trim());
            }
        }
        if let Ok(resp) = request.send().await {
            if resp.status().is_success() {
                if let Ok(text) = resp.text().await {
                    if !text.trim().is_empty() {
                        return Some(text);
                    }
                }
            }
        }
    }
    None
}
