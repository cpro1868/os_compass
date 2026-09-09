use crate::feature_plugin::PluginInfo;
use log;
use rusqlite::{Connection, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Mutex;

pub struct PluginConfigDb {
    conn: Mutex<Connection>,
}

impl PluginConfigDb {
    pub fn new() -> Result<Self> {
        let db_path = get_plugin_config_db_path();

        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent).ok();
        }

        let conn = Connection::open(&db_path)?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;

        let db = PluginConfigDb {
            conn: Mutex::new(conn),
        };
        db.init_schema()?;

        Ok(db)
    }

    fn init_schema(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS system_plugins (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                plugin_type TEXT NOT NULL,
                version TEXT DEFAULT '1.0.0',
                enabled INTEGER DEFAULT 0,
                config TEXT,
                db_mode TEXT DEFAULT 'none',
                db_path_template TEXT,
                created_at TEXT DEFAULT (datetime('now', 'localtime')),
                updated_at TEXT DEFAULT (datetime('now', 'localtime'))
            );

            CREATE TABLE IF NOT EXISTS vault_plugins (
                vault_path TEXT NOT NULL,
                plugin_id TEXT NOT NULL,
                enabled INTEGER DEFAULT 0,
                config TEXT,
                PRIMARY KEY (vault_path, plugin_id)
            );

            CREATE TABLE IF NOT EXISTS radar_sources (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                source_type TEXT NOT NULL,
                url TEXT NOT NULL,
                platform TEXT,
                check_interval INTEGER DEFAULT 3600,
                proxy_enabled INTEGER DEFAULT 0,
                proxy_protocol TEXT DEFAULT 'http',
                proxy_host TEXT,
                proxy_port INTEGER DEFAULT 0,
                proxy_username TEXT,
                proxy_password TEXT,
                last_checked_at TEXT,
                last_status TEXT,
                last_error TEXT,
                created_at TEXT DEFAULT (datetime('now', 'localtime')),
                updated_at TEXT DEFAULT (datetime('now', 'localtime'))
            );

            CREATE TABLE IF NOT EXISTS radar_schedule (
                id INTEGER PRIMARY KEY CHECK(id = 1),
                enabled INTEGER DEFAULT 0,
                mode TEXT DEFAULT 'interval',
                interval_seconds INTEGER DEFAULT 1800,
                custom_unit TEXT,
                custom_value INTEGER,
                notification_enabled INTEGER DEFAULT 1,
                system_notification INTEGER DEFAULT 1,
                badge_notification INTEGER DEFAULT 1,
                last_run_at TEXT,
                next_run_at TEXT,
                new_items_count INTEGER DEFAULT 0,
                updated_at TEXT DEFAULT (datetime('now', 'localtime'))
            );

            CREATE INDEX IF NOT EXISTS idx_radar_schedule_enabled ON radar_schedule(enabled);

            CREATE TABLE IF NOT EXISTS radar_notifications (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                title TEXT NOT NULL,
                body TEXT,
                item_count INTEGER DEFAULT 0,
                read INTEGER DEFAULT 0,
                created_at TEXT DEFAULT (datetime('now', 'localtime'))
            );

            CREATE INDEX IF NOT EXISTS idx_radar_notifications_read ON radar_notifications(read);

            INSERT OR IGNORE INTO radar_schedule (id, enabled, mode, interval_seconds) VALUES (1, 0, 'interval', 1800);
            "#
        )?;
        Ok(())
    }

    pub fn list_system_plugins(&self) -> Vec<PluginInfo> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = match conn.prepare(
            "SELECT id, name, plugin_type, version, enabled, config, db_mode, db_path_template FROM system_plugins"
        ) {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        };

        stmt.query_map([], |row| {
            Ok(PluginInfo {
                id: row.get(0)?,
                name: row.get(1)?,
                plugin_type: row.get(2)?,
                version: row.get(3)?,
                enabled: row.get::<_, i32>(4)? == 1,
                config: row.get(5)?,
                db_mode: row
                    .get::<_, Option<String>>(6)?
                    .unwrap_or_else(|| "none".to_string()),
                db_path_template: row.get(7)?,
            })
        })
        .ok()
        .map(|iter| iter.filter_map(|r| r.ok()).collect())
        .unwrap_or_default()
    }

    pub fn register_system_plugin(&self, info: &PluginInfo) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO system_plugins (id, name, plugin_type, version, enabled, config, db_mode, db_path_template, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, datetime('now', 'localtime'))",
            rusqlite::params![
                info.id,
                info.name,
                info.plugin_type,
                info.version,
                if info.enabled { 1 } else { 0 },
                info.config,
                info.db_mode,
                info.db_path_template,
            ],
        )?;
        Ok(())
    }

    pub fn set_enabled(&self, plugin_id: &str, enabled: bool) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE system_plugins SET enabled = ?, updated_at = datetime('now', 'localtime') WHERE id = ?",
            rusqlite::params![if enabled { 1 } else { 0 }, plugin_id],
        )?;
        Ok(())
    }

    pub fn unregister_plugin(&self, plugin_id: &str) -> Result<(), String> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "DELETE FROM system_plugins WHERE id = ?",
            rusqlite::params![plugin_id],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn get_enabled(&self, plugin_id: &str) -> bool {
        let conn = self.conn.lock().unwrap();
        let result = conn.query_row(
            "SELECT enabled FROM system_plugins WHERE id = ?",
            rusqlite::params![plugin_id],
            |row| row.get::<_, i32>(0),
        );
        match result {
            Ok(enabled) => {
                log::debug!(
                    "[plugin_config_db] get_enabled({}) = {}",
                    plugin_id,
                    enabled
                );
                enabled == 1
            }
            Err(e) => {
                log::warn!(
                    "[plugin_config_db] get_enabled({}) error: {:?}",
                    plugin_id,
                    e
                );
                false
            }
        }
    }

    pub fn get_config(&self, plugin_id: &str) -> Option<String> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT config FROM system_plugins WHERE id = ?",
            rusqlite::params![plugin_id],
            |row| row.get::<_, Option<String>>(0),
        )
        .ok()
        .flatten()
    }

    pub fn save_config(&self, plugin_id: &str, config: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE system_plugins SET config = ?, updated_at = datetime('now', 'localtime') WHERE id = ?",
            rusqlite::params![config, plugin_id],
        )?;
        Ok(())
    }

    // ========== Radar Sources (系统级) ==========

    pub fn add_radar_source(
        &self,
        name: &str,
        source_type: &str,
        url: &str,
        platform: Option<&str>,
        proxy_enabled: bool,
        proxy_protocol: &str,
        proxy_host: &str,
        proxy_port: i32,
        proxy_username: &str,
        proxy_password: &str,
    ) -> Result<i64> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO radar_sources (name, source_type, url, platform, check_interval, proxy_enabled, proxy_protocol, proxy_host, proxy_port, proxy_username, proxy_password)
             VALUES (?, ?, ?, ?, 3600, ?, ?, ?, ?, ?, ?)",
            rusqlite::params![name, source_type, url, platform, proxy_enabled as i32, proxy_protocol, proxy_host, proxy_port, proxy_username, proxy_password],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn list_radar_sources(&self) -> Vec<RadarSourceRow> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = match conn.prepare(
            "SELECT id, name, source_type, url, platform, check_interval, proxy_enabled, proxy_protocol, proxy_host, proxy_port, proxy_username, proxy_password, last_checked_at, last_status, last_error FROM radar_sources ORDER BY created_at DESC"
        ) {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        };
        stmt.query_map([], |row| {
            Ok(RadarSourceRow {
                id: row.get(0)?,
                name: row.get(1)?,
                source_type: row.get(2)?,
                url: row.get(3)?,
                platform: row.get(4)?,
                check_interval: row.get::<_, Option<i64>>(5)?.unwrap_or(3600),
                proxy_enabled: row.get::<_, Option<i32>>(6)?.unwrap_or(0) == 1,
                proxy_protocol: row
                    .get::<_, Option<String>>(7)?
                    .unwrap_or_else(|| "http".to_string()),
                proxy_host: row.get::<_, Option<String>>(8)?.unwrap_or_default(),
                proxy_port: row.get::<_, Option<i64>>(9)?.unwrap_or(0) as i32,
                proxy_username: row.get::<_, Option<String>>(10)?.unwrap_or_default(),
                proxy_password: row.get::<_, Option<String>>(11)?.unwrap_or_default(),
                last_checked_at: row.get(12)?,
                last_status: row.get(13)?,
                last_error: row.get(14)?,
            })
        })
        .ok()
        .map(|iter| iter.filter_map(|r| r.ok()).collect())
        .unwrap_or_default()
    }

    pub fn update_radar_source_status(
        &self,
        id: i64,
        status: &str,
        error: Option<&str>,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE radar_sources SET last_checked_at = datetime('now', 'localtime'), last_status = ?, last_error = ? WHERE id = ?",
            rusqlite::params![status, error, id],
        )?;
        Ok(())
    }

    pub fn update_radar_source(
        &self,
        id: i64,
        name: Option<&str>,
        url: Option<&str>,
        enabled: Option<bool>,
        check_interval: Option<i64>,
        proxy_enabled: Option<bool>,
        proxy_protocol: Option<&str>,
        proxy_host: Option<&str>,
        proxy_port: Option<i32>,
        proxy_username: Option<&str>,
        proxy_password: Option<&str>,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        if let Some(n) = name {
            conn.execute("UPDATE radar_sources SET name = ?, updated_at = datetime('now', 'localtime') WHERE id = ?", rusqlite::params![n, id])?;
        }
        if let Some(u) = url {
            conn.execute("UPDATE radar_sources SET url = ?, updated_at = datetime('now', 'localtime') WHERE id = ?", rusqlite::params![u, id])?;
        }
        if let Some(e) = enabled {
            conn.execute("UPDATE radar_sources SET enabled = ?, updated_at = datetime('now', 'localtime') WHERE id = ?", rusqlite::params![if e { 1 } else { 0 }, id])?;
        }
        if let Some(i) = check_interval {
            conn.execute("UPDATE radar_sources SET check_interval = ?, updated_at = datetime('now', 'localtime') WHERE id = ?", rusqlite::params![i, id])?;
        }
        if let Some(e) = proxy_enabled {
            conn.execute("UPDATE radar_sources SET proxy_enabled = ?, updated_at = datetime('now', 'localtime') WHERE id = ?", rusqlite::params![if e { 1 } else { 0 }, id])?;
        }
        if let Some(p) = proxy_protocol {
            conn.execute("UPDATE radar_sources SET proxy_protocol = ?, updated_at = datetime('now', 'localtime') WHERE id = ?", rusqlite::params![p, id])?;
        }
        if let Some(h) = proxy_host {
            conn.execute("UPDATE radar_sources SET proxy_host = ?, updated_at = datetime('now', 'localtime') WHERE id = ?", rusqlite::params![h, id])?;
        }
        if let Some(p) = proxy_port {
            conn.execute("UPDATE radar_sources SET proxy_port = ?, updated_at = datetime('now', 'localtime') WHERE id = ?", rusqlite::params![p, id])?;
        }
        if let Some(u) = proxy_username {
            conn.execute("UPDATE radar_sources SET proxy_username = ?, updated_at = datetime('now', 'localtime') WHERE id = ?", rusqlite::params![u, id])?;
        }
        if let Some(p) = proxy_password {
            conn.execute("UPDATE radar_sources SET proxy_password = ?, updated_at = datetime('now', 'localtime') WHERE id = ?", rusqlite::params![p, id])?;
        }

        Ok(())
    }

    pub fn delete_radar_source(&self, id: i64) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "DELETE FROM radar_sources WHERE id = ?",
            rusqlite::params![id],
        )?;
        Ok(())
    }

    pub fn get_radar_source_name(&self, id: i64) -> Result<String, String> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT name FROM radar_sources WHERE id = ?",
            rusqlite::params![id],
            |row| row.get::<_, String>(0),
        )
        .map_err(|e| e.to_string())
    }

    pub fn get_radar_schedule(&self) -> Result<RadarScheduleRow, String> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT id, enabled, mode, interval_seconds, custom_unit, custom_value, notification_enabled, system_notification, badge_notification, last_run_at, next_run_at, new_items_count FROM radar_schedule WHERE id = 1",
            [],
            |row| {
                Ok(RadarScheduleRow {
                    enabled: row.get::<_, i32>(1)? == 1,
                    mode: row.get::<_, Option<String>>(2)?.unwrap_or_else(|| "interval".to_string()),
                    interval_seconds: row.get::<_, i64>(3)?,
                    custom_unit: row.get::<_, Option<String>>(4)?,
                    custom_value: row.get::<_, Option<i64>>(5)?,
                    notification_enabled: row.get::<_, i32>(6)? == 1,
                    system_notification: row.get::<_, i32>(7)? == 1,
                    badge_notification: row.get::<_, i32>(8)? == 1,
                    last_run_at: row.get::<_, Option<String>>(9)?,
                    next_run_at: row.get::<_, Option<String>>(10)?,
                    new_items_count: row.get::<_, i64>(11)?,
                })
            },
        ).map_err(|e| e.to_string())
    }

    pub fn update_radar_schedule(&self, update: &RadarScheduleUpdate) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        if let Some(enabled) = update.enabled {
            conn.execute("UPDATE radar_schedule SET enabled = ?, updated_at = datetime('now', 'localtime') WHERE id = 1", rusqlite::params![if enabled { 1 } else { 0 }])?;
        }
        if let Some(ref mode) = update.mode {
            conn.execute("UPDATE radar_schedule SET mode = ?, updated_at = datetime('now', 'localtime') WHERE id = 1", rusqlite::params![mode])?;
        }
        if let Some(interval_seconds) = update.interval_seconds {
            conn.execute("UPDATE radar_schedule SET interval_seconds = ?, updated_at = datetime('now', 'localtime') WHERE id = 1", rusqlite::params![interval_seconds])?;
        }
        if let Some(ref custom_unit) = update.custom_unit {
            conn.execute("UPDATE radar_schedule SET custom_unit = ?, updated_at = datetime('now', 'localtime') WHERE id = 1", rusqlite::params![custom_unit])?;
        }
        if let Some(custom_value) = update.custom_value {
            conn.execute("UPDATE radar_schedule SET custom_value = ?, updated_at = datetime('now', 'localtime') WHERE id = 1", rusqlite::params![custom_value])?;
        }
        if let Some(notification_enabled) = update.notification_enabled {
            conn.execute("UPDATE radar_schedule SET notification_enabled = ?, updated_at = datetime('now', 'localtime') WHERE id = 1", rusqlite::params![if notification_enabled { 1 } else { 0 }])?;
        }
        if let Some(system_notification) = update.system_notification {
            conn.execute("UPDATE radar_schedule SET system_notification = ?, updated_at = datetime('now', 'localtime') WHERE id = 1", rusqlite::params![if system_notification { 1 } else { 0 }])?;
        }
        if let Some(badge_notification) = update.badge_notification {
            conn.execute("UPDATE radar_schedule SET badge_notification = ?, updated_at = datetime('now', 'localtime') WHERE id = 1", rusqlite::params![if badge_notification { 1 } else { 0 }])?;
        }
        Ok(())
    }

    pub fn update_radar_schedule_run(
        &self,
        last_run_at: &str,
        next_run_at: &str,
        new_items_count: i64,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE radar_schedule SET last_run_at = ?, next_run_at = ?, new_items_count = ?, updated_at = datetime('now', 'localtime') WHERE id = 1",
            rusqlite::params![last_run_at, next_run_at, new_items_count],
        )?;
        Ok(())
    }

    pub fn add_notification(
        &self,
        title: &str,
        body: Option<&str>,
        item_count: i64,
    ) -> Result<i64> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO radar_notifications (title, body, item_count) VALUES (?, ?, ?)",
            rusqlite::params![title, body, item_count],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn list_notifications(&self, unread_only: bool) -> Result<Vec<NotificationRow>> {
        let conn = self.conn.lock().unwrap();
        let query = if unread_only {
            "SELECT id, title, body, item_count, read, created_at FROM radar_notifications WHERE read = 0 ORDER BY created_at DESC"
        } else {
            "SELECT id, title, body, item_count, read, created_at FROM radar_notifications ORDER BY created_at DESC"
        };
        let mut stmt = conn.prepare(query)?;
        let rows = stmt.query_map([], |row| {
            Ok(NotificationRow {
                id: row.get(0)?,
                title: row.get(1)?,
                body: row.get(2)?,
                item_count: row.get(3)?,
                read: row.get::<_, i32>(4)? == 1,
                created_at: row.get(5)?,
            })
        })?;
        rows.collect::<Result<Vec<_>>>()
    }

    pub fn mark_notification_read(&self, id: i64) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE radar_notifications SET read = 1 WHERE id = ?",
            rusqlite::params![id],
        )?;
        Ok(())
    }

    pub fn clear_notifications(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM radar_notifications", [])?;
        Ok(())
    }

    pub fn get_unread_notification_count(&self) -> Result<i64, String> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT COUNT(*) FROM radar_notifications WHERE read = 0",
            [],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())
    }
}

#[derive(Debug, Clone)]
pub struct RadarScheduleRow {
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

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RadarScheduleUpdate {
    pub enabled: Option<bool>,
    pub mode: Option<String>,
    pub interval_seconds: Option<i64>,
    pub custom_unit: Option<String>,
    pub custom_value: Option<i64>,
    pub notification_enabled: Option<bool>,
    pub system_notification: Option<bool>,
    pub badge_notification: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct NotificationRow {
    pub id: i64,
    pub title: String,
    pub body: Option<String>,
    pub item_count: i64,
    pub read: bool,
    pub created_at: String,
}

#[derive(Debug, Clone)]
pub struct RadarSourceRow {
    pub id: i64,
    pub name: String,
    pub source_type: String,
    pub url: String,
    pub platform: Option<String>,
    pub check_interval: i64,
    pub proxy_enabled: bool,
    pub proxy_protocol: String,
    pub proxy_host: String,
    pub proxy_port: i32,
    pub proxy_username: String,
    pub proxy_password: String,
    pub last_checked_at: Option<String>,
    pub last_status: Option<String>,
    pub last_error: Option<String>,
}

fn get_plugin_config_db_path() -> PathBuf {
    let base_dirs = directories::BaseDirs::new().expect("Cannot determine user directories");
    let app_data = base_dirs.data_dir();
    app_data.join(".os-compass").join("plugins.db")
}

lazy_static::lazy_static! {
    pub static ref PLUGIN_CONFIG_DB: PluginConfigDb =
        PluginConfigDb::new().expect("Failed to initialize plugin config database");
}
