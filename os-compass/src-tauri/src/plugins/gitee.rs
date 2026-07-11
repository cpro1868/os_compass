use crate::plugins::{extract_source_id, ImportResult, Platform, ProjectData};

pub async fn fetch_gitee_project(url: &str, token: Option<&str>) -> ImportResult {
    let platform = Platform::Gitee;
    let source_id = match extract_source_id(url, platform) {
        Some(id) => id,
        None => return ImportResult {
            success: false,
            project_id: None,
            error: Some("Invalid Gitee URL".to_string()),
            project_data: None,
            readme_variants: None,
        },
    };

    let parts: Vec<&str> = source_id.split('/').collect();
    if parts.len() != 2 {
        return ImportResult {
            success: false,
            project_id: None,
            error: Some("Invalid Gitee project path".to_string()),
            project_data: None,
            readme_variants: None,
        };
    }

    let owner = parts[0];
    let repo_name = parts[1];

    let api_url = format!(
        "https://gitee.com/api/v5/repos/{}/{}",
        owner, repo_name
    );

    let client = reqwest::Client::new();
    
    let mut request = client.get(&api_url)
        .query(&[("owner", owner), ("repo", repo_name)]);
    
    if let Some(t) = token {
        request = request.query(&[("access_token", t)]);
    }

    #[derive(serde::Deserialize)]
    struct GiteeRepo {
        name: String,
        html_url: String,
        description: Option<String>,
        language: Option<String>,
        stargazers_count: i64,
        forks_count: i64,
        open_issues_count: i64,
        license: Option<String>,
        homepage: Option<String>,
        pushed_at: Option<String>,
    }

    let repo: GiteeRepo = match request.send().await {
        Ok(resp) => {
            if resp.status() == 200 {
                match resp.json().await {
                    Ok(r) => r,
                    Err(e) => return ImportResult {
                        success: false,
                        project_id: None,
                        error: Some(format!("Failed to parse Gitee response: {}", e)),
                        project_data: None,
                        readme_variants: None,
                    },
                }
            } else {
                return ImportResult {
                    success: false,
                    project_id: None,
                    error: Some(format!("Gitee API error: {}", resp.status())),
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

    let project_data = ProjectData {
        name: repo.name,
        url: repo.html_url,
        source: "gitee".to_string(),
        source_id,
        description: repo.description,
        languages: repo.language,
        stars: repo.stargazers_count as i32,
        forks: repo.forks_count as i32,
        open_issues: repo.open_issues_count as i32,
        license: repo.license,
        homepage: repo.homepage,
        latest_commit: repo.pushed_at,
        latest_release: None,
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
