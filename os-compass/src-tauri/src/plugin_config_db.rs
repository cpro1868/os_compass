use crate::feature_plugin::PluginInfo;
use rusqlite::{Connection, Result};
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
        conn.execute_batch(
            "PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;"
        )?;
        
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
                db_mode: row.get::<_, Option<String>>(6)?.unwrap_or_else(|| "none".to_string()),
                db_path_template: row.get(7)?,
            })
        }).ok()
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
    
    pub fn get_enabled(&self, plugin_id: &str) -> bool {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT enabled FROM system_plugins WHERE id = ?",
            rusqlite::params![plugin_id],
            |row| row.get::<_, i32>(0),
        ).unwrap_or(0) == 1
    }
    
    pub fn get_config(&self, plugin_id: &str) -> Option<String> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT config FROM system_plugins WHERE id = ?",
            rusqlite::params![plugin_id],
            |row| row.get::<_, Option<String>>(0),
        ).ok().flatten()
    }
    
    pub fn save_config(&self, plugin_id: &str, config: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE system_plugins SET config = ?, updated_at = datetime('now', 'localtime') WHERE id = ?",
            rusqlite::params![config, plugin_id],
        )?;
        Ok(())
    }
}

fn get_plugin_config_db_path() -> PathBuf {
    let base_dirs = directories::BaseDirs::new()
        .expect("Cannot determine user directories");
    let app_data = base_dirs.data_dir();
    app_data.join(".os-compass").join("plugins.db")
}

lazy_static::lazy_static! {
    pub static ref PLUGIN_CONFIG_DB: PluginConfigDb = 
        PluginConfigDb::new().expect("Failed to initialize plugin config database");
}
