use crate::db::DATABASE;
use crate::plugins::search::{three_layer_search, SearchResult, ProjectMatch};
use crate::vault::CURRENT_VAULT_CONFIG;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use tauri::command;

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
