use crate::db::DATABASE;
use crate::embedding::{EmbeddingConfig, update_config as update_embedding_config};
use crate::plugins::search::{three_layer_search, SearchResult, ProjectMatch, IntentAnalysis, analyze_intent};
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

fn get_search_conn() -> Result<rusqlite::Connection, String> {
    let config = CURRENT_VAULT_CONFIG.lock().unwrap();
    let path = config.as_ref().ok_or("No vault opened")?;
    if path.path.is_empty() {
        return Err("Vault not initialized".to_string());
    }
    let vault_dir = std::path::Path::new(&path.path);
    crate::plugins::search::init_search_db(vault_dir)
}

#[command]
pub fn intent_search(query: String, _conversation_id: Option<String>) -> Result<SearchResult, String> {
    eprintln!("[DEBUG] intent_search: 开始处理查询 '{}'", query);
    let vault_dir = {
        let config = CURRENT_VAULT_CONFIG.lock().unwrap();
        eprintln!("[DEBUG] intent_search: 获取 vault config 成功");
        let path = config.as_ref().ok_or("No vault opened")?;
        eprintln!("[DEBUG] intent_search: vault path: {:?}", path.path);
        std::path::PathBuf::from(&path.path)
    };

    eprintln!("[DEBUG] intent_search: 调用 three_layer_search...");
    let result = tauri::async_runtime::block_on(three_layer_search(&vault_dir, &query));
    eprintln!("[DEBUG] intent_search: three_layer_search 完成，结果: {:?}", result.is_ok());
    result
}

#[command]
pub async fn analyze_user_intent(user_input: String) -> Result<IntentAnalysis, String> {
    analyze_intent(&user_input).await
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
        "UPDATE embedding_settings SET embedding_enabled = ?, embedding_api_type = ?, embedding_api_url = ?, embedding_api_key = ?, embedding_model = ?, embedding_dimension = ?, vec_extension_path = ? WHERE id = 1",
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
