use crate::db::open_db_at_path;
use crate::feature_plugin::PluginContext;
use crate::plugins::radar::{init_radar_db, radar_scan_all};
use crate::plugin_config_db::PLUGIN_CONFIG_DB;
use crate::plugin_manager::PLUGIN_MANAGER;
use crate::settings::get_settings;
use crate::vault::CURRENT_VAULT_CONFIG;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use tauri::command;
use log;

#[derive(Debug, Serialize, Deserialize)]
pub struct RadarSource {
    pub id: i64,
    pub name: String,
    pub source_type: String,
    pub url: String,
    pub platform: Option<String>,
    pub enabled: bool,
    pub check_interval: i64,
    pub proxy_enabled: bool,
    pub proxy_protocol: String,
    pub proxy_host: Option<String>,
    pub proxy_port: i64,
    pub proxy_username: Option<String>,
    pub proxy_password: Option<String>,
    pub last_checked_at: Option<String>,
    pub last_status: Option<String>,
    pub last_error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RadarItem {
    pub id: i64,
    pub source_id: i64,
    pub source_name: Option<String>,
    pub project_name: Option<String>,
    pub project_url: Option<String>,
    pub description: Option<String>,
    pub language: Option<String>,
    pub status: String,
    pub published_at: Option<String>,
    pub fetched_at: String,
}

fn check_plugin_enabled(plugin_id: &str) -> Result<(), String> {
    log::debug!("[radar] check_plugin_enabled: {}", plugin_id);
    let enabled = PLUGIN_MANAGER.is_enabled(plugin_id);
    log::debug!("[radar] is_enabled: {:?}", enabled);
    match enabled {
        Ok(true) => Ok(()),
        Ok(false) => Err(format!("Plugin '{}' is disabled", plugin_id)),
        Err(e) => {
            log::warn!("[radar] is_enabled error: {}", e);
            Err(format!("Plugin '{}' error: {}", plugin_id, e))
        }
    }
}

#[command]
pub fn debug_vault_status() -> String {
    let config_guard = CURRENT_VAULT_CONFIG.lock().unwrap();
    let status = format!("CURRENT_VAULT_CONFIG: {:?}, last_vault.txt: {:?}",
        config_guard,
        std::fs::read_to_string(
            std::env::var("APPDATA").unwrap_or_default().to_string() + "\\com.administrator.os-compass\\last-vault.txt"
        ).ok()
    );
    log::debug!("[debug_vault_status] {}", status);
    status
}

fn get_radar_conn() -> Result<rusqlite::Connection, String> {
    // 采集数据在系统目录
    init_radar_db()
}

#[command]
pub fn list_radar_sources() -> Result<Vec<RadarSource>, String> {
    let sources = PLUGIN_CONFIG_DB.list_radar_sources();
    Ok(sources.into_iter().map(|r| RadarSource {
        id: r.id,
        name: r.name,
        source_type: r.source_type,
        url: r.url,
        platform: r.platform,
        enabled: true,
        check_interval: r.check_interval,
        proxy_enabled: r.proxy_enabled,
        proxy_protocol: r.proxy_protocol,
        proxy_host: if r.proxy_host.is_empty() { None } else { Some(r.proxy_host) },
        proxy_port: r.proxy_port as i64,
        proxy_username: if r.proxy_username.is_empty() { None } else { Some(r.proxy_username) },
        proxy_password: if r.proxy_password.is_empty() { None } else { Some(r.proxy_password) },
        last_checked_at: r.last_checked_at,
        last_status: r.last_status,
        last_error: r.last_error,
    }).collect())
}

#[command]
pub fn add_radar_source(
    name: String,
    source_type: String,
    url: String,
    platform: Option<String>,
    check_interval: Option<i64>,
    proxy_enabled: Option<bool>,
    proxy_protocol: Option<String>,
    proxy_host: Option<String>,
    proxy_port: Option<i64>,
    proxy_username: Option<String>,
    proxy_password: Option<String>,
) -> Result<RadarSource, String> {
    let proxy_proto = proxy_protocol.unwrap_or_else(|| "http".to_string());
    let proxy_host_val = proxy_host.clone().unwrap_or_default();
    let proxy_username_val = proxy_username.clone().unwrap_or_default();
    let proxy_password_val = proxy_password.clone().unwrap_or_default();

    let id = PLUGIN_CONFIG_DB.add_radar_source(
        &name,
        &source_type,
        &url,
        platform.as_deref(),
        proxy_enabled.unwrap_or(false),
        &proxy_proto,
        &proxy_host_val,
        proxy_port.unwrap_or(0) as i32,
        &proxy_username_val,
        &proxy_password_val,
    ).map_err(|e| e.to_string())?;

    Ok(RadarSource {
        id,
        name,
        source_type,
        url,
        platform,
        enabled: true,
        check_interval: check_interval.unwrap_or(86400),
        proxy_enabled: proxy_enabled.unwrap_or(false),
        proxy_protocol: proxy_proto,
        proxy_host: proxy_host.filter(|s| !s.is_empty()),
        proxy_port: proxy_port.unwrap_or(0),
        proxy_username: proxy_username.filter(|s| !s.is_empty()),
        proxy_password: proxy_password.filter(|s| !s.is_empty()),
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
    proxy_enabled: Option<bool>,
    proxy_protocol: Option<String>,
    proxy_host: Option<String>,
    proxy_port: Option<i64>,
    proxy_username: Option<String>,
    proxy_password: Option<String>,
) -> Result<(), String> {
    PLUGIN_CONFIG_DB.update_radar_source(
        id,
        name.as_deref(),
        url.as_deref(),
        enabled,
        check_interval,
        proxy_enabled,
        proxy_protocol.as_deref(),
        proxy_host.as_deref(),
        proxy_port.map(|p| p as i32),
        proxy_username.as_deref(),
        proxy_password.as_deref(),
    ).map_err(|e| e.to_string())
}

#[command]
pub fn delete_radar_source(id: i64) -> Result<(), String> {
    PLUGIN_CONFIG_DB.delete_radar_source(id).map_err(|e| e.to_string())
}

#[command]
pub fn get_radar_items(
    status: Option<String>,
    limit: Option<i64>,
    keyword: Option<String>,
    source_ids: Option<String>,
    start_date: Option<String>,
    end_date: Option<String>,
    page: Option<i64>,
    page_size: Option<i64>,
) -> Result<RadarItemsResult, String> {
    let conn = get_radar_conn()?;
    let lim = limit.unwrap_or(500);
    let page = page.unwrap_or(1).max(1);
    let page_size = page_size.unwrap_or(20).min(100);
    let offset = (page - 1) * page_size;

    let mut conditions = Vec::new();

    if let Some(ref s) = status {
        conditions.push(format!("status = '{}'", s));
    }

    if let Some(ref kw) = keyword {
        if !kw.is_empty() {
            conditions.push(format!(
                "(project_name LIKE '%{}%' OR description LIKE '%{}%')",
                kw.replace('\'', "''"),
                kw.replace('\'', "''")
            ));
        }
    }

    if let Some(ref ids) = source_ids {
        if !ids.is_empty() {
            conditions.push(format!("source_id IN ({})", ids));
        }
    }

    if let Some(ref start) = start_date {
        conditions.push(format!("COALESCE(published_at, fetched_at) >= '{}'", start));
    }

    if let Some(ref end) = end_date {
        conditions.push(format!("COALESCE(published_at, fetched_at) <= '{}'", end));
    }

    let where_clause = if conditions.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", conditions.join(" AND "))
    };

    // 查询总数
    let count_query = format!("SELECT COUNT(*) FROM radar_items {}", where_clause);
    let total: i64 = conn
        .query_row(&count_query, [], |row| row.get(0))
        .map_err(|e| e.to_string())?;

    // 查询数据
    let query = format!(
        "SELECT id, source_id, project_name, project_url, description, language, status, published_at, fetched_at \
         FROM radar_items {} \
         ORDER BY COALESCE(published_at, fetched_at) DESC \
         LIMIT {} OFFSET {}",
        where_clause, lim, offset
    );

    let mut stmt = conn.prepare(&query).map_err(|e| e.to_string())?;
    let rows = stmt.query_map([], |row| {
        Ok(RadarItem {
            id: row.get(0)?,
            source_id: row.get(1)?,
            source_name: None,
            project_name: row.get(2)?,
            project_url: row.get(3)?,
            description: row.get(4)?,
            language: row.get(5)?,
            status: row.get(6)?,
            published_at: row.get(7)?,
            fetched_at: row.get(8)?,
        })
    }).map_err(|e| e.to_string())?;

    let mut items = Vec::new();
    for row in rows {
        let mut item = row.map_err(|e| e.to_string())?;
        if let Ok(sources) = PLUGIN_CONFIG_DB.get_radar_source_name(item.source_id) {
            item.source_name = Some(sources);
        }
        items.push(item);
    }

    let total_pages = (total as f64 / page_size as f64).ceil() as i64;

    log::debug!("[radar] get_radar_items returning {} items (page {}/{})", items.len(), page, total_pages);
    Ok(RadarItemsResult {
        items,
        total,
        page,
        page_size,
        total_pages,
    })
}

#[derive(Serialize, Deserialize)]
pub struct RadarItemsResult {
    pub items: Vec<RadarItem>,
    pub total: i64,
    pub page: i64,
    pub page_size: i64,
    pub total_pages: i64,
}

#[command]
pub fn radar_item_action(
    item_id: i64,
    action: String,
    category_id: Option<i64>,
) -> Result<Option<i64>, String> {
    let conn = get_radar_conn()?;

    match action.as_str() {
        "collect" => {
            conn.execute(
                "UPDATE radar_items SET status = 'collected' WHERE id = ?",
                params![item_id],
            ).map_err(|e| e.to_string())?;
            Ok(None)
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
        "delete" => {
            conn.execute(
                "DELETE FROM radar_items WHERE id = ?",
                params![item_id],
            ).map_err(|e| e.to_string())?;
            Ok(None)
        }
        _ => Err("Unknown action".to_string()),
    }
}

#[command]
pub fn trigger_radar_scan(time_range: Option<String>) -> Result<serde_json::Value, String> {
    crate::plugins::radar::init_spam_lexicon();
    let time_range_str = time_range.as_deref();
    let (scanned, new_items, errors) = tauri::async_runtime::block_on(crate::plugins::radar::radar_scan_all(time_range_str))?;

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

#[command]
pub fn clear_radar_all() -> Result<i64, String> {
    let conn = get_radar_conn()?;
    let affected = conn.execute("DELETE FROM radar_items", [])
        .map_err(|e| e.to_string())?;
    log::debug!("[radar] Cleared {} items", affected);
    Ok(affected as i64)
}
