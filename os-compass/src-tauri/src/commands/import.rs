use crate::crawler_service;
use crate::db::DATABASE;
use crate::embedding;
use crate::plugins::{self, crawler, gitee, github, npm, pypi, crates, ImportResult, Platform, ProjectData};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct ImportInput {
    pub url: String,
    #[serde(default)]
    pub category_id: Option<i64>,
    #[serde(default = "default_true")]
    pub generate_ai_report: bool,
    #[serde(default = "default_true")]
    pub auto_translate: bool,
    #[serde(default)]
    pub github_token: Option<String>,
    #[serde(default)]
    pub gitee_token: Option<String>,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Serialize)]
pub struct ImportResponse {
    pub success: bool,
    pub project_id: Option<i64>,
    pub error: Option<String>,
    pub data: Option<ProjectData>,
}

#[tauri::command]
pub async fn import_project(input: ImportInput) -> ImportResponse {
    let url = input.url.trim();
    
    if url.is_empty() {
        return ImportResponse {
            success: false,
            project_id: None,
            error: Some("URL is required".to_string()),
            data: None,
        };
    }

    let result = match plugins::identify_platform(url) {
        Platform::GitHub => {
            let proxy = crate::commands::variables::get_variable_value_internal("github_proxy");
            github::fetch_github_project(url, input.github_token.as_deref(), proxy.as_deref()).await
        }
        Platform::Gitee => {
            gitee::fetch_gitee_project(url, input.gitee_token.as_deref()).await
        }
        Platform::NPM => {
            npm::fetch_npm_project(url, None).await
        }
        Platform::PyPI => {
            pypi::fetch_pypi_project(url, None).await
        }
        Platform::Crates => {
            crates::fetch_crates_project(url, None).await
        }
        Platform::Unknown => {
            match crawler_service::start_crawler_service() {
                Ok(_) => {
                    let result = crawler::fetch_with_crawler(url).await;
                    crawler_service::stop_crawler_service();
                    result
                }
                Err(e) => ImportResult {
                    success: false,
                    project_id: None,
                    error: Some(format!("Failed to start crawler service: {}", e)),
                    project_data: None,
                },
            }
        }
    };

    match result {
        ImportResult {
            success: true,
            project_id: _,
            error: _,
            project_data: Some(data),
        } => {
            let db = match DATABASE.lock() {
                Ok(db) => db,
                Err(e) => return ImportResponse {
                    success: false,
                    project_id: None,
                    error: Some(e.to_string()),
                    data: None,
                },
            };
            
            if let Some(db) = db.as_ref() {
                let conn = db.get_connection();
                let result = conn.execute(
                    r#"INSERT INTO projects
                       (name, url, source, source_id, description, languages, stars, forks, open_issues, license, homepage, latest_commit, readme_content, created_at, updated_at)
                       VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, datetime('now', 'localtime'), datetime('now', 'localtime'))"#,
                    rusqlite::params![
                        data.name,
                        data.url,
                        data.source,
                        data.source_id,
                        data.description,
                        data.languages,
                        data.stars,
                        data.forks,
                        data.open_issues,
                        data.license,
                        data.homepage,
                        data.latest_commit,
                        data.readme_content,
                    ],
                );

                match result {
                    Ok(_) => {
                        let project_id = conn.last_insert_rowid();
                        
                        drop(conn);
                        drop(db);

                        if embedding::is_enabled() {
                            tokio::spawn(async move {
                                let _ = embedding::generate_project_embeddings(project_id).await;
                            });
                        }

                        ImportResponse {
                            success: true,
                            project_id: Some(project_id),
                            error: None,
                            data: Some(data),
                        }
                    }
                    Err(e) => ImportResponse {
                        success: false,
                        project_id: None,
                        error: Some(format!("Database error: {}", e)),
                        data: None,
                    },
                }
            } else {
                ImportResponse {
                    success: false,
                    project_id: None,
                    error: Some("Database not initialized".to_string()),
                    data: None,
                }
            }
        }
        ImportResult {
            success: false,
            project_id: _,
            error: Some(err),
            data: _,
        } => ImportResponse {
            success: false,
            project_id: None,
            error: Some(err),
            data: None,
        },
        _ => ImportResponse {
            success: false,
            project_id: None,
            error: Some("Unknown error".to_string()),
            data: None,
        },
    }
}

#[tauri::command]
pub async fn preview_import(url: String) -> ImportResult {
    let url = url.trim();
    
    if url.is_empty() {
        return ImportResult {
            success: false,
            project_id: None,
            error: Some("URL is required".to_string()),
            project_data: None,
        };
    }

    match plugins::identify_platform(url) {
        Platform::GitHub => {
            let proxy = crate::commands::variables::get_variable_value_internal("github_proxy");
            let token = crate::commands::variables::get_variable_value_internal("github_token");
            github::fetch_github_project(url, token.as_deref(), proxy.as_deref()).await
        }
        Platform::Gitee => {
            gitee::fetch_gitee_project(url, None).await
        }
        Platform::NPM => {
            npm::fetch_npm_project(url, None).await
        }
        Platform::PyPI => {
            pypi::fetch_pypi_project(url, None).await
        }
        Platform::Crates => {
            crates::fetch_crates_project(url, None).await
        }
        Platform::Unknown => {
            crawler::fetch_with_crawler(url).await
        }
    }
}
