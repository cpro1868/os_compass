use rusqlite::{Connection, Result};
use std::path::PathBuf;
use std::sync::Mutex;

pub type MutexGuard<'a, T> = std::sync::MutexGuard<'a, T>;

pub struct SystemDb {
    conn: Mutex<Connection>,
}

impl SystemDb {
    pub fn new(path: PathBuf) -> Result<Self> {
        let conn = Connection::open(&path)?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    pub fn init_schema(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS app_settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL,
                is_secret INTEGER DEFAULT 0,
                updated_at TEXT DEFAULT (datetime('now', 'localtime'))
            );

            CREATE TABLE IF NOT EXISTS system_variables (
                key TEXT PRIMARY KEY,
                encrypted_value TEXT NOT NULL,
                is_secret INTEGER DEFAULT 0,
                created_at TEXT DEFAULT (datetime('now', 'localtime')),
                updated_at TEXT DEFAULT (datetime('now', 'localtime'))
            );

            CREATE TABLE IF NOT EXISTS system_migration_info (
                key TEXT PRIMARY KEY,
                version TEXT NOT NULL,
                migrated_at TEXT DEFAULT (datetime('now', 'localtime')),
                backup_path TEXT
            );

            CREATE TABLE IF NOT EXISTS source_plugins (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                plugin_class TEXT NOT NULL,
                description TEXT,
                enabled INTEGER DEFAULT 1,
                version TEXT,
                required_variables TEXT,
                url_patterns TEXT,
                created_at TEXT DEFAULT (datetime('now', 'localtime')),
                updated_at TEXT DEFAULT (datetime('now', 'localtime'))
            );

            CREATE INDEX IF NOT EXISTS idx_plugins_enabled ON source_plugins(enabled);
            "#,
        )?;

        let _: Result<usize, _> = conn.execute(
            "ALTER TABLE app_settings ADD COLUMN is_secret INTEGER DEFAULT 0",
            [],
        );

        conn.execute(
            r#"
            INSERT OR IGNORE INTO source_plugins (id, name, plugin_class, description, enabled, version, required_variables, url_patterns) VALUES
            ('github', 'GitHub', 'plugins::GitHubPlugin', '获取 GitHub 项目信息、健康度评分', 1, '1.0.0',
             '[{"key": "github_token", "name": "GitHub Token", "description": "GitHub Personal Access Token，用于访问 GitHub API","secret": true}, {"key": "github_proxy", "name": "GitHub 代理地址", "description": "访问 GitHub API 使用的代理地址","secret": false}]',
             '["github.com", "www.github.com"]'),
            ('gitee', 'Gitee', 'plugins::GiteePlugin', '获取 Gitee 项目信息、健康度评分', 1, '1.0.0',
             '[{"key": "gitee_token", "name": "Gitee 私有令牌", "description": "Gitee 私有令牌，用于访问 Gitee API","secret": true}]',
             '["gitee.com", "www.gitee.com"]'),
            ('crawler', '通用爬虫', 'plugins::CrawlerPlugin', '无 Token 时的降级方案', 1, '1.0.0', '[]', '[]')
            "#,
            [],
        )?;

        conn.execute(
            r#"
            INSERT OR IGNORE INTO system_variables (key, encrypted_value, is_secret) VALUES
            ('github_token', '', 1),
            ('github_proxy', '', 0),
            ('gitee_token', '', 1)
            "#,
            [],
        )?;

        Ok(())
    }

    pub fn get_connection(&self) -> std::sync::MutexGuard<'_, Connection> {
        self.conn.lock().unwrap()
    }
}

lazy_static::lazy_static! {
    pub static ref SYSTEM_DB: Mutex<Option<SystemDb>> = Mutex::new(None);
}

pub fn get_system_settings_path(app_data_dir: &PathBuf) -> PathBuf {
    app_data_dir.join("system_settings.db")
}

pub fn get_system_vars_path(app_data_dir: &PathBuf) -> PathBuf {
    app_data_dir.join("system_vars.db")
}

pub fn init_system_db(app_data_dir: &PathBuf) -> Result<(), String> {
    let settings_path = get_system_settings_path(app_data_dir);
    let _vars_path = get_system_vars_path(app_data_dir);

    let system_db = SystemDb::new(settings_path.clone()).map_err(|e| e.to_string())?;
    system_db.init_schema().map_err(|e| e.to_string())?;

    let mut db_lock = SYSTEM_DB.lock().map_err(|e| e.to_string())?;
    *db_lock = Some(system_db);

    println!("[system_db] System settings DB initialized at {:?}", settings_path);

    Ok(())
}

pub fn is_migration_needed() -> bool {
    let db_lock = match SYSTEM_DB.lock() {
        Ok(lock) => lock,
        Err(_) => return false,
    };

    let db = match db_lock.as_ref() {
        Some(db) => db,
        None => return false,
    };

    let conn = db.get_connection();

    let count: i32 = conn
        .query_row("SELECT COUNT(*) FROM system_migration_info WHERE key = 'v2_migration'", [], |row| row.get(0))
        .unwrap_or(0);

    count == 0
}

pub fn mark_migration_completed(backup_path: Option<&str>) -> Result<(), String> {
    let db_lock = SYSTEM_DB.lock().map_err(|e| e.to_string())?;
    let db = db_lock.as_ref().ok_or("System DB not initialized")?;
    let conn = db.get_connection();

    conn.execute(
        "INSERT OR REPLACE INTO system_migration_info (key, version, migrated_at, backup_path) VALUES ('v2_migration', '2.0', datetime('now', 'localtime'), ?)",
        rusqlite::params![backup_path],
    )
    .map_err(|e| e.to_string())?;

    println!("[system_db] Migration marked as completed");
    Ok(())
}

pub fn get_system_setting(key: &str) -> Option<String> {
    let db_lock = match SYSTEM_DB.lock() {
        Ok(lock) => lock,
        Err(_) => return None,
    };

    let db = match db_lock.as_ref() {
        Some(db) => db,
        None => return None,
    };

    let conn = db.get_connection();

    let has_is_secret = {
        let mut stmt = conn.prepare("PRAGMA table_info(app_settings)").ok()?;
        let mut rows = stmt.query([]).ok()?;
        let mut found = false;
        while let Some(row) = rows.next().ok()? {
            let name: String = row.get(1).ok()?;
            if name == "is_secret" {
                found = true;
                break;
            }
        }
        found
    };

    if has_is_secret {
        let result: (String, i32) = conn
            .query_row(
                "SELECT value, is_secret FROM app_settings WHERE key = ?",
                [key],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .ok()?;

        let (value, is_secret) = result;
        if is_secret != 0 {
            match crate::crypto::decrypt_string(&value) {
                Ok(plaintext) => Some(plaintext),
                Err(e) => {
                    println!("[system_db] WARNING: Cannot decrypt key='{}': {}.", key, e);
                    None
                }
            }
        } else {
            Some(value)
        }
    } else {
        let value: String = conn
            .query_row(
                "SELECT value FROM app_settings WHERE key = ?",
                [key],
                |row| row.get(0),
            )
            .ok()?;
        Some(value)
    }
}

pub fn set_system_setting(key: &str, value: &str, is_secret: bool) -> Result<(), String> {
    let db_lock = SYSTEM_DB.lock().map_err(|e| e.to_string())?;
    let db = db_lock.as_ref().ok_or("System DB not initialized")?;
    let conn = db.get_connection();

    let has_is_secret = {
        let mut stmt = conn.prepare("PRAGMA table_info(app_settings)").map_err(|e| e.to_string())?;
        let mut rows = stmt.query([]).map_err(|e| e.to_string())?;
        let mut found = false;
        while let Some(row) = rows.next().map_err(|e| e.to_string())? {
            let name: String = row.get(1).map_err(|e| e.to_string())?;
            if name == "is_secret" {
                found = true;
                break;
            }
        }
        found
    };

    if !has_is_secret {
        conn.execute("ALTER TABLE app_settings ADD COLUMN is_secret INTEGER DEFAULT 0", [])
            .map_err(|e| e.to_string())?;
    }

    let stored_value = if is_secret {
        crate::crypto::encrypt_string(value)?
    } else {
        value.to_string()
    };

    conn.execute(
        "INSERT INTO app_settings (key, value, is_secret, updated_at) VALUES (?, ?, ?, datetime('now', 'localtime'))
         ON CONFLICT(key) DO UPDATE SET value = excluded.value, is_secret = excluded.is_secret, updated_at = datetime('now', 'localtime')",
        rusqlite::params![key, stored_value, is_secret as i32],
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

pub fn get_all_system_variables() -> Result<Vec<(String, Option<String>, bool)>, String> {
    let db_lock = SYSTEM_DB.lock().map_err(|e| e.to_string())?;
    let db = db_lock.as_ref().ok_or("System DB not initialized")?;
    let conn = db.get_connection();

    let mut stmt = conn
        .prepare("SELECT key, encrypted_value, is_secret FROM system_variables ORDER BY key")
        .map_err(|e| e.to_string())?;

    let mut result = Vec::new();
    let mut rows = stmt.query([]).map_err(|e| e.to_string())?;

    while let Some(row) = rows.next().map_err(|e| e.to_string())? {
        let key: String = row.get(0).map_err(|e| e.to_string())?;
        let encrypted_value: String = row.get(1).map_err(|e| e.to_string())?;
        let is_secret: bool = row.get::<_, i32>(2).map_err(|e| e.to_string())? != 0;

        let value = if is_secret {
            match crate::crypto::decrypt_string(&encrypted_value) {
                Ok(plain) => Some(plain),
                Err(e) => {
                    println!("[system_db] WARNING: Cannot decrypt variable key='{}': {}", key, e);
                    Some(String::new())
                }
            }
        } else {
            Some(encrypted_value)
        };

        result.push((key, value, is_secret));
    }

    Ok(result)
}

pub fn is_variable_secret(key: &str) -> Option<bool> {
    let db_lock = SYSTEM_DB.lock().ok()?;
    let db = db_lock.as_ref()?;
    let conn = db.get_connection();

    let is_secret: i32 = conn
        .query_row(
            "SELECT is_secret FROM system_variables WHERE key = ?",
            [key],
            |row| row.get(0),
        )
        .ok()?;

    Some(is_secret != 0)
}

pub fn get_system_variable(key: &str) -> Option<String> {
    let db_lock = match SYSTEM_DB.lock() {
        Ok(lock) => lock,
        Err(_) => return None,
    };

    let db = match db_lock.as_ref() {
        Some(db) => db,
        None => return None,
    };

    let conn = db.get_connection();

    let result: String = conn
        .query_row(
            "SELECT encrypted_value FROM system_variables WHERE key = ?",
            [key],
            |row| row.get(0),
        )
        .ok()?;

    match crate::crypto::decrypt_string(&result) {
        Ok(plaintext) => Some(plaintext),
        Err(e) => {
            println!("[system_db] WARNING: Cannot decrypt variable key='{}': {}.", key, e);
            None
        }
    }
}

pub fn set_system_variable(key: &str, value: &str, is_secret: bool) -> Result<(), String> {
    let db_lock = SYSTEM_DB.lock().map_err(|e| e.to_string())?;
    let db = db_lock.as_ref().ok_or("System DB not initialized")?;
    let conn = db.get_connection();

    let encrypted_value = if is_secret {
        crate::crypto::encrypt_string(value)?
    } else {
        value.to_string()
    };

    conn.execute(
        "INSERT INTO system_variables (key, encrypted_value, is_secret, updated_at) VALUES (?, ?, ?, datetime('now', 'localtime'))
         ON CONFLICT(key) DO UPDATE SET encrypted_value = excluded.encrypted_value, is_secret = excluded.is_secret, updated_at = datetime('now', 'localtime')",
        rusqlite::params![key, encrypted_value, is_secret as i32],
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

pub fn delete_system_variable(key: &str) -> Result<(), String> {
    let db_lock = SYSTEM_DB.lock().map_err(|e| e.to_string())?;
    let db = db_lock.as_ref().ok_or("System DB not initialized")?;
    let conn = db.get_connection();

    conn.execute("DELETE FROM system_variables WHERE key = ?", [key])
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct SourcePluginInfo {
    pub id: String,
    pub name: String,
    pub plugin_class: String,
    pub description: Option<String>,
    pub enabled: bool,
    pub version: Option<String>,
    pub required_variables: Vec<VariableDef>,
    pub url_patterns: Option<Vec<String>>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct VariableDef {
    pub key: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub secret: bool,
    #[serde(skip_serializing_if = "Option::is_none", rename = "default")]
    pub default: Option<String>,
}

pub fn list_source_plugins() -> Result<Vec<SourcePluginInfo>, String> {
    let db_lock = SYSTEM_DB.lock().map_err(|e| e.to_string())?;
    let db = db_lock.as_ref().ok_or("System DB not initialized")?;
    let conn = db.get_connection();

    let mut stmt = conn
        .prepare("SELECT id, name, plugin_class, description, enabled, version, required_variables, url_patterns, created_at, updated_at FROM source_plugins ORDER BY id")
        .map_err(|e| e.to_string())?;

    let plugins = stmt
        .query_map([], |row| {
            let required_variables_json: Option<String> = row.get(6)?;
            let required_variables: Vec<VariableDef> = required_variables_json
                .and_then(|json| serde_json::from_str(&json).ok())
                .unwrap_or_default();
            let url_patterns_json: Option<String> = row.get(7)?;
            let url_patterns: Option<Vec<String>> = url_patterns_json
                .and_then(|json| serde_json::from_str(&json).ok());
            Ok(SourcePluginInfo {
                id: row.get(0)?,
                name: row.get(1)?,
                plugin_class: row.get(2)?,
                description: row.get(3)?,
                enabled: row.get::<_, i32>(4)? != 0,
                version: row.get(5)?,
                required_variables,
                url_patterns,
                created_at: row.get(8)?,
                updated_at: row.get(9)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(plugins)
}

pub fn get_source_plugin(id: &str) -> Result<Option<SourcePluginInfo>, String> {
    let db_lock = SYSTEM_DB.lock().map_err(|e| e.to_string())?;
    let db = db_lock.as_ref().ok_or("System DB not initialized")?;
    let conn = db.get_connection();

    let mut stmt = conn
        .prepare("SELECT id, name, plugin_class, description, enabled, version, required_variables, url_patterns, created_at, updated_at FROM source_plugins WHERE id = ?")
        .map_err(|e| e.to_string())?;

    let plugin = stmt
        .query_row([id], |row| {
            let required_variables_json: Option<String> = row.get(6)?;
            let required_variables: Vec<VariableDef> = required_variables_json
                .and_then(|json| serde_json::from_str(&json).ok())
                .unwrap_or_default();
            let url_patterns_json: Option<String> = row.get(7)?;
            let url_patterns: Option<Vec<String>> = url_patterns_json
                .and_then(|json| serde_json::from_str(&json).ok());
            Ok(SourcePluginInfo {
                id: row.get(0)?,
                name: row.get(1)?,
                plugin_class: row.get(2)?,
                description: row.get(3)?,
                enabled: row.get::<_, i32>(4)? != 0,
                version: row.get(5)?,
                required_variables,
                url_patterns,
                created_at: row.get(8)?,
                updated_at: row.get(9)?,
            })
        })
        .ok();

    Ok(plugin)
}

pub fn update_source_plugin(id: &str, name: &str, description: Option<&str>, enabled: bool, required_variables: &str, url_patterns: Option<&str>) -> Result<(), String> {
    let db_lock = SYSTEM_DB.lock().map_err(|e| e.to_string())?;
    let db = db_lock.as_ref().ok_or("System DB not initialized")?;
    let conn = db.get_connection();

    conn.execute(
        "UPDATE source_plugins SET name = ?, description = ?, enabled = ?, required_variables = ?, url_patterns = ?, updated_at = datetime('now', 'localtime') WHERE id = ?",
        rusqlite::params![name, description, enabled as i32, required_variables, url_patterns, id],
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

pub fn toggle_source_plugin(id: &str, enabled: bool) -> Result<(), String> {
    let db_lock = SYSTEM_DB.lock().map_err(|e| e.to_string())?;
    let db = db_lock.as_ref().ok_or("System DB not initialized")?;
    let conn = db.get_connection();

    conn.execute(
        "UPDATE source_plugins SET enabled = ?, updated_at = datetime('now', 'localtime') WHERE id = ?",
        rusqlite::params![enabled as i32, id],
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}
