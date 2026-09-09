use crate::plugins;
use crate::system_db;
use serde::Serialize;

pub use system_db::VariableDef;

#[derive(Debug, Clone, Serialize)]
pub struct SourcePlugin {
    pub id: String,
    pub name: String,
    #[serde(rename = "pluginClass")]
    pub plugin_class: String,
    pub description: Option<String>,
    pub enabled: bool,
    pub version: Option<String>,
    #[serde(rename = "requiredVariables")]
    pub required_variables: Vec<system_db::VariableDef>,
    #[serde(rename = "urlPatterns", skip_serializing_if = "Option::is_none")]
    pub url_patterns: Option<Vec<String>>,
    #[serde(rename = "createdAt", skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    #[serde(rename = "updatedAt", skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
}

impl From<system_db::SourcePluginInfo> for SourcePlugin {
    fn from(info: system_db::SourcePluginInfo) -> Self {
        SourcePlugin {
            id: info.id,
            name: info.name,
            plugin_class: info.plugin_class,
            description: info.description,
            enabled: info.enabled,
            version: info.version,
            required_variables: info.required_variables,
            url_patterns: info.url_patterns,
            created_at: info.created_at,
            updated_at: info.updated_at,
        }
    }
}

#[tauri::command]
pub fn list_extensions() -> Result<Vec<SourcePlugin>, String> {
    let plugins = system_db::list_source_plugins()?;
    Ok(plugins.into_iter().map(SourcePlugin::from).collect())
}

#[tauri::command]
pub fn get_extension(id: String) -> Result<Option<SourcePlugin>, String> {
    match system_db::get_source_plugin(&id)? {
        Some(info) => Ok(Some(SourcePlugin::from(info))),
        None => Ok(None),
    }
}

#[tauri::command]
pub fn set_extension_enabled(id: String, enabled: bool) -> Result<(), String> {
    system_db::toggle_source_plugin(&id, enabled)
}

#[tauri::command]
pub fn get_enabled_extensions() -> Result<Vec<SourcePlugin>, String> {
    let plugins = system_db::list_source_plugins()?;
    Ok(plugins
        .into_iter()
        .filter(|p| p.enabled)
        .map(SourcePlugin::from)
        .collect())
}

#[tauri::command]
pub fn get_supported_platform_domains() -> Result<Vec<String>, String> {
    system_db::get_enabled_source_domains()
}

#[tauri::command]
pub fn get_supported_platforms() -> Result<Vec<serde_json::Value>, String> {
    Ok(plugins::get_supported_platforms()
        .into_iter()
        .map(|(domain, name)| serde_json::json!({ "domain": domain, "name": name }))
        .collect())
}
