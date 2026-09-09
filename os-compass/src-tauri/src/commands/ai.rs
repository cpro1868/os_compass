use crate::db::DATABASE;
use crate::health::{calculate_health_score, health_score_to_rating, ProjectMetrics};
use crate::llm::{build_system_prompt, LlmClient, LlmMessage};
use crate::settings::AppSettings;
use serde::{Deserialize, Serialize};
use std::io::{Write, stderr};
use log;

#[derive(Debug, Serialize)]
pub struct AiAnalysisResult {
    pub summary: Option<String>,
    #[serde(rename = "useCases")]
    pub use_cases: Option<String>,
    pub risks: Option<String>,
    pub dependencies: Option<String>,
    pub health_score: Option<f64>,
    pub health_rating: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SettingsInput {
    #[serde(rename = "data_dir")]
    pub data_dir: Option<String>,
    pub theme: Option<String>,
    #[serde(rename = "default_language")]
    pub default_language: Option<String>,
    #[serde(rename = "auto_translate_readme")]
    pub auto_translate_readme: Option<bool>,
    #[serde(rename = "auto_translate_report")]
    pub auto_translate_report: Option<bool>,
    #[serde(rename = "download_path")]
    pub download_path: Option<String>,
    #[serde(rename = "default_editor")]
    pub default_editor: Option<String>,
    #[serde(rename = "llm_provider")]
    pub llm_provider: Option<String>,
    #[serde(rename = "llm_api_base")]
    pub llm_api_base: Option<String>,
    #[serde(rename = "llm_api_key")]
    pub llm_api_key: Option<String>,
    #[serde(rename = "llm_model")]
    pub llm_model: Option<String>,
    #[serde(rename = "llm_proxy_enabled")]
    pub llm_proxy_enabled: Option<bool>,
    #[serde(rename = "llm_proxy_protocol")]
    pub llm_proxy_protocol: Option<String>,
    #[serde(rename = "llm_proxy_host")]
    pub llm_proxy_host: Option<String>,
    #[serde(rename = "llm_proxy_port")]
    pub llm_proxy_port: Option<i32>,
    #[serde(rename = "llm_proxy_username")]
    pub llm_proxy_username: Option<String>,
    #[serde(rename = "llm_proxy_password")]
    pub llm_proxy_password: Option<String>,
    #[serde(rename = "proxy_enabled")]
    pub proxy_enabled: Option<bool>,
    #[serde(rename = "proxy_protocol")]
    pub proxy_protocol: Option<String>,
    #[serde(rename = "proxy_host")]
    pub proxy_host: Option<String>,
    #[serde(rename = "proxy_port")]
    pub proxy_port: Option<i32>,
    #[serde(rename = "proxy_username")]
    pub proxy_username: Option<String>,
    #[serde(rename = "proxy_password")]
    pub proxy_password: Option<String>,
    #[serde(rename = "crawler_enabled")]
    pub crawler_enabled: Option<bool>,
    #[serde(rename = "crawler_api_url")]
    pub crawler_api_url: Option<String>,
    #[serde(rename = "readme_update_frequency")]
    pub readme_update_frequency: Option<String>,
    #[serde(rename = "translate_engine")]
    pub translate_engine: Option<String>,
    #[serde(rename = "google_api_key")]
    pub google_api_key: Option<String>,
    #[serde(rename = "google_proxy_enabled")]
    pub google_proxy_enabled: Option<bool>,
    #[serde(rename = "google_proxy_protocol")]
    pub google_proxy_protocol: Option<String>,
    #[serde(rename = "google_proxy_host")]
    pub google_proxy_host: Option<String>,
    #[serde(rename = "google_proxy_port")]
    pub google_proxy_port: Option<i32>,
    #[serde(rename = "google_proxy_username")]
    pub google_proxy_username: Option<String>,
    #[serde(rename = "google_proxy_password")]
    pub google_proxy_password: Option<String>,
}

#[tauri::command]
pub fn get_settings() -> Result<AppSettings, String> {
    Ok(crate::settings::get_settings())
}

#[tauri::command]
pub fn save_settings(settings: AppSettings) -> Result<(), String> {
    crate::settings::save_settings(&settings)
}

#[tauri::command]
pub async fn analyze_project(id: i64) -> AiAnalysisResult {
    let project_data = {
        let db = match DATABASE.lock() {
            Ok(db) => db,
            Err(e) => return AiAnalysisResult {
                summary: None,
                use_cases: None,
                risks: None,
                dependencies: None,
                health_score: None,
                health_rating: None,
                error: Some(e.to_string()),
            },
        };
        
        let db = match db.as_ref() {
            Some(db) => db,
            None => return AiAnalysisResult {
                summary: None,
                use_cases: None,
                risks: None,
                dependencies: None,
                health_score: None,
                health_rating: None,
                error: Some("Database not initialized".to_string()),
            },
        };
        
        let conn = db.get_connection();
        
        let metrics: ProjectMetrics = match conn.query_row(
            "SELECT stars, forks, open_issues, readme_content, license, homepage FROM projects WHERE id = ?",
            [id],
            |row| {
                Ok(ProjectMetrics {
                    stars: row.get(0)?,
                    forks: row.get(1)?,
                    open_issues: row.get(2)?,
                    has_readme: row.get::<_, Option<String>>(3)?.is_some(),
                    has_license: row.get::<_, Option<String>>(4)?.is_some(),
                    has_homepage: row.get::<_, Option<String>>(5)?.is_some(),
                    primary_language: None,
                })
            },
        ) {
            Ok(m) => m,
            Err(_) => return AiAnalysisResult {
                summary: None,
                use_cases: None,
                risks: None,
                dependencies: None,
                health_score: None,
                health_rating: None,
                error: Some("Project not found".to_string()),
            },
        };

        let (readme_content, project_name): (Option<String>, String) = conn.query_row(
            "SELECT readme_content, name FROM projects WHERE id = ?",
            [id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap_or((None, "Unknown".to_string()));

        let health = calculate_health_score(metrics.clone());
        let health_rating = health_score_to_rating(health.overall);

        (metrics, readme_content, project_name, health, health_rating.to_string())
    };

    let llm = match LlmClient::from_settings() {
        Some(llm) => llm,
        None => {
            log::error!("[analyze_project] LLM not configured");
            return AiAnalysisResult {
                summary: None,
                use_cases: None,
                risks: None,
                dependencies: None,
                health_score: Some(project_data.3.overall),
                health_rating: Some(project_data.4.clone()),
                error: Some("LLM not configured".to_string()),
            }
        }
    };

    let readme_len = project_data.1.as_ref().map(|r| r.len()).unwrap_or(0);
    log::debug!("[analyze_project] Processing README ({} chars) for project '{}'...", readme_len, project_data.2);

    let system_msg = build_system_prompt();

    let readme_snippet = project_data.1.as_ref()
        .map(|r| crate::llm::preprocess_readme_for_analysis(r))
        .unwrap_or_else(|| "No README available".to_string());

    log::debug!("[analyze_project] Prompt prepared, sending to LLM (prompt length: {})...", readme_snippet.len());

    let user_msg = LlmMessage {
        role: "user".to_string(),
        content: format!(
            "Project: {}\n\nREADME Content:\n{}\n\nPlease analyze this open source project based on its README and provide a comprehensive JSON report.",
            project_data.2,
            readme_snippet
        ),
    };

    let log_to_file = |msg: &str| {
        let log_msg = format!("[{}] {}", chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f"), msg);
        log::debug!("{}", log_msg);
        
        if let Some(base_dirs) = directories::BaseDirs::new() {
            let app_data_dir = base_dirs.data_dir();
            let os_compass_dir = app_data_dir.join(".os-compass");
            let log_dir = os_compass_dir.join("logs");
            let _ = std::fs::create_dir_all(&log_dir);
            let log_file = log_dir.join("ai_analyze.log");
            if let Ok(mut file) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&log_file)
            {
                let _ = writeln!(file, "{}", log_msg);
                let _ = file.flush();
            }
        }
    };

    log_to_file(&format!("[analyze_project] Waiting for LLM response (timeout: 180s)..."));
    let parse_start = std::time::Instant::now();
    let (summary, use_cases, risks, dependencies) = match llm.chat(vec![system_msg, user_msg]).await {
        Ok(response) => {
            let response_len = response.len();
            log_to_file(&format!("[AI_ANALYZE] LLM response received: {} chars, time: {:?}", response_len, parse_start.elapsed()));
            
            let raw_preview: String = response.chars().take(800).collect();
            log_to_file(&format!("[AI_ANALYZE] Raw response (first 800 chars):\n{}", raw_preview));
            log_to_file(&format!("[AI_ANALYZE] About to extract JSON block..."));
            
            let step1_start = std::time::Instant::now();
            let mut json_str = response.trim().to_string();
            log_to_file(&format!("[AI_ANALYZE] json_str created, len={}", json_str.len()));

            let code_block_start = json_str.find("```json");
            let code_block_tick = json_str.find("```");
            log_to_file(&format!("[AI_ANALYZE] find ```json: {:?}, find ```: {:?}", code_block_start, code_block_tick));

            if let Some(start) = code_block_start {
                let after_start = start + 7;
                if let Some(rel_end) = json_str[after_start..].find("```") {
                    let end = after_start + rel_end;
                    if end > after_start {
                        json_str = json_str[after_start..end].trim().to_string();
                        log_to_file(&format!("[AI_ANALYZE] Step 1a: Extract ```json block, extracted {} chars, time: {:?}", json_str.len(), step1_start.elapsed()));
                    } else {
                        log_to_file(&format!("[AI_ANALYZE] Step 1a: Invalid range after_start={}, end={}", after_start, end));
                    }
                }
            } else if let Some(start) = json_str.find("```") {
                let after_start = start + 3;
                if let Some(rel_end) = json_str[after_start..].find("```") {
                    let end = after_start + rel_end;
                    if end > after_start {
                        json_str = json_str[after_start..end].trim().to_string();
                        log_to_file(&format!("[AI_ANALYZE] Step 1b: Extract ``` block, extracted {} chars, time: {:?}", json_str.len(), step1_start.elapsed()));
                    } else {
                        log_to_file(&format!("[AI_ANALYZE] Step 1b: Invalid range after_start={}, end={}", after_start, end));
                    }
                }
            } else {
                log_to_file(&format!("[AI_ANALYZE] Step 1c: No code block found, time: {:?}", step1_start.elapsed()));
            }

            if !json_str.starts_with('{') {
                if let Some(start) = json_str.find('{') {
                    if let Some(end) = json_str.rfind('}') {
                        json_str = json_str[start..=end].to_string();
                        log_to_file(&format!("[AI_ANALYZE] Step 2: Extract JSON from text, time: {:?}", step1_start.elapsed()));
                    }
                }
            } else {
                log_to_file(&format!("[AI_ANALYZE] Step 2: JSON already starts with {{, time: {:?}", step1_start.elapsed()));
            }

            log_to_file(&format!("[AI_ANALYZE] JSON string to parse (first 500 chars):\n{}", json_str.chars().take(500).collect::<String>()));
            log_to_file(&format!("[AI_ANALYZE] JSON string length: {} chars, time: {:?}", json_str.len(), step1_start.elapsed()));

            #[derive(serde::Deserialize)]
            struct LlmResponse {
                summary: Option<String>,
                #[serde(rename = "use_cases", default)]
                use_cases: Option<serde_json::Value>,
                #[serde(rename = "useCases", default)]
                use_cases_alt: Option<serde_json::Value>,
                #[serde(default)]
                risks: Option<serde_json::Value>,
                #[serde(default)]
                dependencies: Option<serde_json::Value>,
            }

            fn value_to_string(v: Option<serde_json::Value>) -> Option<String> {
                v.and_then(|val| match val {
                    serde_json::Value::String(s) => Some(s),
                    serde_json::Value::Array(arr) => {
                        let items: Vec<String> = arr.iter().filter_map(|i| i.as_str().map(|s| s.to_string())).collect();
                        if items.is_empty() { None } else { Some(items.join(", ")) }
                    },
                    serde_json::Value::Null => None,
                    other => Some(other.to_string()),
                })
            }

            let step3_start = std::time::Instant::now();
            log_to_file(&format!("[AI_ANALYZE] >>> About to call serde_json::from_str, json_len={}", json_str.len()));
            match serde_json::from_str::<LlmResponse>(&json_str) {
                Ok(parsed) => {
                    log_to_file(&format!("[AI_ANALYZE] Step 3: JSON parse OK, time: {:?}, summary_len= {:?}", 
                        step3_start.elapsed(),
                        parsed.summary.as_ref().map(|s| s.len())));
                    let use_cases = value_to_string(parsed.use_cases.clone()).or(value_to_string(parsed.use_cases_alt.clone()));
                    let risks = value_to_string(parsed.risks);
                    let dependencies = value_to_string(parsed.dependencies);
                    log_to_file(&format!("[AI_ANALYZE] Parsing complete, total time: {:?}", parse_start.elapsed()));
                    (parsed.summary, use_cases, risks, dependencies)
                },
                Err(e) => {
                    log_to_file(&format!("[AI_ANALYZE] Step 3: JSON parse FAILED: {}, time: {:?}", e, step3_start.elapsed()));
                    let mut summary: Option<String> = None;
                    let mut use_cases: Option<String> = None;
                    let mut risks: Option<String> = None;
                    let mut dependencies: Option<String> = None;

                    let step4_start = std::time::Instant::now();
                    let extract_field = |json: &str, field_names: &[&str], field_label: &str| -> Option<String> {
                        let field_start = std::time::Instant::now();
                        for &field in field_names {
                            let patterns = [
                                format!("\"{}\":\"", field),
                                format!("\"{}\": \"", field),
                                format!("'{}':'", field),
                                format!("'{}': '", field),
                                format!("{}:", field),
                            ];
                            for pattern in &patterns {
                                if let Some(pos) = json.find(pattern) {
                                    let value_start = pos + pattern.len();
                                    let Some(rest) = json.get(value_start..) else { continue };
                                    if rest.starts_with('[') {
                                        if let Some(end) = rest.find(']') {
                                            let arr_str = &rest[1..end];
                                            let items: Vec<String> = arr_str.split(',')
                                                .map(|s| s.trim().trim_matches('"').trim_matches('\'').to_string())
                                                .filter(|s| !s.is_empty())
                                                .collect();
                                            if !items.is_empty() {
                                                log_to_file(&format!("[AI_ANALYZE] Extract field '{}' ({} patterns): found array with {} items, time: {:?}", 
                                                    field_label, patterns.len(), items.len(), field_start.elapsed()));
                                                return Some(items.join(", "));
                                            }
                                        }
                                    } else if let Some(end) = rest.find('"') {
                                        let val = &rest[..end];
                                        if !val.contains('{') && val.len() > 2 && val.len() < 500 {
                                            log_to_file(&format!("[AI_ANALYZE] Extract field '{}' ({} patterns): found string, len={}, time: {:?}", 
                                                field_label, patterns.len(), val.len(), field_start.elapsed()));
                                            return Some(clean_json_value(val));
                                        }
                                    }
                                }
                            }
                        }
                        log_to_file(&format!("[AI_ANALYZE] Extract field '{}': NOT FOUND, time: {:?}", field_label, field_start.elapsed()));
                        None
                    };

                    summary = extract_field(&json_str, &["summary", "一句话总结"], "summary");
                    let after_summary = std::time::Instant::now();
                    log_to_file(&format!("[AI_ANALYZE] After summary extraction, time: {:?}", after_summary.elapsed()));
                    
                    use_cases = extract_field(&json_str, &["use_cases", "useCases", "适用场景"], "use_cases");
                    risks = extract_field(&json_str, &["risks", "风险"], "risks");
                    dependencies = extract_field(&json_str, &["dependencies", "deps", "依赖"], "dependencies");
                    let after_all_fields = std::time::Instant::now();
                    log_to_file(&format!("[AI_ANALYZE] After all field extractions, time: {:?}", after_all_fields.elapsed()));

                    if summary.is_none() {
                        let lines_start = std::time::Instant::now();
                        let lines: Vec<&str> = response.lines()
                            .filter(|l| !l.trim().starts_with('{') && !l.trim().starts_with('}') && !l.trim().starts_with("```"))
                            .collect();
                        log_to_file(&format!("[AI_ANALYZE] Lines filtering: {} lines, time: {:?}", lines.len(), lines_start.elapsed()));

                        if !lines.is_empty() {
                            let first_para = lines.join(" ").trim().to_string();
                            if first_para.len() > 20 {
                                summary = Some(clean_text(&first_para));
                                log_to_file(&format!("[AI_ANALYZE] Fallback summary extracted, len={}, time: {:?}", first_para.len(), lines_start.elapsed()));
                            }
                        }
                    }

                    log_to_file(&format!("[AI_ANALYZE] Step 4 (fallback parse): summary={:?}, use_cases={:?}, risks={:?}, deps={:?}, total_time: {:?}",
                        summary.as_ref().map(|s| s.len()),
                        use_cases.as_ref().map(|s| s.len()),
                        risks.as_ref().map(|s| s.len()),
                        dependencies.as_ref().map(|s| s.len()),
                        parse_start.elapsed()));

                    (summary, use_cases, risks, dependencies)
                }
            }
        },
        Err(e) => {
            log_to_file(&format!("[AI_ANALYZE] LLM chat FAILED: {}", e));
            return AiAnalysisResult {
                summary: None,
                use_cases: None,
                risks: None,
                dependencies: None,
                health_score: Some(project_data.3.overall),
                health_rating: Some(project_data.4.clone()),
                error: Some(e.to_string()),
            };
        }
    };

    {
        let db = match DATABASE.lock() {
            Ok(db) => db,
            Err(e) => {
                log::error!("Failed to acquire lock for saving: {}", e);
                return AiAnalysisResult {
                    summary,
                    use_cases,
                    risks,
                    dependencies,
                    health_score: Some(project_data.3.overall),
                    health_rating: Some(project_data.4.clone()),
                    error: Some("Failed to save: lock error".to_string()),
                };
            },
        };
        
        if let Some(db) = db.as_ref() {
            let conn = db.get_connection();
            if let Err(e) = conn.execute(
                "UPDATE projects SET ai_summary = ?, ai_use_cases = ?, ai_risks = ?, ai_dependencies = ?, health_score = ?, updated_at = datetime('now', 'localtime') WHERE id = ?",
                rusqlite::params![summary, use_cases, risks, dependencies, project_data.3.overall, id],
            ) {
                log::error!("Failed to save AI analysis: {}", e);
            }
        }
    }

    AiAnalysisResult {
        summary,
        use_cases,
        risks,
        dependencies,
        health_score: Some(project_data.3.overall),
        health_rating: Some(project_data.4),
        error: None,
    }
}

fn clean_json_value(s: &str) -> String {
    s.replace("\\n", " ").replace("\\\"", "\"").replace("\\", "").trim().to_string()
}

fn clean_text(s: &str) -> String {
    s.replace("\\n", "\n").replace("\\\"", "\"").replace("\\", "").trim().to_string()
}

fn save_ai_analysis(id: i64, summary: &Option<String>, use_cases: &Option<String>, risks: &Option<String>, dependencies: &Option<String>, health_score: f64) -> Result<(), String> {
    let db = DATABASE.lock().map_err(|e| e.to_string())?;
    let db = db.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();

    conn.execute(
        "UPDATE projects SET ai_summary = ?, ai_use_cases = ?, ai_risks = ?, ai_dependencies = ?, health_score = ?, updated_at = datetime('now', 'localtime') WHERE id = ?",
        rusqlite::params![summary, use_cases, risks, dependencies, health_score, id],
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub fn calculate_project_health(id: i64) -> Result<(f64, String), String> {
    let db = DATABASE.lock().map_err(|e| e.to_string())?;
    let db = db.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();

    let metrics: ProjectMetrics = conn.query_row(
        "SELECT stars, forks, open_issues, readme_content, license, homepage FROM projects WHERE id = ?",
        [id],
        |row| {
            Ok(ProjectMetrics {
                stars: row.get(0)?,
                forks: row.get(1)?,
                open_issues: row.get(2)?,
                has_readme: row.get::<_, Option<String>>(3)?.is_some(),
                has_license: row.get::<_, Option<String>>(4)?.is_some(),
                has_homepage: row.get::<_, Option<String>>(5)?.is_some(),
                primary_language: None,
            })
        },
    )
    .map_err(|e| e.to_string())?;

    let health = calculate_health_score(metrics);
    let rating = health_score_to_rating(health.overall);
    Ok((health.overall, rating.to_string()))
}

#[tauri::command]
pub async fn get_llm_models(api_base: String, api_key: String) -> Result<Vec<String>, String> {
    crate::llm::list_models(&api_base, &api_key).await
}

#[tauri::command]
pub async fn generate_tags(id: i64) -> Result<String, String> {
    let (project_name, description, languages, readme_content) = {
        let db = DATABASE.lock().map_err(|e| e.to_string())?;
        let db = db.as_ref().ok_or("Database not initialized")?;
        let conn = db.get_connection();

        conn.query_row(
            "SELECT name, description, languages, readme_content FROM projects WHERE id = ?",
            [id],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?, row.get::<_, Option<String>>(2)?, row.get::<_, Option<String>>(3)?)),
        )
        .map_err(|e| e.to_string())?
    };

    let llm = LlmClient::from_settings()
        .ok_or_else(|| "LLM not configured".to_string())?;

    let prompt = format!(
        r#"Based on the following project information, suggest 3-5 relevant tags for categorization.

Project Name: {}
Description: {}
Languages: {}
README excerpt: {}

Please respond with only a JSON array of tag names in Chinese, like: ["标签1", "标签2", "标签3"]"#,
        project_name,
        description.as_deref().unwrap_or("N/A"),
        languages.as_deref().unwrap_or("N/A"),
        readme_content.as_deref().map(|r| crate::llm::preprocess_readme_for_tags(r)).unwrap_or("N/A".to_string())
    );

    let messages = vec![
        LlmMessage {
            role: "system".to_string(),
            content: "You are an expert at categorizing open source projects. Return only a JSON array of 3-5 relevant Chinese tag names.".to_string(),
        },
        LlmMessage {
            role: "user".to_string(),
            content: prompt,
        },
    ];

    let response = llm.chat(messages).await?;

    let tags_str = response.trim();
    let tags_str = if tags_str.starts_with("```json") {
        tags_str.trim_start_matches("```json").trim_end_matches("```").trim()
    } else if tags_str.starts_with("```") {
        tags_str.trim_start_matches("```").trim_end_matches("```").trim()
    } else {
        tags_str
    };

    #[derive(serde::Deserialize)]
    struct TagList {
        #[serde(flatten)]
        _extra: std::collections::HashMap<String, serde_json::Value>,
    }

    let tags: Vec<String> = serde_json::from_str(tags_str)
        .unwrap_or_else(|_| vec![project_name.clone()]);

    let db = DATABASE.lock().map_err(|e| e.to_string())?;
    let db = db.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();

    for tag_name in &tags {
        let tag_name = tag_name.trim();
        if tag_name.is_empty() {
            continue;
        }

        conn.execute(
            "INSERT OR IGNORE INTO tags (name) VALUES (?)",
            [tag_name],
        )
        .map_err(|e| e.to_string())?;

        let tag_id: i64 = conn
            .query_row(
                "SELECT id FROM tags WHERE name = ?",
                [tag_name],
                |row| row.get(0),
            )
            .map_err(|e| e.to_string())?;

        conn.execute(
            "INSERT OR IGNORE INTO project_tags (project_id, tag_id) VALUES (?, ?)",
            rusqlite::params![id, tag_id],
        )
        .map_err(|e| e.to_string())?;
    }

    Ok(tags.join(", "))
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Translations {
    pub translated_summary: Option<String>,
    pub translated_use_cases: Option<String>,
    pub translated_risks: Option<String>,
    pub translated_dependencies: Option<String>,
    pub translated_description: Option<String>,
    pub readme_translation: Option<String>,
}

#[tauri::command]
pub fn save_translations(
    id: i64,
    language: String,
    translated_summary: Option<String>,
    translated_use_cases: Option<String>,
    translated_risks: Option<String>,
    translated_dependencies: Option<String>,
    translated_description: Option<String>,
) -> Result<String, String> {
    let db = DATABASE.lock().map_err(|e| e.to_string())?;
    let db = db.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();

    conn.execute(
        "CREATE TABLE IF NOT EXISTS translations (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            project_id INTEGER NOT NULL,
            field_name TEXT NOT NULL,
            language TEXT NOT NULL,
            content TEXT NOT NULL,
            created_at TEXT DEFAULT (datetime('now', 'localtime')),
            updated_at TEXT DEFAULT (datetime('now', 'localtime')),
            UNIQUE(project_id, field_name, language)
        )",
        [],
    ).map_err(|e| e.to_string())?;

    let fields = [
        ("summary", translated_summary),
        ("use_cases", translated_use_cases),
        ("risks", translated_risks),
        ("dependencies", translated_dependencies),
        ("description", translated_description),
    ];

    let mut count = 0;
    for (field_name, content) in fields {
        if let Some(text) = content {
            let affected = conn.execute(
                "INSERT INTO translations (project_id, field_name, language, content, updated_at) VALUES (?, ?, ?, ?, datetime('now', 'localtime')) ON CONFLICT(project_id, field_name, language) DO UPDATE SET content = excluded.content, updated_at = excluded.updated_at",
                rusqlite::params![id, field_name, language, text],
            ).map_err(|e| format!("保存翻译失败: {}", e))?;
            count += affected;
        }
    }

    let total_count: i64 = conn.query_row("SELECT COUNT(*) FROM translations WHERE project_id = ?", [id], |row| row.get(0)).unwrap_or(0);
    Ok(format!("插入 {} 条，当前项目共 {} 条翻译", count, total_count))
}

#[tauri::command]
pub fn debug_translations_table() -> Result<String, String> {
    let db = DATABASE.lock().map_err(|e| e.to_string())?;
    let db = db.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();

    let mut stmt = conn.prepare("SELECT id, project_id, field_name, language, length(content), created_at FROM translations LIMIT 10").map_err(|e| e.to_string())?;
    let rows = stmt.query_map([], |row| {
        Ok(format!("id={}, project_id={}, field={}, lang={}, content_len={}, created={}",
            row.get::<_, i64>(0)?,
            row.get::<_, i64>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, i64>(4)?,
            row.get::<_, String>(5)?
        ))
    }).map_err(|e| e.to_string())?;

    let mut result = String::new();
    for row in rows {
        result.push_str(&row.map_err(|e| e.to_string())?);
        result.push('\n');
    }
    Ok(result)
}

#[tauri::command]
pub fn debug_translations(id: i64) -> Result<String, String> {
    let db = DATABASE.lock().map_err(|e| e.to_string())?;
    let db = db.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();

    let result: Result<(Option<String>, Option<String>, Option<String>, Option<String>, Option<String>), _> = conn.query_row(
        "SELECT translated_summary, translated_use_cases, translated_risks, translated_dependencies, translated_description FROM projects WHERE id = ?",
        [id],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
    );

    match result {
        Ok((s, u, r, d, desc)) => {
            log::debug!("[DEBUG] Project {} translations: summary={:?}, use_cases={:?}, risks={:?}, deps={:?}, desc={:?}",
                id, s.as_ref().map(|x| x.len()), u.as_ref().map(|x| x.len()), r.as_ref().map(|x| x.len()), d.as_ref().map(|x| x.len()), desc.as_ref().map(|x| x.len()));
            Ok(format!("translated_summary={:?}, translated_use_cases={:?}, translated_risks={:?}, translated_dependencies={:?}, translated_description={:?}", s, u, r, d, desc))
        },
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
pub async fn test_translate(text: String, target_lang: String) -> Result<String, String> {
    log::debug!("[TEST_TRANSLATE] text_len={}, target_lang={}", text.len(), target_lang);
    let result = crate::translate::translate_text(&text, &target_lang).await?;
    log::debug!("[TEST_TRANSLATE] result_len={}", result.len());
    Ok(result)
}

#[tauri::command]
pub fn test_readme(id: i64) -> Result<String, String> {
    let db = DATABASE.lock().map_err(|e| e.to_string())?;
    let db = db.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();

    let result: Result<(Option<String>, Option<String>, Option<String>), _> = conn.query_row(
        "SELECT readme_content, readme_translation, description FROM projects WHERE id = ?",
        [id],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    );

    match result {
        Ok((readme, trans, desc)) => {
            Ok(format!("readme_len={:?}, readme_trans_len={:?}, desc_len={:?}", 
                readme.as_ref().map(|s| s.len()),
                trans.as_ref().map(|s| s.len()),
                desc.as_ref().map(|s| s.len())
            ))
        },
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
pub fn get_translations(id: i64) -> Result<Translations, String> {
    log::debug!("Loading translations for project {}", id);
    let db = DATABASE.lock().map_err(|e| e.to_string())?;
    let db = db.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();

    let translations = conn
        .query_row(
            "SELECT translated_summary, translated_use_cases, translated_risks, translated_dependencies, translated_description, readme_translation FROM projects WHERE id = ?",
            [id],
            |row| {
                Ok(Translations {
                    translated_summary: row.get(0)?,
                    translated_use_cases: row.get(1)?,
                    translated_risks: row.get(2)?,
                    translated_dependencies: row.get(3)?,
                    translated_description: row.get(4)?,
                    readme_translation: row.get(5)?,
                })
            },
        )
        .map_err(|e| e.to_string())?;

    log::debug!("Loaded translations: {:?}", translations);
    Ok(translations)
}

#[tauri::command]
pub fn save_readme_translation(id: i64, translation: String) -> Result<(), String> {
    let db = DATABASE.lock().map_err(|e| e.to_string())?;
    let db = db.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();

    conn.execute(
        "UPDATE projects SET readme_translation = ?, updated_at = datetime('now', 'localtime') WHERE id = ?",
        rusqlite::params![translation, id],
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub fn clear_translations(id: i64) -> Result<(), String> {
    let db = DATABASE.lock().map_err(|e| e.to_string())?;
    let db = db.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();

    conn.execute(
        "UPDATE projects SET translated_summary = NULL, translated_use_cases = NULL, translated_risks = NULL, translated_dependencies = NULL, translated_description = NULL, updated_at = datetime('now', 'localtime') WHERE id = ?",
        [id],
    )
    .map_err(|e| e.to_string())?;

    conn.execute(
        "DELETE FROM translations WHERE project_id = ?",
        [id],
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub fn test_llm_connection(provider: String, api_base: String, api_key: String, model: String) -> Result<(), String> {
    if api_key.is_empty() {
        return Err("API Key 不能为空".to_string());
    }
    if api_base.is_empty() {
        return Err("API Base URL 不能为空".to_string());
    }
    if model.is_empty() {
        return Err("模型名称不能为空".to_string());
    }

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| e.to_string())?;
    
    let url = format!("{}/chat/completions", api_base.trim_end_matches('/'));

    let body = serde_json::json!({
        "model": model,
        "messages": [{"role": "user", "content": "Hi"}],
        "max_tokens": 5
    });

    let response = client
        .post(&url)
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&body)
        .send()
        .map_err(|e| format!("连接失败: {}", e))?;

    if response.status().is_success() {
        Ok(())
    } else {
        Err(format!("API 返回错误: {}", response.status()))
    }
}

#[tauri::command]
pub fn list_available_models(provider: String, api_base: String, api_key: String) -> Result<Vec<String>, String> {
    if api_key.is_empty() {
        return Err("API Key 不能为空".to_string());
    }
    if api_base.is_empty() {
        return Err("API Base URL 不能为空".to_string());
    }

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| e.to_string())?;

    let url = format!("{}/models", api_base.trim_end_matches('/'));

    let response = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .send()
        .map_err(|e| format!("请求失败: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("API 返回错误: {}", response.status()));
    }

    let text = response.text().map_err(|e| format!("读取响应失败: {}", e))?;

    // 写入日志文件便于排查
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let log_dir = dir.join("logs");
            std::fs::create_dir_all(&log_dir).ok();
            let log_path = log_dir.join("models.log");
            let log_entry = format!(
                "===== {} =====\nURL: {}\nLength: {}\nBody:\n{}\n\n",
                chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
                url,
                text.len(),
                &text[..text.len().min(5000)]
            );
            std::fs::write(&log_path, log_entry).ok();
        }
    }

    let json: serde_json::Value = serde_json::from_str(&text)
        .map_err(|e| format!("解析JSON失败: {}", e))?;

    let mut models: Vec<String> = Vec::new();

    // OpenAI 兼容 API 标准格式：{"data": [{"id": "model-name"}, ...]}
    if let Some(data) = json["data"].as_array() {
        for m in data {
            if let Some(id) = m["id"].as_str() {
                models.push(id.to_string());
            }
        }
    }

    // Ollama 格式：{"models": [{"name": "llama3"}, ...]}
    if models.is_empty() {
        if let Some(arr) = json["models"].as_array() {
            for m in arr {
                if let Some(name) = m["name"].as_str() {
                    models.push(name.to_string());
                } else if let Some(id) = m["id"].as_str() {
                    models.push(id.to_string());
                }
            }
        }
    }

    if models.is_empty() {
        return Err(format!("未找到模型列表。响应前500字符: {}", &text[..text.len().min(500)]));
    }

    Ok(models)
}

#[derive(Serialize)]
pub struct LlmDebugInfo {
    pub llm_configured: bool,
    pub api_key_len: usize,
    pub api_key_preview: String,
    pub api_base: String,
    pub model: String,
    pub proxy_enabled: bool,
    pub error: Option<String>,
}

#[tauri::command]
pub fn debug_llm_status() -> LlmDebugInfo {
    log::debug!("[debug_llm_status] Called");
    let s = crate::settings::get_settings();

    let api_key_preview = if s.llm_api_key.is_empty() {
        String::new()
    } else {
        s.llm_api_key.chars().take(5).collect::<String>() + "..."
    };

    LlmDebugInfo {
        llm_configured: !s.llm_api_key.is_empty(),
        api_key_len: s.llm_api_key.len(),
        api_key_preview,
        api_base: s.llm_api_base.clone(),
        model: s.llm_model.clone(),
        proxy_enabled: s.llm_proxy_enabled,
        error: None,
    }
}

#[derive(Serialize)]
pub struct TestLlmResult {
    pub success: bool,
    pub elapsed_ms: u64,
    pub response_preview: String,
    pub error: Option<String>,
}

#[tauri::command]
pub async fn test_llm_direct(project_id: i64) -> TestLlmResult {
    use std::time::Instant;

    log::debug!("[test_llm_direct] Testing LLM for project_id={}", project_id);

    let start = Instant::now();

    let project_data = {
        let db = match DATABASE.lock() {
            Ok(db) => db,
            Err(e) => return TestLlmResult {
                success: false,
                elapsed_ms: start.elapsed().as_millis() as u64,
                response_preview: String::new(),
                error: Some(format!("DB lock error: {}", e)),
            },
        };

        let db = match db.as_ref() {
            Some(db) => db,
            None => return TestLlmResult {
                success: false,
                elapsed_ms: start.elapsed().as_millis() as u64,
                response_preview: String::new(),
                error: Some("Database not initialized".to_string()),
            },
        };

        let conn = db.get_connection();

        let (readme_content, name): (Option<String>, String) = conn.query_row(
            "SELECT readme_content, name FROM projects WHERE id = ?",
            [project_id],
            |row| Ok((row.get::<_, Option<String>>(0)?, row.get::<_, String>(1)?)),
        ).unwrap_or((None, "Unknown".to_string()));

        drop(conn);
        drop(db);
        drop(db);

        (readme_content, name)
    };

    log::debug!("[test_llm_direct] Project loaded: {}, readme len={}", project_data.1, project_data.0.as_ref().map(|s| s.len()).unwrap_or(0));

    let llm = match crate::llm::LlmClient::from_settings() {
        Some(llm) => llm,
        None => return TestLlmResult {
            success: false,
            elapsed_ms: start.elapsed().as_millis() as u64,
            response_preview: String::new(),
            error: Some("LLM not configured".to_string()),
        },
    };

    log::debug!("[test_llm_direct] LLM client created");

    let system_msg = crate::llm::build_system_prompt();
    let readme_snippet = project_data.0.as_ref()
        .map(|r| crate::llm::preprocess_readme_for_analysis(r))
        .unwrap_or_else(|| "No README available".to_string());

    let user_msg = crate::llm::LlmMessage {
        role: "user".to_string(),
        content: format!("Project: {}\n\nREADME Content:\n{}\n\nPlease analyze this project briefly in one sentence.", project_data.1, readme_snippet),
    };

    log::debug!("[test_llm_direct] Calling LLM...");

    match llm.chat(vec![system_msg, user_msg]).await {
        Ok(response) => {
            let elapsed = start.elapsed().as_millis() as u64;
            log::info!("[test_llm_direct] Success after {}ms", elapsed);
            TestLlmResult {
                success: true,
                elapsed_ms: elapsed,
                response_preview: response.chars().take(500).collect(),
                error: None,
            }
        }
        Err(e) => {
            let elapsed = start.elapsed().as_millis() as u64;
            log::error!("[test_llm_direct] Failed after {}ms: {}", elapsed, e);
            TestLlmResult {
                success: false,
                elapsed_ms: elapsed,
                response_preview: String::new(),
                error: Some(e),
            }
        }
    }
}
