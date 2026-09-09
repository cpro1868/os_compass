use crate::plugin_config_db::PLUGIN_CONFIG_DB;
use std::path::PathBuf;

pub struct AppPaths;

impl AppPaths {
    pub fn system_dir() -> PathBuf {
        directories::BaseDirs::new()
            .map(|d| d.data_dir().join(".os-compass"))
            .unwrap_or_else(|| PathBuf::from("."))
    }

    pub fn vault_db_name() -> String {
        PLUGIN_CONFIG_DB.get_system_config("vault_db_name")
            .unwrap_or_else(|| "os_compass.db".to_string())
    }

    pub fn vault_key_name() -> String {
        PLUGIN_CONFIG_DB.get_system_config("vault_key_name")
            .unwrap_or_else(|| ".cryptokey".to_string())
    }

    pub fn vault_config_name() -> String {
        PLUGIN_CONFIG_DB.get_system_config("vault_config_name")
            .unwrap_or_else(|| "config.json".to_string())
    }

    pub fn vault_index_name() -> String {
        PLUGIN_CONFIG_DB.get_system_config("vault_index_name")
            .unwrap_or_else(|| "vault-index.json".to_string())
    }

    pub fn last_vault_name() -> String {
        PLUGIN_CONFIG_DB.get_system_config("last_vault_name")
            .unwrap_or_else(|| "last-vault.txt".to_string())
    }

    pub fn ad_filter_lexicon_name() -> String {
        PLUGIN_CONFIG_DB.get_system_config("ad_filter_lexicon")
            .unwrap_or_else(|| "ad_filter_lexicon.json".to_string())
    }

    pub fn vault_db_path(vault_dir: &PathBuf) -> PathBuf {
        vault_dir.join(Self::vault_db_name())
    }

    pub fn vault_key_path(vault_dir: &PathBuf) -> PathBuf {
        vault_dir.join(Self::vault_key_name())
    }

    pub fn vault_config_path(vault_dir: &PathBuf) -> PathBuf {
        vault_dir.join(Self::vault_config_name())
    }

    pub fn vault_index_path() -> PathBuf {
        Self::system_dir().join(Self::vault_index_name())
    }

    pub fn last_vault_path() -> PathBuf {
        Self::system_dir().join(Self::last_vault_name())
    }

    pub fn ad_filter_lexicon_path() -> PathBuf {
        Self::system_dir().join(Self::ad_filter_lexicon_name())
    }
}
