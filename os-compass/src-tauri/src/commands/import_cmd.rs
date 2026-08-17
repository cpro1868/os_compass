use crate::crawler_service;
use crate::db::DATABASE;
use crate::plugins::{self, crawler, gitee, github, npm, pypi, crates, ImportResult, Platform, ProjectData};
use serde::{Deserialize, Serialize};
use tauri::Emitter;
use log;

fn normalize_url(url: &str) -> String {
    let trimmed = url.trim().trim_end_matches('/');
    if let Some(scheme_pos) = trimmed.find("://") {
        let (scheme, rest) = trimmed.split_at(scheme_pos + 3);
        if let Some(path_pos) = rest.find('/') {
            let (domain, path) = rest.split_at(path_pos);
            let no_query = path.split(['?', '#']).next().unwrap_or("");
            return format!("{}{}{}", scheme, domain.to_lowercase(), no_query.trim_end_matches('/'));
        }
        return format!("{}{}", scheme, rest.to_lowercase());
    }
    trimmed.to_lowercase()
}

fn check_url_exists(url: &str) -> Option<(i64, String)> {
    let normalized = normalize_url(url);
    let db = DATABASE.lock().ok()?;
    let db = db.as_ref()?;
    let conn = db.get_connection();
    let existing_urls: Vec<(String, String)> = conn.prepare("SELECT url, name FROM projects WHERE data_status = 'ACTIVE'").ok()?
        .query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))).ok()?
        .filter_map(|r| r.ok())
        .collect();
    for (stored_url, name) in existing_urls {
        if normalize_url(&stored_url) == normalized {
            let id: i64 = conn.query_row("SELECT id FROM projects WHERE url = ? AND data_status = 'ACTIVE'", [&stored_url], |row| row.get(0)).ok()?;
            return Some((id, name));
        }
    }
    None
}

async fn start_crawler_and_fetch(url: &str) -> ImportResult {
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
            readme_variants: None,
        },
    }
}

#[derive(Debug, Deserialize)]
pub struct ImportInput {
    pub url: String,
    #[serde(default)]
    pub github_token: Option<String>,
    #[serde(default)]
    pub gitee_token: Option<String>,
    #[serde(default)]
    pub auto_translate: Option<bool>,
    #[serde(default)]
    pub default_language: Option<String>,
}

fn get_variable_value(key: &str) -> Option<String> {
    crate::commands::variables::get_variable_value_internal(key)
}

#[derive(Debug, Serialize)]
pub struct ImportResponse {
    pub success: bool,
    pub project_id: Option<i64>,
    pub error: Option<String>,
    pub data: Option<ProjectData>,
    pub duplicate: Option<DuplicateInfo>,
}

#[derive(Debug, Serialize)]
pub struct DuplicateInfo {
    pub project_id: i64,
    pub project_name: String,
    pub url: String,
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
            duplicate: None,
        };
    }

    if let Some((existing_id, existing_name)) = check_url_exists(url) {
        return ImportResponse {
            success: false,
            project_id: None,
            error: Some(format!("项目已存在：{}", existing_name)),
            data: None,
            duplicate: Some(DuplicateInfo {
                project_id: existing_id,
                project_name: existing_name,
                url: url.to_string(),
            }),
        };
    }

    let result = match plugins::identify_platform(url) {
        Platform::GitHub => {
            let token = input.github_token
                .or_else(|| get_variable_value("github_token"));
            let proxy = get_variable_value("github_proxy");
            let api_result = github::fetch_github_project(url, token.as_deref(), proxy.as_deref()).await;
            if !api_result.success {
                let fallback_result = start_crawler_and_fetch(url).await;
                if fallback_result.success {
                    fallback_result
                } else {
                    api_result
                }
            } else {
                api_result
            }
        }
        Platform::Gitee => {
            let token = input.gitee_token
                .or_else(|| get_variable_value("gitee_token"));
            let api_result = gitee::fetch_gitee_project(url, token.as_deref()).await;
            if !api_result.success {
                let fallback_result = start_crawler_and_fetch(url).await;
                if fallback_result.success {
                    fallback_result
                } else {
                    api_result
                }
            } else {
                api_result
            }
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
            start_crawler_and_fetch(url).await
        }
    };

    match result {
        ImportResult {
            success: true,
            project_id: _,
            error: _,
            project_data: Some(data),
            readme_variants: variants,
        } => {
            let db = match DATABASE.lock() {
                Ok(db) => db,
                Err(e) => return ImportResponse {
                    success: false,
                    project_id: None,
                    error: Some(e.to_string()),
                    data: None, duplicate: None, },
            };
            
            if let Some(db) = db.as_ref() {
                let conn = db.get_connection();
                let insert_result = conn.execute(
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

                match insert_result {
                    Ok(_) => {
                        let project_id = conn.last_insert_rowid();
                        if let Some(v) = variants {
                            for (file_name, content) in v {
                                let language = crate::commands::project_extra::detect_readme_lang(&file_name);
                                let _ = conn.execute(
                                    "INSERT INTO readme_variants (project_id, file_name, language, content) VALUES (?, ?, ?, ?)",
                                    rusqlite::params![project_id, file_name, language, content],
                                );
                            }
                        }
                        ImportResponse {
                            success: true,
                            project_id: Some(project_id),
                            error: None,
                            data: Some(data), duplicate: None, }
                    }
                    Err(e) => ImportResponse {
                        success: false,
                        project_id: None,
                        error: Some(format!("Database error: {}", e)),
                        data: None, duplicate: None, },
                }
            } else {
                ImportResponse {
                    success: false,
                    project_id: None,
                    error: Some("Database not initialized".to_string()),
                    data: None, duplicate: None, }
            }
        }
        ImportResult {
            success: false,
            project_id: _,
            error: Some(err),
            project_data: _,
            readme_variants: _,
        } => ImportResponse {
            success: false,
            project_id: None,
            error: Some(err),
            data: None, duplicate: None, },
        _ => ImportResponse {
            success: false,
            project_id: None,
            error: Some("Unknown error".to_string()),
            data: None, duplicate: None, },
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
            readme_variants: None,
        };
    }

    match plugins::identify_platform(url) {
        Platform::GitHub => {
            let token = get_variable_value("github_token");
            let proxy = get_variable_value("github_proxy");
            let api_result = github::fetch_github_project(url, token.as_deref(), proxy.as_deref()).await;
            if !api_result.success {
                let fallback_result = start_crawler_and_fetch(url).await;
                if fallback_result.success {
                    fallback_result
                } else {
                    api_result
                }
            } else {
                api_result
            }
        }
        Platform::Gitee => {
            let token = get_variable_value("gitee_token");
            let api_result = gitee::fetch_gitee_project(url, token.as_deref()).await;
            if !api_result.success {
                let fallback_result = start_crawler_and_fetch(url).await;
                if fallback_result.success {
                    fallback_result
                } else {
                    api_result
                }
            } else {
                api_result
            }
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
            start_crawler_and_fetch(url).await
        }
    }
}

#[derive(Debug, Serialize, Clone)]
pub struct TaskProgress {
    pub project_id: i64,
    pub task_type: String,
    pub status: String,
    pub error: Option<String>,
}

#[tauri::command]
pub async fn post_import_tasks(
    app: tauri::AppHandle,
    project_id: i64,
    generate_ai_report: bool,
    auto_translate: bool,
    auto_translate_readme: bool,
    default_language: Option<String>,
) -> Result<(), String> {
    let lang = default_language.unwrap_or_else(|| "zh-CN".to_string());
    let should_translate = lang != "en-US" && lang != "en";

    let app_clone = app.clone();
    tauri::async_runtime::spawn(async move {
        run_post_import_tasks(app_clone, project_id, generate_ai_report, auto_translate, auto_translate_readme, should_translate, lang).await;
    });

    Ok(())
}

async fn run_post_import_tasks(
    app: tauri::AppHandle,
    project_id: i64,
    generate_ai_report: bool,
    auto_translate: bool,
    auto_translate_readme: bool,
    should_translate: bool,
    lang: String,
) {
    let emit_progress = |task_type: &str, status: &str, error: Option<String>| {
        let _ = app.emit("import:task_progress", TaskProgress {
            project_id,
            task_type: task_type.to_string(),
            status: status.to_string(),
            error,
        });
    };

    if generate_ai_report {
        emit_progress("ai_analysis", "running", None);
        let result = crate::commands::ai::analyze_project(project_id).await;
        if result.error.is_none() {
            emit_progress("ai_analysis", "done", None);

            if should_translate && auto_translate {
                let ai_data: Option<(Option<String>, Option<String>, Option<String>, Option<String>)> = {
                    if let Ok(db) = DATABASE.lock() {
                        if let Some(db) = db.as_ref() {
                            let conn = db.get_connection();
                            conn.query_row(
                                "SELECT ai_summary, ai_use_cases, ai_risks, ai_dependencies FROM projects WHERE id = ?",
                                [project_id],
                                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
                            ).ok()
                        } else { None }
                    } else { None }
                };

                if let Some((summary, use_cases, risks, deps)) = ai_data {
                    translate_ai_field(&app, project_id, &lang, "summary", summary).await;
                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                    translate_ai_field(&app, project_id, &lang, "use_cases", use_cases).await;
                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                    translate_ai_field(&app, project_id, &lang, "risks", risks).await;
                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                    translate_ai_field(&app, project_id, &lang, "dependencies", deps).await;
                }
            }
        } else {
            emit_progress("ai_analysis", "failed", result.error.clone());
        }

        emit_progress("generate_tags", "running", None);
        match crate::commands::ai::generate_tags(project_id).await {
            Ok(_) => emit_progress("generate_tags", "done", None),
            Err(e) => emit_progress("generate_tags", "failed", Some(e)),
        }
    }

    if should_translate && auto_translate {
        let project_data: Option<(Option<String>, Option<String>)> = {
            if let Ok(db) = DATABASE.lock() {
                if let Some(db) = db.as_ref() {
                    let conn = db.get_connection();
                    conn.query_row(
                        "SELECT description, readme_content FROM projects WHERE id = ?",
                        [project_id],
                        |row| Ok((row.get(0)?, row.get(1)?)),
                    ).ok()
                } else { None }
            } else { None }
        };

        if let Some((desc, readme)) = project_data {
            if let Some(desc_text) = desc {
                if !desc_text.trim().is_empty() {
                    emit_progress("description", "running", None);
                    match crate::translate::translate_text_with_llm(&desc_text, &lang).await {
                        Ok(translated) => {
                            if let Ok(db) = DATABASE.lock() {
                                if let Some(db) = db.as_ref() {
                                    let conn = db.get_connection();
                                    let _ = conn.execute(
                                        "INSERT INTO translations (project_id, field_name, language, content, updated_at) VALUES (?, ?, ?, ?, datetime('now', 'localtime')) ON CONFLICT(project_id, field_name, language) DO UPDATE SET content = excluded.content, updated_at = excluded.updated_at",
                                        rusqlite::params![project_id, "description", lang, translated],
                                    );
                                }
                            }
                            emit_progress("description", "done", None);
                        }
                        Err(e) => emit_progress("description", "failed", Some(e)),
                    }
                }
            }

            if auto_translate_readme {
                if let Some(readme_text) = readme {
                    if !readme_text.trim().is_empty() {
                        emit_progress("readme_translation", "running", None);
                        match crate::translate::translate_text(&readme_text, &lang).await {
                            Ok(translated) => {
                                if let Ok(db) = DATABASE.lock() {
                                    if let Some(db) = db.as_ref() {
                                        let conn = db.get_connection();
                                        let _ = conn.execute(
                                            "UPDATE projects SET readme_translation = ? WHERE id = ?",
                                            rusqlite::params![translated, project_id],
                                        );
                                    }
                                }
                                emit_progress("readme_translation", "done", None);
                            }
                            Err(e) => emit_progress("readme_translation", "failed", Some(e)),
                        }
                    }
                }
            }
        }
    }

    if generate_ai_report {
        emit_progress("runbook", "running", None);
        match crate::commands::runbook_cmd::generate_runbook(project_id).await {
            Ok(runbook) => {
                if !runbook.trim().is_empty() {
                    if let Ok(db) = DATABASE.lock() {
                        if let Some(db) = db.as_ref() {
                            let conn = db.get_connection();
                            let _ = conn.execute(
                                "UPDATE projects SET runbook = ?, updated_at = datetime('now', 'localtime') WHERE id = ?",
                                rusqlite::params![runbook, project_id],
                            );
                        }
                    }
                }
                emit_progress("runbook", "done", None);
            }
            Err(e) => emit_progress("runbook", "failed", Some(e)),
        }
    }

    let _ = app.emit("import:task_progress", TaskProgress {
        project_id,
        task_type: "all".to_string(),
        status: "done".to_string(),
        error: None,
    });
}

async fn translate_ai_field(
    app: &tauri::AppHandle,
    project_id: i64,
    lang: &str,
    field_name: &str,
    text: Option<String>,
) {
    if let Some(t) = text {
        if t.trim().is_empty() { return; }
        log::debug!("[TRANSLATE_AI] Starting {} (len={})", field_name, t.len());
        let _ = app.emit("import:task_progress", TaskProgress {
            project_id,
            task_type: field_name.to_string(),
            status: "running".to_string(),
            error: None,
        });

        let mut success = false;
        let mut last_error = String::new();
        for attempt in 0..3u32 {
            if attempt > 0 {
                log::debug!("[TRANSLATE_AI] {} retry {} after 3s", field_name, attempt);
                tokio::time::sleep(std::time::Duration::from_secs(3)).await;
            }
            match crate::translate::translate_text_with_llm(&t, lang).await {
                Ok(translated) => {
                    if !translated.trim().is_empty() {
                        log::debug!("[TRANSLATE_AI] {} success (len={})", field_name, translated.len());
                        if let Ok(db) = DATABASE.lock() {
                            if let Some(db) = db.as_ref() {
                                let conn = db.get_connection();
                                let _ = conn.execute(
                                    "INSERT INTO translations (project_id, field_name, language, content, updated_at) VALUES (?, ?, ?, ?, datetime('now', 'localtime')) ON CONFLICT(project_id, field_name, language) DO UPDATE SET content = excluded.content, updated_at = excluded.updated_at",
                                    rusqlite::params![project_id, field_name, lang, translated],
                                );
                            }
                        }
                        success = true;
                        break;
                    } else {
                        last_error = "Empty translation result".to_string();
                    }
                }
                Err(e) => {
                    last_error = e;
                    log::error!("[TRANSLATE_AI] {} attempt {} error: {}", field_name, attempt + 1, last_error);
                }
            }
        }

        let _ = app.emit("import:task_progress", TaskProgress {
            project_id,
            task_type: field_name.to_string(),
            status: if success { "done".to_string() } else { "failed".to_string() },
            error: if success { None } else { Some(last_error) },
        });
    }
}
