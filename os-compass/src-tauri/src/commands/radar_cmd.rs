use crate::db::open_db_at_path;
use crate::feature_plugin::PluginContext;
use crate::plugins::radar::{init_radar_db, radar_scan_all};
use crate::plugin_manager::PLUGIN_MANAGER;
use crate::settings::get_settings;
use crate::vault::CURRENT_VAULT_CONFIG;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use tauri::command;

#[derive(Debug, Serialize, Deserialize)]
pub struct RadarSource {
    pub id: i64,
    pub name: String,
    pub source_type: String,
    pub url: String,
    pub platform: Option<String>,
    pub enabled: bool,
    pub check_interval: i64,
    pub last_checked_at: Option<String>,
    pub last_status: Option<String>,
    pub last_error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RadarItem {
    pub id: i64,
    pub source_id: i64,
    pub project_name: Option<String>,
    pub project_url: Option<String>,
    pub description: Option<String>,
    pub language: Option<String>,
    pub status: String,
    pub published_at: Option<String>,
    pub fetched_at: String,
}

fn get_radar_conn() -> Result<rusqlite::Connection, String> {
    let config = CURRENT_VAULT_CONFIG.lock().unwrap();
    let path = config.as_ref().ok_or("No vault opened")?;
    let vault_dir = std::path::Path::new(&path.path);
    init_radar_db(vault_dir)
}

#[command]
pub fn list_radar_sources() -> Result<Vec<RadarSource>, String> {
    let conn = get_radar_conn()?;
    let mut stmt = conn
        .prepare("SELECT id, name, source_type, url, platform, enabled, check_interval, last_checked_at, last_status, last_error FROM radar_sources ORDER BY created_at DESC")
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map([], |row| {
            Ok(RadarSource {
                id: row.get(0)?,
                name: row.get(1)?,
                source_type: row.get(2)?,
                url: row.get(3)?,
                platform: row.get(4)?,
                enabled: row.get::<_, i32>(5)? == 1,
                check_interval: row.get(6)?,
                last_checked_at: row.get(7)?,
                last_status: row.get(8)?,
                last_error: row.get(9)?,
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
pub fn add_radar_source(
    name: String,
    source_type: String,
    url: String,
    platform: Option<String>,
    check_interval: Option<i64>,
) -> Result<RadarSource, String> {
    let conn = get_radar_conn()?;
    let interval = check_interval.unwrap_or(86400);

    conn.execute(
        "INSERT INTO radar_sources (name, source_type, url, platform, check_interval) VALUES (?, ?, ?, ?, ?)",
        params![name, source_type, url, platform, interval],
    ).map_err(|e| e.to_string())?;

    let id = conn.last_insert_rowid();

    Ok(RadarSource {
        id,
        name,
        source_type,
        url,
        platform,
        enabled: true,
        check_interval: interval,
        last_checked_at: None,
        last_status: None,
        last_error: None,
    })
}

#[command]
pub fn update_radar_source(
    id: i64,
    name: Option<String>,
    url: Option<String>,
    enabled: Option<bool>,
    check_interval: Option<i64>,
) -> Result<(), String> {
    let conn = get_radar_conn()?;

    if let Some(n) = name {
        conn.execute("UPDATE radar_sources SET name = ?, updated_at = datetime('now', 'localtime') WHERE id = ?", params![n, id])
            .map_err(|e| e.to_string())?;
    }
    if let Some(u) = url {
        conn.execute("UPDATE radar_sources SET url = ?, updated_at = datetime('now', 'localtime') WHERE id = ?", params![u, id])
            .map_err(|e| e.to_string())?;
    }
    if let Some(e) = enabled {
        conn.execute("UPDATE radar_sources SET enabled = ?, updated_at = datetime('now', 'localtime') WHERE id = ?", params![if e { 1 } else { 0 }, id])
            .map_err(|e| e.to_string())?;
    }
    if let Some(i) = check_interval {
        conn.execute("UPDATE radar_sources SET check_interval = ?, updated_at = datetime('now', 'localtime') WHERE id = ?", params![i, id])
            .map_err(|e| e.to_string())?;
    }

    Ok(())
}

#[command]
pub fn delete_radar_source(id: i64) -> Result<(), String> {
    let conn = get_radar_conn()?;
    conn.execute("DELETE FROM radar_sources WHERE id = ?", params![id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[command]
pub fn get_radar_items(status: Option<String>, limit: Option<i64>) -> Result<Vec<RadarItem>, String> {
    let conn = get_radar_conn()?;
    let lim = limit.unwrap_or(100);

    let mut items = Vec::new();

    match status {
        Some(s) => {
            let mut stmt = conn.prepare("SELECT id, source_id, project_name, project_url, description, language, status, published_at, fetched_at FROM radar_items WHERE status = ? ORDER BY fetched_at DESC LIMIT ?")
                .map_err(|e| e.to_string())?;
            let rows = stmt.query_map(params![s, lim], |row| {
                Ok(RadarItem {
                    id: row.get(0)?,
                    source_id: row.get(1)?,
                    project_name: row.get(2)?,
                    project_url: row.get(3)?,
                    description: row.get(4)?,
                    language: row.get(5)?,
                    status: row.get(6)?,
                    published_at: row.get(7)?,
                    fetched_at: row.get(8)?,
                })
            }).map_err(|e| e.to_string())?;
            for row in rows {
                items.push(row.map_err(|e| e.to_string())?);
            }
        }
        None => {
            let mut stmt = conn.prepare("SELECT id, source_id, project_name, project_url, description, language, status, published_at, fetched_at FROM radar_items ORDER BY fetched_at DESC LIMIT ?")
                .map_err(|e| e.to_string())?;
            let rows = stmt.query_map(params![lim], |row| {
                Ok(RadarItem {
                    id: row.get(0)?,
                    source_id: row.get(1)?,
                    project_name: row.get(2)?,
                    project_url: row.get(3)?,
                    description: row.get(4)?,
                    language: row.get(5)?,
                    status: row.get(6)?,
                    published_at: row.get(7)?,
                    fetched_at: row.get(8)?,
                })
            }).map_err(|e| e.to_string())?;
            for row in rows {
                items.push(row.map_err(|e| e.to_string())?);
            }
        }
    }

    Ok(items)
}

#[command]
pub fn radar_item_action(
    item_id: i64,
    action: String,
    category_id: Option<i64>,
) -> Result<Option<i64>, String> {
    let conn = get_radar_conn()?;

    match action.as_str() {
        "import" => {
            let url: Option<String> = conn
                .query_row(
                    "SELECT project_url FROM radar_items WHERE id = ?",
                    params![item_id],
                    |row| row.get(0),
                )
                .map_err(|e| e.to_string())?;

            let project_url = url.ok_or("No project URL")?;
            let project_name: Option<String> = conn
                .query_row(
                    "SELECT project_name FROM radar_items WHERE id = ?",
                    params![item_id],
                    |row| row.get(0),
                )
                .map_err(|e| e.to_string())?;

            let settings = get_settings();
            let _provider = if project_url.contains("github.com") {
                "github"
            } else if project_url.contains("gitee.com") {
                "gitee"
            } else {
                "crawler"
            };

            let cat_id = category_id.unwrap_or(1);

            let db_guard = crate::db::DATABASE.lock().unwrap();
            let db = db_guard.as_ref().ok_or("Database not initialized")?;
            let main_conn = db.get_connection();

            let description: Option<String> = conn
                .query_row(
                    "SELECT description FROM radar_items WHERE id = ?",
                    params![item_id],
                    |row| row.get(0),
                )
                .map_err(|e| e.to_string())?;

            main_conn.execute(
                "INSERT INTO projects (name, url, source, description, category_id, lifecycle_status) VALUES (?, ?, ?, ?, ?, 'TO_EXPLORE')",
                params![project_name, project_url, _provider, description, cat_id],
            ).map_err(|e| e.to_string())?;

            let project_id = main_conn.last_insert_rowid();

            conn.execute(
                "UPDATE radar_items SET status = 'imported', imported_project_id = ? WHERE id = ?",
                params![project_id, item_id],
            ).map_err(|e| e.to_string())?;

            Ok(Some(project_id))
        }
        "blacklist" => {
            let url_hash: String = conn
                .query_row(
                    "SELECT url_hash FROM radar_items WHERE id = ?",
                    params![item_id],
                    |row| row.get(0),
                )
                .map_err(|e| e.to_string())?;

            let project_url: Option<String> = conn
                .query_row(
                    "SELECT project_url FROM radar_items WHERE id = ?",
                    params![item_id],
                    |row| row.get(0),
                )
                .map_err(|e| e.to_string())?;

            conn.execute(
                "INSERT OR IGNORE INTO radar_blacklist (url_hash, project_url) VALUES (?, ?)",
                params![url_hash, project_url],
            ).map_err(|e| e.to_string())?;

            conn.execute(
                "UPDATE radar_items SET status = 'blacklisted' WHERE id = ?",
                params![item_id],
            ).map_err(|e| e.to_string())?;

            Ok(None)
        }
        "ignore" => {
            conn.execute(
                "UPDATE radar_items SET status = 'ignored' WHERE id = ?",
                params![item_id],
            ).map_err(|e| e.to_string())?;
            Ok(None)
        }
        _ => Err("Unknown action".to_string()),
    }
}

#[command]
pub fn trigger_radar_scan() -> Result<serde_json::Value, String> {
    let vault_dir = {
        let config = CURRENT_VAULT_CONFIG.lock().unwrap();
        let path = config.as_ref().ok_or("No vault opened")?;
        std::path::PathBuf::from(&path.path)
    };

    let (scanned, new_items, errors) = tauri::async_runtime::block_on(crate::plugins::radar::radar_scan_all(&vault_dir))?;

    Ok(serde_json::json!({
        "scanned": scanned,
        "newItems": new_items,
        "errors": errors
    }))
}

#[command]
pub fn get_radar_unread_count() -> Result<i64, String> {
    let conn = get_radar_conn()?;
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM radar_items WHERE status = 'unread'",
            [],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;
    Ok(count)
}

#[command]
pub fn clear_radar_cache(before_days: Option<i64>) -> Result<i64, String> {
    let conn = get_radar_conn()?;

    let affected = if let Some(days) = before_days {
        conn.execute(
            "DELETE FROM radar_items WHERE status IN ('imported', 'ignored', 'blacklisted') AND fetched_at < datetime('now', ?)",
            params![format!("-{} days", days)],
        )
    } else {
        conn.execute(
            "DELETE FROM radar_items WHERE status IN ('imported', 'ignored', 'blacklisted')",
            [],
        )
    }.map_err(|e| e.to_string())?;

    Ok(affected as i64)
}
