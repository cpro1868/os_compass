use crate::feature_plugin::{resolve_db_path, DbMode, PluginInfo};
use crate::plugin_manager::PLUGIN_MANAGER;
use tauri::command;

#[command]
pub fn list_feature_plugins() -> Result<Vec<PluginInfo>, String> {
    PLUGIN_MANAGER.list_plugins()
}

#[command]
pub fn set_plugin_enabled(plugin_id: String, enabled: bool) -> Result<(), String> {
    if plugin_id.is_empty() {
        return Err("Plugin ID cannot be empty".to_string());
    }
    PLUGIN_MANAGER.set_enabled(&plugin_id, enabled)
}

#[command]
pub fn get_plugin_config(plugin_id: String) -> Result<Option<String>, String> {
    if plugin_id.is_empty() {
        return Err("Plugin ID cannot be empty".to_string());
    }
    PLUGIN_MANAGER.get_config(&plugin_id)
}

#[command]
pub fn save_plugin_config(plugin_id: String, config: String) -> Result<(), String> {
    if plugin_id.is_empty() {
        return Err("Plugin ID cannot be empty".to_string());
    }
    PLUGIN_MANAGER.save_config(&plugin_id, &config)
}

#[command]
pub fn plugin_get_db_path(
    plugin_id: String,
    vault_dir: String,
    app_data_dir: String,
) -> Result<String, String> {
    if plugin_id.is_empty() {
        return Err("Plugin ID cannot be empty".to_string());
    }
    let plugins = PLUGIN_MANAGER.list_plugins()?;
    let plugin = plugins.iter().find(|p| p.id == plugin_id)
        .ok_or_else(|| format!("Plugin not found: {}", plugin_id))?;

    let db_mode = DbMode::from_str(&plugin.db_mode)
        .ok_or_else(|| format!("Invalid db_mode: {}", plugin.db_mode))?;

    let db_path = resolve_db_path(
        db_mode,
        plugin.db_path_template.as_deref(),
        &std::path::PathBuf::from(vault_dir),
        &std::path::PathBuf::from(app_data_dir),
        &plugin_id,
    );

    Ok(db_path.map(|p| p.to_string_lossy().to_string()).unwrap_or_default())
}
