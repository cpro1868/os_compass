use crate::plugin_loader::{PluginLoader, PluginManifest};
use crate::plugin_config_db::PLUGIN_CONFIG_DB;
use std::path::PathBuf;

#[derive(serde::Serialize)]
pub struct LocalPlugin {
    pub id: String,
    pub name: String,
    pub version: String,
    pub author: Option<String>,
    pub description: Option<String>,
    pub entry: Option<String>,
    pub permissions: Vec<String>,
    pub enabled: bool,
    pub installed: bool,
}

fn get_app_data_dir() -> PathBuf {
    let base_dirs = directories::BaseDirs::new()
        .expect("Cannot determine user directories");
    base_dirs.data_dir().join(".os-compass")
}

#[tauri::command]
pub fn list_local_plugins() -> Result<Vec<LocalPlugin>, String> {
    let loader = PluginLoader::new(&get_app_data_dir());
    let manifests = loader.scan_plugins()?;

    let system_plugins = PLUGIN_CONFIG_DB.list_system_plugins();
    let system_ids: Vec<String> = system_plugins.iter().map(|p| p.id.clone()).collect();

    let plugins: Vec<LocalPlugin> = manifests.into_iter().map(|m| {
        let enabled = system_plugins.iter()
            .find(|p| p.id == m.id)
            .map(|p| p.enabled)
            .unwrap_or(false);

        LocalPlugin {
            id: m.id,
            name: m.name,
            version: m.version,
            author: m.author,
            description: m.description,
            entry: m.entry,
            permissions: m.permissions.unwrap_or_default(),
            enabled,
            installed: true,
        }
    }).collect();

    Ok(plugins)
}

#[tauri::command]
pub fn get_local_plugin_dir() -> Result<String, String> {
    let loader = PluginLoader::new(&get_app_data_dir());
    Ok(loader.get_plugin_dir().to_string_lossy().to_string())
}

#[tauri::command]
pub fn uninstall_local_plugin(plugin_id: String) -> Result<(), String> {
    let loader = PluginLoader::new(&get_app_data_dir());
    loader.uninstall_plugin(&plugin_id)
}
