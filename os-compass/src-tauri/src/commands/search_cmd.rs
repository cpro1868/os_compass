use crate::db::DATABASE;
use crate::embedding::{EmbeddingConfig, update_config as update_embedding_config};
use crate::plugins::search::{three_layer_search, SearchResult, ProjectMatch};
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
    let vault_dir = {
        let config = CURRENT_VAULT_CONFIG.lock().unwrap();
        let path = config.as_ref().ok_or("No vault opened")?;
        std::path::PathBuf::from(&path.path)
    };

    tauri::async_runtime::block_on(three_layer_search(&vault_dir, &query))
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
            vss_extension_path: row.get(6)?,
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
            settings.vss_extension_path,
        ],
    ).map_err(|e| e.to_string())?;

    update_embedding_config(EmbeddingConfig {
        embedding_enabled: settings.embedding_enabled,
        embedding_api_type: settings.embedding_api_type,
        embedding_api_url: settings.embedding_api_url,
        embedding_api_key: settings.embedding_api_key,
        embedding_model: settings.embedding_model,
        embedding_dimension: settings.embedding_dimension,
        vss_extension_path: settings.vss_extension_path,
    });

    Ok(())
}

#[command]
pub fn test_embedding_connection() -> Result<bool, String> {
    Ok(true)
}

#[command]
pub fn list_embedding_models(provider: String, _api_url: String, _api_key: String) -> Result<Vec<String>, String> {
    match provider.as_str() {
        "custom" => {
            Ok(vec![
                "BAAI/bge-m3".to_string(),
                "BAAI/bge-large-zh".to_string(),
                "netease-youdao/bce-multimodalembedding-multilingual".to_string(),
                "Pro/Qwen/Qwen2.5-MOE".to_string(),
                "Pro/Qwen/Qwen2.5-7B".to_string(),
            ])
        }
        "deepseek" => {
            Ok(vec![
                "text-embedding-3".to_string(),
                "text-embedding-3-small".to_string(),
                "text-embedding-2".to_string(),
            ])
        }
        "ollama" => {
            let client = reqwest::blocking::Client::new();
            let url = format!("{}/api/tags", _api_url.trim_end_matches('/'));

            let response = client.get(&url).send().map_err(|e| e.to_string())?;

            #[derive(serde::Deserialize)]
            struct OllamaResponse {
                models: Vec<OllamaModel>,
            }
            #[derive(serde::Deserialize)]
            struct OllamaModel {
                name: String,
            }

            let result: OllamaResponse = response.json().map_err(|e| e.to_string())?;
            let models: Vec<String> = result.models
                .into_iter()
                .filter(|m| m.name.contains("embedding"))
                .map(|m| m.name)
                .collect();
            Ok(models)
        }
        _ => {
            let client = reqwest::blocking::Client::new();
            let url = format!("{}/models", _api_url.trim_end_matches('/'));

            let response = client.get(&url)
                .header("Authorization", format!("Bearer {}", _api_key))
                .send()
                .map_err(|e| e.to_string())?;

            #[derive(serde::Deserialize)]
            struct OpenAIResponse {
                data: Vec<serde_json::Value>,
            }

            let result: OpenAIResponse = response.json().map_err(|e| e.to_string())?;
            let models: Vec<String> = result.data
                .into_iter()
                .filter_map(|m| {
                    m.get("id")
                        .and_then(|id| id.as_str())
                        .filter(|id| id.contains("embedding"))
                        .map(|s| s.to_string())
                })
                .collect();
            Ok(models)
        }
    }
}
