use crate::feature_plugin::PluginInfo;
use crate::plugin_config_db::PLUGIN_CONFIG_DB;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    pub author: Option<String>,
    pub description: Option<String>,
    pub entry: Option<String>,
    pub permissions: Option<Vec<String>>,
}

impl PluginManifest {
    pub fn from_file(path: &PathBuf) -> Result<Self, String> {
        let content = fs::read_to_string(path)
            .map_err(|e| format!("Failed to read manifest: {}", e))?;

        serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse manifest JSON: {}", e))
    }
}

pub struct PluginLoader {
    plugins_dir: PathBuf,
}

impl PluginLoader {
    pub fn new(app_data_dir: &PathBuf) -> Self {
        let plugins_dir = app_data_dir.join("plugins");
        PluginLoader { plugins_dir }
    }

    pub fn scan_plugins(&self) -> Result<Vec<PluginManifest>, String> {
        if !self.plugins_dir.exists() {
            fs::create_dir_all(&self.plugins_dir)
                .map_err(|e| format!("Failed to create plugins directory: {}", e))?;
            return Ok(vec![]);
        }

        let mut manifests = Vec::new();

        let entries = fs::read_dir(&self.plugins_dir)
            .map_err(|e| format!("Failed to read plugins directory: {}", e))?;

        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let manifest_path = path.join("manifest.json");
                if manifest_path.exists() {
                    match PluginManifest::from_file(&manifest_path) {
                        Ok(manifest) => {
                            manifests.push(manifest);
                        }
                        Err(e) => {
                            eprintln!("Failed to load plugin manifest {:?}: {}", manifest_path, e);
                        }
                    }
                }
            }
        }

        Ok(manifests)
    }

    pub fn load_plugin(&self, plugin_id: &str) -> Result<PluginManifest, String> {
        let plugin_dir = self.plugins_dir.join(plugin_id);
        let manifest_path = plugin_dir.join("manifest.json");

        if !manifest_path.exists() {
            return Err(format!("Plugin manifest not found: {}", manifest_path.display()));
        }

        PluginManifest::from_file(&manifest_path)
    }

    pub fn register_plugins(&self) -> Result<Vec<PluginInfo>, String> {
        let manifests = self.scan_plugins()?;
        let mut infos = Vec::new();

        for manifest in manifests {
            let info = PluginInfo {
                id: manifest.id.clone(),
                name: manifest.name.clone(),
                plugin_type: "custom".to_string(),
                version: manifest.version.clone(),
                enabled: false,
                config: None,
                db_mode: "global".to_string(),
                db_path_template: None,
            };

            PLUGIN_CONFIG_DB.register_system_plugin(&info).ok();
            infos.push(info);
        }

        Ok(infos)
    }

    pub fn uninstall_plugin(&self, plugin_id: &str) -> Result<(), String> {
        let plugin_dir = self.plugins_dir.join(plugin_id);

        if !plugin_dir.exists() {
            return Err(format!("Plugin directory not found: {}", plugin_dir.display()));
        }

        fs::remove_dir_all(&plugin_dir)
            .map_err(|e| format!("Failed to remove plugin directory: {}", e))?;

        PLUGIN_CONFIG_DB.unregister_plugin(plugin_id)?;

        Ok(())
    }

    pub fn get_plugin_dir(&self) -> &PathBuf {
        &self.plugins_dir
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_manifest_parsing() {
        let json = r#"{
            "id": "test-plugin",
            "name": "Test Plugin",
            "version": "1.0.0",
            "author": "Test Author",
            "description": "A test plugin",
            "entry": "plugin.js",
            "permissions": ["network", "storage"]
        }"#;

        let manifest: PluginManifest = serde_json::from_str(json).unwrap();
        assert_eq!(manifest.id, "test-plugin");
        assert_eq!(manifest.name, "Test Plugin");
        assert_eq!(manifest.version, "1.0.0");
    }
}
