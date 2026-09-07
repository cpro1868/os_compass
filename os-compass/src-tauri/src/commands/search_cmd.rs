use crate::db::DATABASE;
use crate::embedding::{EmbeddingConfig, update_config as update_embedding_config};
use crate::plugins::search::{
    three_layer_search, SearchResult, ProjectMatch, IntentAnalysis, analyze_intent,
};
use crate::settings::AppSettings;
use crate::vault::CURRENT_VAULT_CONFIG;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use tauri::command;

pub use crate::embedding::EmbeddingConfig as EmbeddingSettings;

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchHistoryItem {
    pub id: i64,
    pub query: String,
    pub result_count: i64,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchSource {
    pub id: i64,
    pub name: String,
    pub source_type: String,
    pub url: String,
    pub platform: Option<String>,
    pub enabled: bool,
}

fn resolve_vault_dir(path: &str) -> Result<std::path::PathBuf, String> {
    if path.trim().is_empty() {
        return Err("Vault not initialized".to_string());
    }

    let path = std::path::Path::new(path);
    if path.file_name().and_then(|name| name.to_str()) == Some("os_compass.db") {
        return path
            .parent()
            .map(std::path::Path::to_path_buf)
            .ok_or_else(|| "Invalid vault path".to_string());
    }

    Ok(path.to_path_buf())
}

fn get_search_conn() -> Result<rusqlite::Connection, String> {
    let config = CURRENT_VAULT_CONFIG.lock().unwrap();
    let path = config.as_ref().ok_or("No vault opened")?;
    let vault_dir = resolve_vault_dir(&path.path)?;
    crate::plugins::search::init_search_db(&vault_dir)
}

#[command]
pub async fn intent_search(query: String, _conversation_id: Option<String>) -> Result<SearchResult, String> {
    eprintln!("[DEBUG] intent_search: 开始处理查询 '{}'", query);
    let vault_dir = {
        let config = CURRENT_VAULT_CONFIG.lock().unwrap();
        eprintln!("[DEBUG] intent_search: 获取 vault config 成功");
        let path = config.as_ref().ok_or("No vault opened")?;
        eprintln!("[DEBUG] intent_search: vault path: {:?}", path.path);
        resolve_vault_dir(&path.path)?
    };

    eprintln!("[DEBUG] intent_search: 调用 three_layer_search...");
    let result = three_layer_search(&vault_dir, &query).await;
    eprintln!("[DEBUG] intent_search: three_layer_search 完成，结果: {:?}", result.is_ok());
    result
}

pub use crate::plugins::search::SearchContextMessage;

#[command]
pub async fn analyze_user_intent(
    user_input: String,
    history: Option<Vec<SearchContextMessage>>,
) -> Result<IntentAnalysis, String> {
    let hist = history.unwrap_or_default();
    crate::plugins::search::analyze_intent_with_history(&user_input, &hist).await
}

#[command]
pub fn get_search_history(limit: Option<i64>) -> Result<Vec<SearchHistoryItem>, String> {
    let conn = get_search_conn()?;
    let lim = limit.unwrap_or(20);

    let mut stmt = conn
        .prepare("SELECT id, query, result_count, created_at FROM search_history ORDER BY created_at DESC LIMIT ?")
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map(params![lim], |row| {
            Ok(SearchHistoryItem {
                id: row.get(0)?,
                query: row.get(1)?,
                result_count: row.get(2)?,
                created_at: row.get(3)?,
            })
        })
        .map_err(|e| e.to_string())?;

    let mut items = Vec::new();
    for row in rows {
        items.push(row.map_err(|e| e.to_string())?);
    }
    Ok(items)
}

#[command]
pub fn clear_search_history() -> Result<(), String> {
    let conn = get_search_conn()?;
    conn.execute("DELETE FROM search_history", [])
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[command]
pub fn delete_search_history_item(id: i64) -> Result<(), String> {
    let conn = get_search_conn()?;
    conn.execute("DELETE FROM search_history WHERE id = ?", params![id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RecommendMoreResult {
    pub items: Vec<ProjectMatch>,
    #[serde(default)]
    pub raw_text: Option<String>,
}

#[command]
pub async fn recommend_more_projects(query: String, limit: Option<usize>) -> Result<RecommendMoreResult, String> {
    let lim = limit.unwrap_or(5);

    crate::plugins::search::write_debug_log(&format!(
        "[DEBUG] recommend_more_projects: 开始 query='{}' limit={}",
        query, lim
    ));

    let settings = crate::settings::get_settings();
    crate::plugins::search::write_debug_log(&format!(
        "[DEBUG] recommend_more_projects: api_base='{}' model='{}' key_len={}",
        settings.llm_api_base, settings.llm_model, settings.llm_api_key.len()
    ));

    let prompt = format!(
        "你是开源项目专家。用户正在寻找关于「{}」的开源项目。\n\n请直接推荐 {} 个真实存在的优秀开源项目（优先 GitHub/Gitee 活跃项目），并用自然语言清晰列出：\n1. 项目名称与仓库地址（URL）\n2. 项目用途与核心特色（一两句话说明）\n3. 主要编程语言\n\n请直接输出推荐内容，排版清晰美观，不需要多余的问候语。",
        query, lim
    );

    let started = std::time::Instant::now();
    let direct_text = crate::source_engine::llm_parser::ask_llm_direct(&prompt, &settings).await;
    crate::plugins::search::write_debug_log(&format!(
        "[DEBUG] recommend_more_projects: ask_llm_direct 结束 (耗时 {:?})",
        started.elapsed()
    ));

    if let Ok(text) = direct_text {
        if !text.trim().is_empty() {
            crate::plugins::search::write_debug_log(&format!(
                "[DEBUG] recommend_more_projects: LLM 返回原文长度={}",
                text.len()
            ));
            return Ok(RecommendMoreResult {
                items: Vec::new(),
                raw_text: Some(text),
            });
        }
    }

    crate::plugins::search::write_debug_log(
        "[DEBUG] recommend_more_projects: 直推未返回，回落到本地向量并格式化为文字"
    );

    let mut local_text = String::new();
    if let Ok(vault_dir) = resolve_vault_dir(&settings_vault_path(&settings)) {
        let semantic = crate::embedding::semantic_search(&query, lim).await;
        if let Ok(matches) = semantic {
            let db_guard = crate::db::DATABASE.lock().unwrap();
            if let Some(db) = db_guard.as_ref() {
                let main_conn = db.get_connection();
                let mut lines: Vec<String> = Vec::new();
                for (idx, (project_id, _)) in matches.into_iter().enumerate() {
                    if let Ok((name, url, desc, lang)) = main_conn.query_row(
                        "SELECT name, url, description, languages FROM projects WHERE id = ?",
                        rusqlite::params![project_id],
                        |row| Ok((
                            row.get::<_, String>(0)?,
                            row.get::<_, Option<String>>(1)?.unwrap_or_default(),
                            row.get::<_, Option<String>>(2)?.unwrap_or_default(),
                            row.get::<_, Option<String>>(3)?,
                        )),
                    ) {
                        let l = crate::plugins::search::primary_language(lang).unwrap_or_else(|| "Unknown".to_string());
                        lines.push(format!("{}. **{}** ({})\n   - 仓库：{}\n   - 简介：{}", idx + 1, name, l, url, desc));
                    }
                }
                if !lines.is_empty() {
                    local_text = format!("本地库中与「{}」相关的项目：\n\n{}", query, lines.join("\n\n"));
                }
            }
        }
    }

    if local_text.is_empty() {
        return Ok(RecommendMoreResult {
            items: Vec::new(),
            raw_text: None,
        });
    }

    Ok(RecommendMoreResult {
        items: Vec::new(),
        raw_text: Some(local_text),
    })
}

fn settings_vault_path(_settings: &AppSettings) -> String {
    use crate::vault::CURRENT_VAULT_CONFIG;
    if let Ok(g) = CURRENT_VAULT_CONFIG.lock() {
        if let Some(c) = g.as_ref() {
            return c.path.clone();
        }
    }
    String::new()
}

#[command]
pub fn list_search_sources() -> Result<Vec<SearchSource>, String> {
    let conn = get_search_conn()?;
    let mut stmt = conn
        .prepare("SELECT id, name, source_type, url, platform, enabled FROM search_sources ORDER BY created_at DESC")
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map([], |row| {
            Ok(SearchSource {
                id: row.get(0)?,
                name: row.get(1)?,
                source_type: row.get(2)?,
                url: row.get(3)?,
                platform: row.get(4)?,
                enabled: row.get::<_, i32>(5)? == 1,
            })
        })
        .map_err(|e| e.to_string())?;

    let mut sources = Vec::new();
    for row in rows {
        sources.push(row.map_err(|e| e.to_string())?);
    }
    Ok(sources)
}

#[command]
pub fn add_search_source(
    name: String,
    source_type: String,
    url: String,
    platform: Option<String>,
) -> Result<SearchSource, String> {
    let conn = get_search_conn()?;

    conn.execute(
        "INSERT INTO search_sources (name, source_type, url, platform) VALUES (?, ?, ?, ?)",
        params![name, source_type, url, platform],
    ).map_err(|e| e.to_string())?;

    let id = conn.last_insert_rowid();

    Ok(SearchSource {
        id,
        name,
        source_type,
        url,
        platform,
        enabled: true,
    })
}

#[command]
pub fn update_search_source(
    id: i64,
    name: Option<String>,
    url: Option<String>,
    enabled: Option<bool>,
) -> Result<(), String> {
    let conn = get_search_conn()?;

    if let Some(n) = name {
        conn.execute("UPDATE search_sources SET name = ?, updated_at = datetime('now', 'localtime') WHERE id = ?", params![n, id])
            .map_err(|e| e.to_string())?;
    }
    if let Some(u) = url {
        conn.execute("UPDATE search_sources SET url = ?, updated_at = datetime('now', 'localtime') WHERE id = ?", params![u, id])
            .map_err(|e| e.to_string())?;
    }
    if let Some(e) = enabled {
        conn.execute("UPDATE search_sources SET enabled = ?, updated_at = datetime('now', 'localtime') WHERE id = ?", params![if e { 1 } else { 0 }, id])
            .map_err(|e| e.to_string())?;
    }

    Ok(())
}

#[command]
pub fn delete_search_source(id: i64) -> Result<(), String> {
    let conn = get_search_conn()?;
    conn.execute("DELETE FROM search_sources WHERE id = ?", params![id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[command]
pub fn refresh_search_cache(_source_id: Option<i64>) -> Result<i64, String> {
    Ok(0)
}

#[command]
pub fn import_search_result(
    project_url: String,
    project_name: String,
    category_id: Option<i64>,
) -> Result<i64, String> {
    let db_guard = DATABASE.lock().unwrap();
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();

    let provider = if project_url.contains("github.com") {
        "github"
    } else if project_url.contains("gitee.com") {
        "gitee"
    } else {
        "crawler"
    };

    let cat_id = category_id.unwrap_or(1);

    conn.execute(
        "INSERT INTO projects (name, url, source, category_id, lifecycle_status) VALUES (?, ?, ?, ?, 'TO_EXPLORE')",
        params![project_name, project_url, provider, cat_id],
    ).map_err(|e| e.to_string())?;

    let project_id = conn.last_insert_rowid();
    Ok(project_id)
}

fn get_system_db() -> Result<rusqlite::Connection, String> {
    let base_dirs = directories::BaseDirs::new().ok_or("Cannot find base directories")?;
    let app_data = base_dirs.data_dir().join(".os-compass");
    std::fs::create_dir_all(&app_data).ok();
    let db_path = app_data.join("plugins.db");
    let conn = rusqlite::Connection::open(&db_path).map_err(|e| e.to_string())?;

    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS embedding_settings (
            id INTEGER PRIMARY KEY,
            embedding_enabled INTEGER DEFAULT 1,
            embedding_api_type TEXT DEFAULT 'openai',
            embedding_api_url TEXT DEFAULT '',
            embedding_api_key TEXT DEFAULT '',
            embedding_model TEXT DEFAULT 'text-embedding-3-small',
            embedding_dimension INTEGER DEFAULT 1536,
            vss_extension_path TEXT DEFAULT ''
        );
        INSERT OR IGNORE INTO embedding_settings (id) VALUES (1);
        "#,
    ).map_err(|e| e.to_string())?;

    Ok(conn)
}

#[command]
pub fn get_embedding_settings() -> Result<EmbeddingSettings, String> {
    let conn = get_system_db()?;

    let mut stmt = conn
        .prepare("SELECT embedding_enabled, embedding_api_type, embedding_api_url, embedding_api_key, embedding_model, embedding_dimension, vss_extension_path FROM embedding_settings WHERE id = 1")
        .map_err(|e| e.to_string())?;

    stmt.query_row([], |row| {
        Ok(EmbeddingSettings {
            embedding_enabled: row.get::<_, i32>(0)? == 1,
            embedding_api_type: row.get(1)?,
            embedding_api_url: row.get(2)?,
            embedding_api_key: row.get(3)?,
            embedding_model: row.get(4)?,
            embedding_dimension: row.get(5)?,
            vec_extension_path: row.get(6)?,
        })
    }).map_err(|e| e.to_string())
}

#[command]
pub fn save_embedding_settings(settings: EmbeddingSettings) -> Result<(), String> {
    let conn = get_system_db()?;

    conn.execute(
        "UPDATE embedding_settings SET embedding_enabled = ?, embedding_api_type = ?, embedding_api_url = ?, embedding_api_key = ?, embedding_model = ?, embedding_dimension = ?, vss_extension_path = ? WHERE id = 1",
        params![
            if settings.embedding_enabled { 1 } else { 0 },
            settings.embedding_api_type,
            settings.embedding_api_url,
            settings.embedding_api_key,
            settings.embedding_model,
            settings.embedding_dimension,
            settings.vec_extension_path,
        ],
    ).map_err(|e| e.to_string())?;

    update_embedding_config(EmbeddingConfig {
        embedding_enabled: settings.embedding_enabled,
        embedding_api_type: settings.embedding_api_type,
        embedding_api_url: settings.embedding_api_url,
        embedding_api_key: settings.embedding_api_key,
        embedding_model: settings.embedding_model,
        embedding_dimension: settings.embedding_dimension,
        vec_extension_path: settings.vec_extension_path,
    });

    Ok(())
}

#[command]
pub fn test_embedding_connection() -> Result<bool, String> {
    Ok(true)
}

#[command]
pub fn list_embedding_models(provider: String, api_url: String, api_key: String) -> Result<Vec<String>, String> {
    let client = reqwest::blocking::Client::new();
    let base_url = api_url.trim_end_matches('/');

    match provider.as_str() {
        "custom" => {
            let url = if base_url.contains("/v1") {
                format!("{}/models", base_url)
            } else {
                format!("{}/v1/models", base_url)
            };

            let response = client.get(&url)
                .header("Authorization", format!("Bearer {}", api_key))
                .send()
                .map_err(|e| format!("请求失败: {}", e))?;

            let body = response.text().map_err(|e| format!("读取响应失败: {}", e))?;

            #[derive(serde::Deserialize)]
            struct ModelItem {
                id: String,
                object: Option<String>,
                #[serde(rename = "type")]
                model_type: Option<String>,
            }
            #[derive(serde::Deserialize)]
            struct ModelsResponse {
                data: Vec<ModelItem>,
            }

            let result: ModelsResponse = serde_json::from_str(&body)
                .map_err(|e| format!("解析失败: {} | 响应: {}", e, &body[..body.len().min(200)]))?;

            let models: Vec<String> = result.data
                .into_iter()
                .filter(|m| {
                    let id = m.id.to_lowercase();
                    id.contains("embedding") || id.contains("reranker") || id.contains("bge")
                })
                .map(|m| m.id)
                .collect();
            Ok(models)
        }
        "deepseek" => {
            Ok(vec![
                "text-embedding-3".to_string(),
                "text-embedding-3-small".to_string(),
            ])
        }
        _ => {
            let base_url = api_url.trim_end_matches('/');
            let url = if base_url.contains("/v1") {
                format!("{}/models", base_url)
            } else {
                format!("{}/v1/models", base_url)
            };

            let response = client.get(&url)
                .header("Authorization", format!("Bearer {}", api_key))
                .send()
                .map_err(|e| e.to_string())?;

            let body = response.text().map_err(|e| e.to_string())?;

            #[derive(serde::Deserialize)]
            struct ModelsResponse {
                data: Vec<serde_json::Value>,
            }

            let result: ModelsResponse = serde_json::from_str(&body)
                .map_err(|e| e.to_string())?;

            let models: Vec<String> = result.data
                .into_iter()
                .filter_map(|m| m.get("id").and_then(|id| id.as_str()).map(|s| s.to_string()))
                .filter(|id| {
                    let lower = id.to_lowercase();
                    lower.contains("embedding") || lower.contains("embed") || lower.contains("bge")
                })
                .collect();
            Ok(models)
        }
    }
}
