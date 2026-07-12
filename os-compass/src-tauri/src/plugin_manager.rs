use crate::db::DATABASE;
use crate::feature_plugin::{resolve_db_path, DbMode, FeaturePlugin, PluginContext, PluginInfo};
use crate::settings::get_settings;
use rusqlite::params;
use std::path::PathBuf;
use std::sync::Mutex;

pub struct PluginRegistry {
    plugins: Vec<Box<dyn FeaturePlugin>>,
}

impl PluginRegistry {
    pub fn new() -> Self {
        PluginRegistry {
            plugins: Vec::new(),
        }
    }

    pub fn register(&mut self, plugin: Box<dyn FeaturePlugin>) {
        self.plugins.push(plugin);
    }

    pub fn find(&self, plugin_id: &str) -> Option<&Box<dyn FeaturePlugin>> {
        self.plugins.iter().find(|p| p.id() == plugin_id)
    }

    pub fn list(&self) -> &Vec<Box<dyn FeaturePlugin>> {
        &self.plugins
    }
}

pub struct PluginManager {
    registry: Mutex<PluginRegistry>,
}

impl PluginManager {
    pub fn new() -> Self {
        let mut manager = PluginManager {
            registry: Mutex::new(PluginRegistry::new()),
        };
        manager.registry.lock().unwrap().register(Box::new(crate::plugins::radar::RadarPlugin::new()));
        manager.registry.lock().unwrap().register(Box::new(crate::plugins::search::SearchPlugin::new()));
        manager
    }

    pub fn list_plugins(&self) -> Result<Vec<PluginInfo>, String> {
        let db_guard = DATABASE.lock().unwrap();
        let db = db_guard.as_ref().ok_or("Database not initialized")?;
        let conn = db.get_connection();

        let mut stmt = conn.prepare(
            "SELECT id, name, plugin_type, enabled, config, version, db_mode, db_path_template FROM feature_plugins"
        ).map_err(|e| e.to_string())?;

        let rows = stmt.query_map([], |row| {
            Ok(PluginInfo {
                id: row.get(0)?,
                name: row.get(1)?,
                plugin_type: row.get(2)?,
                enabled: row.get::<_, i32>(3)? == 1,
                config: row.get(4)?,
                version: row.get(5)?,
                db_mode: row.get::<_, Option<String>>(6)?.unwrap_or_else(|| "none".to_string()),
                db_path_template: row.get(7)?,
            })
        }).map_err(|e| e.to_string())?;

        let mut plugins = Vec::new();
        for row in rows {
            plugins.push(row.map_err(|e| e.to_string())?);
        }

        Ok(plugins)
    }

    pub fn get_config(&self, plugin_id: &str) -> Result<Option<String>, String> {
        let db_guard = DATABASE.lock().unwrap();
        let db = db_guard.as_ref().ok_or("Database not initialized")?;
        let conn = db.get_connection();

        let config: Option<String> = conn.query_row(
            "SELECT config FROM feature_plugins WHERE id = ?",
            params![plugin_id],
            |row| row.get(0),
        ).map_err(|e| e.to_string())?;

        Ok(config)
    }

    pub fn save_config(&self, plugin_id: &str, config: &str) -> Result<(), String> {
        let db_guard = DATABASE.lock().unwrap();
        let db = db_guard.as_ref().ok_or("Database not initialized")?;
        let conn = db.get_connection();

        conn.execute(
            "UPDATE feature_plugins SET config = ?, updated_at = datetime('now', 'localtime') WHERE id = ?",
            params![config, plugin_id],
        ).map_err(|e| e.to_string())?;

        Ok(())
    }

    pub fn set_enabled(&self, plugin_id: &str, enabled: bool) -> Result<(), String> {
        let db_guard = DATABASE.lock().unwrap();
        let db = db_guard.as_ref().ok_or("Database not initialized")?;
        let conn = db.get_connection();

        conn.execute(
            "UPDATE feature_plugins SET enabled = ?, updated_at = datetime('now', 'localtime') WHERE id = ?",
            params![if enabled { 1 } else { 0 }, plugin_id],
        ).map_err(|e| e.to_string())?;

        Ok(())
    }

    pub fn is_enabled(&self, plugin_id: &str) -> Result<bool, String> {
        let db_guard = DATABASE.lock().unwrap();
        let db = db_guard.as_ref().ok_or("Database not initialized")?;
        let conn = db.get_connection();

        let enabled: i32 = conn.query_row(
            "SELECT enabled FROM feature_plugins WHERE id = ?",
            params![plugin_id],
            |row| row.get(0),
        ).map_err(|e| e.to_string())?;

        Ok(enabled == 1)
    }

    pub fn build_context(&self, plugin: &dyn FeaturePlugin, vault_dir: &PathBuf, app_data_dir: &PathBuf) -> PluginContext {
        let config_str = self.get_config(plugin.id()).unwrap_or(None);
        let config = config_str
            .and_then(|c| serde_json::from_str(&c).ok())
            .unwrap_or(serde_json::Value::Null);

        let settings = get_settings();
        let llm_config = serde_json::json!({
            "llm_provider": settings.llm_provider,
            "llm_api_base": settings.llm_api_base,
            "llm_api_key": settings.llm_api_key,
            "llm_model": settings.llm_model,
        });

        let db_mode = plugin.db_mode();
        let db_path_template = plugin.db_path_template();
        let plugin_db_path = resolve_db_path(db_mode, db_path_template, vault_dir, app_data_dir, plugin.id());

        PluginContext::new(
            vault_dir.clone(),
            app_data_dir.clone(),
            config,
            llm_config,
            plugin_db_path,
        )
    }
}

lazy_static::lazy_static! {
    pub static ref PLUGIN_MANAGER: PluginManager = PluginManager::new();
}
