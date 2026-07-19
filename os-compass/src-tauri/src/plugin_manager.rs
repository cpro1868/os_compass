use crate::feature_plugin::{resolve_db_path, DbMode, FeaturePlugin, PluginContext, PluginInfo};
use crate::plugin_config_db::PLUGIN_CONFIG_DB;
use crate::settings::get_settings;
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
    initialized: Mutex<bool>,
}

impl PluginManager {
    pub fn new() -> Self {
        let mut manager = PluginManager {
            registry: Mutex::new(PluginRegistry::new()),
            initialized: Mutex::new(false),
        };
        manager.registry.lock().unwrap().register(Box::new(crate::plugins::radar::RadarPlugin::new()));
        manager.registry.lock().unwrap().register(Box::new(crate::plugins::search::SearchPlugin::new()));
        manager
    }

    pub fn init(&self) {
        let mut initialized = self.initialized.lock().unwrap();
        if *initialized {
            return;
        }

        {
            let registry = self.registry.lock().unwrap();
            let plugins = registry.list();
            for plugin in plugins {
                let info = PluginInfo {
                    id: plugin.id().to_string(),
                    name: plugin.name().to_string(),
                    plugin_type: plugin.plugin_type().as_str().to_string(),
                    version: plugin.version().to_string(),
                    enabled: PLUGIN_CONFIG_DB.get_enabled(plugin.id()),
                    config: PLUGIN_CONFIG_DB.get_config(plugin.id()),
                    db_mode: plugin.db_mode().as_str().to_string(),
                    db_path_template: plugin.db_path_template().map(|s| s.to_string()),
                };
                PLUGIN_CONFIG_DB.register_system_plugin(&info).ok();
            }
        }

        *initialized = true;
    }

    pub fn list_plugins(&self) -> Result<Vec<PluginInfo>, String> {
        self.init();

        let registry = self.registry.lock().unwrap();
        let plugins: Vec<PluginInfo> = registry.list()
            .iter()
            .map(|p| {
                let enabled = PLUGIN_CONFIG_DB.get_enabled(p.id());
                let config = PLUGIN_CONFIG_DB.get_config(p.id());
                
                PluginInfo {
                    id: p.id().to_string(),
                    name: p.name().to_string(),
                    plugin_type: p.plugin_type().as_str().to_string(),
                    enabled,
                    config,
                    version: p.version().to_string(),
                    db_mode: p.db_mode().as_str().to_string(),
                    db_path_template: p.db_path_template().map(|s| s.to_string()),
                }
            })
            .collect();
        
        Ok(plugins)
    }

    pub fn get_config(&self, plugin_id: &str) -> Result<Option<String>, String> {
        Ok(PLUGIN_CONFIG_DB.get_config(plugin_id))
    }

    pub fn save_config(&self, plugin_id: &str, config: &str) -> Result<(), String> {
        PLUGIN_CONFIG_DB.save_config(plugin_id, config)
            .map_err(|e| e.to_string())
    }

    pub fn set_enabled(&self, plugin_id: &str, enabled: bool) -> Result<(), String> {
        PLUGIN_CONFIG_DB.set_enabled(plugin_id, enabled)
            .map_err(|e| e.to_string())
    }

    pub fn is_enabled(&self, plugin_id: &str) -> Result<bool, String> {
        let enabled = PLUGIN_CONFIG_DB.get_enabled(plugin_id);
        println!("[plugin_manager] is_enabled({}) = {}", plugin_id, enabled);
        Ok(enabled)
    }

    pub fn build_context(&self, plugin: &dyn FeaturePlugin, vault_dir: &PathBuf, app_data_dir: &PathBuf) -> PluginContext {
        let config = PLUGIN_CONFIG_DB.get_config(plugin.id())
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

    pub fn switch_vault(&self, new_vault_dir: &PathBuf) -> Result<(), String> {
        use crate::feature_plugin::FeaturePlugin;
        
        println!("[plugin_manager] switch_vault to {:?}", new_vault_dir);
        
        let app_data_dir = directories::BaseDirs::new()
            .map(|d| d.data_dir().to_path_buf())
            .unwrap_or_else(|| PathBuf::from("."));
        
        let registry = self.registry.lock().unwrap();
        let plugins = registry.list();
        
        for plugin in plugins {
            let plugin_id = plugin.id();
            let enabled = PLUGIN_CONFIG_DB.get_enabled(plugin_id);
            println!("[plugin_manager] processing plugin: {}, enabled: {}", plugin_id, enabled);
            
            let context = self.build_context(plugin.as_ref(), new_vault_dir, &app_data_dir);
            
            let _ = plugin.on_disable(&context);
            
            if let Err(e) = tauri::async_runtime::block_on(plugin.init(&context)) {
                println!("[plugin_manager] plugin {} init error: {}", plugin_id, e);
            }
            
            if enabled {
                if let Err(e) = tauri::async_runtime::block_on(plugin.on_enable(&context)) {
                    println!("[plugin_manager] plugin {} on_enable error: {}", plugin_id, e);
                }
            }
        }
        
        Ok(())
    }
}

lazy_static::lazy_static! {
    pub static ref PLUGIN_MANAGER: PluginManager = PluginManager::new();
}
