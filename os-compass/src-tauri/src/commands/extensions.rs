use crate::db::DATABASE;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariableDef {
    pub key: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub secret: bool,
    #[serde(skip_serializing_if = "Option::is_none", rename = "default")]
    pub default: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourcePlugin {
    pub id: String,
    pub name: String,
    #[serde(rename = "pluginClass")]
    pub plugin_class: String,
    pub description: Option<String>,
    pub enabled: bool,
    pub version: Option<String>,
    #[serde(rename = "requiredVariables")]
    pub required_variables: Vec<VariableDef>,
    #[serde(rename = "createdAt", skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    #[serde(rename = "updatedAt", skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
}

#[tauri::command]
pub fn list_extensions() -> Result<Vec<SourcePlugin>, String> {
    let db = DATABASE.lock().map_err(|e| e.to_string())?;
    let db = db.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();
    let mut stmt = conn
        .prepare("SELECT id, name, plugin_class, description, enabled, version, required_variables, created_at, updated_at FROM source_plugins ORDER BY id")
        .map_err(|e| e.to_string())?;
    let plugins = stmt
        .query_map([], |row| {
            let required_variables_json: Option<String> = row.get(6)?;
            let required_variables: Vec<VariableDef> = required_variables_json
                .and_then(|json| serde_json::from_str(&json).ok())
                .unwrap_or_default();
            Ok(SourcePlugin {
                id: row.get(0)?,
                name: row.get(1)?,
                plugin_class: row.get(2)?,
                description: row.get(3)?,
                enabled: row.get::<_, i32>(4)? != 0,
                version: row.get(5)?,
                required_variables,
                created_at: row.get(7)?,
                updated_at: row.get(8)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    Ok(plugins)
}

#[tauri::command]
pub fn get_extension(id: String) -> Result<Option<SourcePlugin>, String> {
    let db = DATABASE.lock().map_err(|e| e.to_string())?;
    let db = db.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();
    let mut stmt = conn
        .prepare("SELECT id, name, plugin_class, description, enabled, version, required_variables, created_at, updated_at FROM source_plugins WHERE id = ?")
        .map_err(|e| e.to_string())?;
    let plugin = stmt
        .query_row([&id], |row| {
            let required_variables_json: Option<String> = row.get(6)?;
            let required_variables: Vec<VariableDef> = required_variables_json
                .and_then(|json| serde_json::from_str(&json).ok())
                .unwrap_or_default();
            Ok(SourcePlugin {
                id: row.get(0)?,
                name: row.get(1)?,
                plugin_class: row.get(2)?,
                description: row.get(3)?,
                enabled: row.get::<_, i32>(4)? != 0,
                version: row.get(5)?,
                required_variables,
                created_at: row.get(7)?,
                updated_at: row.get(8)?,
            })
        })
        .ok();
    Ok(plugin)
}

#[tauri::command]
pub fn set_extension_enabled(id: String, enabled: bool) -> Result<(), String> {
    let db = DATABASE.lock().map_err(|e| e.to_string())?;
    let db = db.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();
    conn.execute(
        "UPDATE source_plugins SET enabled = ?, updated_at = datetime('now', 'localtime') WHERE id = ?",
        rusqlite::params![enabled as i32, id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn get_enabled_extensions() -> Result<Vec<SourcePlugin>, String> {
    let db = DATABASE.lock().map_err(|e| e.to_string())?;
    let db = db.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();
    let mut stmt = conn
        .prepare("SELECT id, name, plugin_class, description, enabled, version, required_variables, created_at, updated_at FROM source_plugins WHERE enabled = 1 ORDER BY id")
        .map_err(|e| e.to_string())?;
    let plugins = stmt
        .query_map([], |row| {
            let required_variables_json: Option<String> = row.get(6)?;
            let required_variables: Vec<VariableDef> = required_variables_json
                .and_then(|json| serde_json::from_str(&json).ok())
                .unwrap_or_default();
            Ok(SourcePlugin {
                id: row.get(0)?,
                name: row.get(1)?,
                plugin_class: row.get(2)?,
                description: row.get(3)?,
                enabled: row.get::<_, i32>(4)? != 0,
                version: row.get(5)?,
                required_variables,
                created_at: row.get(7)?,
                updated_at: row.get(8)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    Ok(plugins)
}
