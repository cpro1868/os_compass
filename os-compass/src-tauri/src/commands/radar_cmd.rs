use crate::plugins::radar::{init_radar_db, radar_scan_all};
use crate::plugin_config_db::{PLUGIN_CONFIG_DB, RadarScheduleUpdate};
use crate::plugin_manager::PLUGIN_MANAGER;
use crate::vault::CURRENT_VAULT_CONFIG;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use tauri::{command, Emitter};
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

#[derive(Debug, Serialize, Deserialize)]
pub struct RadarSchedule {
    pub enabled: bool,
    pub mode: String,
    pub interval_seconds: i64,
    pub custom_unit: Option<String>,
    pub custom_value: Option<i64>,
    pub notification_enabled: bool,
    pub system_notification: bool,
    pub badge_notification: bool,
    pub last_run_at: Option<String>,
    pub next_run_at: Option<String>,
    pub new_items_count: i64,
}

#[command]
pub fn get_radar_schedule() -> Result<RadarSchedule, String> {
    let row = PLUGIN_CONFIG_DB.get_radar_schedule()
        .map_err(|e| e.to_string())?;
    Ok(RadarSchedule {
        enabled: row.enabled,
        mode: row.mode,
        interval_seconds: row.interval_seconds,
        custom_unit: row.custom_unit,
        custom_value: row.custom_value,
        notification_enabled: row.notification_enabled,
        system_notification: row.system_notification,
        badge_notification: row.badge_notification,
        last_run_at: row.last_run_at,
        next_run_at: row.next_run_at,
        new_items_count: row.new_items_count,
    })
}

#[command]
pub fn update_radar_schedule(update: RadarScheduleUpdate) -> Result<(), String> {
    PLUGIN_CONFIG_DB.update_radar_schedule(&update)
        .map_err(|e| e.to_string())?;
    log::info!("[radar_schedule] Updated schedule: {:?}", serde_json::to_string(&update).unwrap_or_default());
    Ok(())
}

#[command]
pub fn trigger_radar_scan_now() -> Result<serde_json::Value, String> {
    crate::plugins::radar::init_spam_lexicon();
    let (scanned, new_items, errors) = tauri::async_runtime::block_on(crate::plugins::radar::radar_scan_all(None))?;

    if new_items > 0 {
        let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        PLUGIN_CONFIG_DB.update_radar_schedule_run(&now, &now, new_items)
            .map_err(|e| e.to_string())?;
    }

    Ok(serde_json::json!({
        "scanned": scanned,
        "newItems": new_items,
        "errors": errors
    }))
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Notification {
    pub id: i64,
    pub title: String,
    pub body: Option<String>,
    pub item_count: i64,
    pub read: bool,
    pub created_at: String,
}

#[command]
pub fn list_radar_notifications(unread_only: Option<bool>) -> Result<Vec<Notification>, String> {
    let unread = unread_only.unwrap_or(false);
    let rows = PLUGIN_CONFIG_DB.list_notifications(unread)
        .map_err(|e| e.to_string())?;
    Ok(rows.into_iter().map(|row| Notification {
        id: row.id,
        title: row.title,
        body: row.body,
        item_count: row.item_count,
        read: row.read,
        created_at: row.created_at,
    }).collect())
}

#[command]
pub fn mark_radar_notification_read(id: i64) -> Result<(), String> {
    PLUGIN_CONFIG_DB.mark_notification_read(id)
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[command]
pub fn clear_radar_notifications() -> Result<(), String> {
    PLUGIN_CONFIG_DB.clear_notifications()
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[command]
pub fn get_radar_unread_notification_count() -> Result<i64, String> {
    PLUGIN_CONFIG_DB.get_unread_notification_count()
        .map_err(|e| e.to_string())
}

fn calculate_interval_seconds(schedule: &crate::plugin_config_db::RadarScheduleRow) -> u64 {
    if schedule.mode == "custom" {
        let value = schedule.custom_value.unwrap_or(1) as u64;
        let unit = schedule.custom_unit.as_deref().unwrap_or("minute");
        match unit {
            "second" => value,
            "minute" => value * 60,
            "hour" => value * 3600,
            _ => value * 60,
        }
    } else {
        schedule.interval_seconds as u64
    }
}

pub fn start_radar_scheduler(app: tauri::AppHandle) {
    log::info!("[radar_scheduler] Starting radar scheduler...");

    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async move {
            scheduler_loop(app).await;
        });
    });
}

async fn scheduler_loop(app: tauri::AppHandle) {
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

        let schedule = match PLUGIN_CONFIG_DB.get_radar_schedule() {
            Ok(s) => s,
            Err(e) => {
                log::error!("[radar_scheduler] Failed to get schedule: {}", e);
                continue;
            }
        };

        if !schedule.enabled {
            continue;
        }

        let interval = calculate_interval_seconds(&schedule);
        log::info!("[radar_scheduler] interval={} seconds, last_run={:?}", interval, schedule.last_run_at);

        if let Some(last_run) = &schedule.last_run_at {
            let now = chrono::Local::now();
            if let Ok(last) = chrono::NaiveDateTime::parse_from_str(last_run, "%Y-%m-%d %H:%M:%S") {
                let last_dt = chrono::DateTime::<chrono::Local>::from_naive_utc_and_offset(last, *now.offset());
                let elapsed = (now - last_dt).num_seconds() as u64;
                log::info!("[radar_scheduler] elapsed={} seconds", elapsed);
                if elapsed < interval {
                    let wait_time = interval - elapsed;
                    log::info!("[radar_scheduler] Waiting {} seconds before next scan", wait_time);
                    tokio::time::sleep(tokio::time::Duration::from_secs(wait_time)).await;
                } else {
                    log::info!("[radar_scheduler] Interval elapsed, executing scan now");
                }
            } else {
                log::warn!("[radar_scheduler] Failed to parse last_run_at: {}", last_run);
                tokio::time::sleep(tokio::time::Duration::from_secs(interval)).await;
            }
        } else {
            log::info!("[radar_scheduler] No last_run_at, waiting interval seconds");
            tokio::time::sleep(tokio::time::Duration::from_secs(interval)).await;
        }

        if let Ok(current_schedule) = PLUGIN_CONFIG_DB.get_radar_schedule() {
            if !current_schedule.enabled {
                continue;
            }
        } else {
            continue;
        }

        log::info!("[radar_scheduler] Running scheduled radar scan...");

        crate::plugins::radar::init_spam_lexicon();

        let (scanned, new_items, errors) = match radar_scan_all(None).await {
            Ok(result) => result,
            Err(e) => {
                log::error!("[radar_scheduler] Scan failed: {}", e);
                (0, 0, 1)
            }
        };

        let now = chrono::Local::now();
        let last_run = now.format("%Y-%m-%d %H:%M:%S").to_string();
        let next_run = (now + chrono::Duration::seconds(interval as i64)).format("%Y-%m-%d %H:%M:%S").to_string();

        if let Err(e) = PLUGIN_CONFIG_DB.update_radar_schedule_run(&last_run, &next_run, new_items) {
            log::error!("[radar_scheduler] Failed to update schedule run: {}", e);
        }

        if new_items > 0 {
            log::info!("[radar_scheduler] Scan completed: scanned={}, new={}, errors={}", scanned, new_items, errors);

            if let Ok(current_schedule) = PLUGIN_CONFIG_DB.get_radar_schedule() {
                if current_schedule.notification_enabled && current_schedule.system_notification {
                    if let Err(e) = send_system_notification(&app, new_items) {
                        log::error!("[radar_scheduler] Failed to send notification: {}", e);
                    }
                }

                if current_schedule.notification_enabled && current_schedule.badge_notification {
                    let _ = app.emit("radar-new-items", new_items);
                }
            }

            let _ = app.emit("radar-scan-complete", serde_json::json!({
                "scanned": scanned,
                "newItems": new_items,
                "errors": errors
            }));
        }
    }
}

fn send_system_notification(app: &tauri::AppHandle, new_items: i64) -> Result<(), String> {
    use tauri_plugin_notification::NotificationExt;

    app.notification()
        .builder()
        .title("📡 情报更新")
        .body(&format!("发现 {} 条新情报", new_items))
        .show()
        .map_err(|e| e.to_string())
}
