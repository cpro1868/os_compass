use crate::db::DATABASE;
use crate::system_db;
use crate::vault::VaultConfig;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::Manager;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub data_dir: String,
    pub theme: String,
    #[serde(rename = "defaultLanguage")]
    pub default_language: String,
    #[serde(rename = "autoTranslateReadme")]
    pub auto_translate_readme: bool,
    #[serde(rename = "autoTranslateReport")]
    pub auto_translate_report: bool,
    pub download_path: String,
    pub default_editor: String,
    pub llm_provider: String,
    pub llm_api_base: String,
    pub llm_api_key: String,
    pub llm_model: String,
    pub llm_proxy_enabled: bool,
    pub llm_proxy_protocol: String,
    pub llm_proxy_host: String,
    pub llm_proxy_port: i32,
    pub llm_proxy_username: String,
    pub llm_proxy_password: String,
    pub proxy_enabled: bool,
    pub proxy_protocol: String,
    pub proxy_host: String,
    pub proxy_port: i32,
    pub proxy_username: String,
    pub proxy_password: String,
    pub crawler_enabled: bool,
    pub crawler_api_url: String,
    pub readme_update_frequency: String,
    pub translate_engine: String,
    pub google_api_key: String,
    pub google_proxy_enabled: bool,
    pub google_proxy_protocol: String,
    pub google_proxy_host: String,
    pub google_proxy_port: i32,
    pub google_proxy_username: String,
    pub google_proxy_password: String,
}

impl Default for AppSettings {
    fn default() -> Self {
        let download_path = directories::UserDirs::new()
            .and_then(|d| d.download_dir().map(|p| p.to_string_lossy().to_string()))
            .unwrap_or_default();
        Self {
            data_dir: String::new(),
            theme: "dark".to_string(),
            default_language: "zh-CN".to_string(),
            auto_translate_readme: true,
            auto_translate_report: true,
            download_path,
            default_editor: "code".to_string(),
            llm_provider: "openai".to_string(),
            llm_api_base: "https://api.openai.com/v1".to_string(),
            llm_api_key: String::new(),
            llm_model: "gpt-4o-mini".to_string(),
            llm_proxy_enabled: false,
            llm_proxy_protocol: "http".to_string(),
            llm_proxy_host: String::new(),
            llm_proxy_port: 0,
            llm_proxy_username: String::new(),
            llm_proxy_password: String::new(),
            proxy_enabled: false,
            proxy_protocol: "http".to_string(),
            proxy_host: String::new(),
            proxy_port: 0,
            proxy_username: String::new(),
            proxy_password: String::new(),
            crawler_enabled: true,
            crawler_api_url: String::new(),
            readme_update_frequency: "smart".to_string(),
            translate_engine: "auto".to_string(),
            google_api_key: String::new(),
            google_proxy_enabled: false,
            google_proxy_protocol: "http".to_string(),
            google_proxy_host: String::new(),
            google_proxy_port: 0,
            google_proxy_username: String::new(),
            google_proxy_password: String::new(),
        }
    }
}

impl From<VaultConfig> for AppSettings {
    fn from(_config: VaultConfig) -> Self {
        Self::default()
    }
}

impl From<&AppSettings> for VaultConfig {
    fn from(_settings: &AppSettings) -> Self {
        Self {
            path: String::new(),
        }
    }
}

pub fn get_settings() -> AppSettings {
    let mut settings = AppSettings::default();

    let get = |key: &str| -> Option<String> { system_db::get_system_setting(key) };
    let get_bool = |key: &str, default: bool| -> bool {
        get(key).map(|v| v == "1" || v.to_lowercase() == "true").unwrap_or(default)
    };
    let get_i32 = |key: &str, default: i32| -> i32 {
        get(key).and_then(|v| v.parse().ok()).unwrap_or(default)
    };

    if let Some(v) = get("settings.theme") {
        if !v.is_empty() { settings.theme = v; }
    }
    if let Some(v) = get("settings.default_language") {
        if !v.is_empty() { settings.default_language = v; }
    }
    if let Some(v) = get("settings.auto_translate_readme") {
        settings.auto_translate_readme = v == "1" || v.to_lowercase() == "true";
    }
    if let Some(v) = get("settings.auto_translate_report") {
        settings.auto_translate_report = v == "1" || v.to_lowercase() == "true";
    }
    if let Some(dp) = get("settings.download_path") {
        if !dp.is_empty() { settings.download_path = dp; }
    }
    if let Some(v) = get("settings.default_editor") {
        if !v.is_empty() { settings.default_editor = v; }
    }
    if let Some(v) = get("settings.llm_provider") {
        if !v.is_empty() { settings.llm_provider = v; }
    }
    if let Some(v) = get("settings.llm_api_base") {
        if !v.is_empty() { settings.llm_api_base = v; }
    }
    if let Some(v) = get("settings.llm_api_key") {
        if !v.is_empty() { settings.llm_api_key = v; }
    }
    if let Some(v) = get("settings.llm_model") {
        if !v.is_empty() { settings.llm_model = v; }
    }
    settings.llm_proxy_enabled = get_bool("settings.llm_proxy_enabled", false);
    if let Some(v) = get("settings.llm_proxy_protocol") {
        if !v.is_empty() { settings.llm_proxy_protocol = v; }
    }
    if let Some(v) = get("settings.llm_proxy_host") {
        if !v.is_empty() { settings.llm_proxy_host = v; }
    }
    settings.llm_proxy_port = get_i32("settings.llm_proxy_port", 0);
    if let Some(v) = get("settings.llm_proxy_username") {
        if !v.is_empty() { settings.llm_proxy_username = v; }
    }
    if let Some(v) = get("settings.llm_proxy_password") {
        if !v.is_empty() { settings.llm_proxy_password = v; }
    }
    settings.proxy_enabled = get_bool("settings.proxy_enabled", false);
    if let Some(v) = get("settings.proxy_protocol") {
        if !v.is_empty() { settings.proxy_protocol = v; }
    }
    if let Some(v) = get("settings.proxy_host") {
        if !v.is_empty() { settings.proxy_host = v; }
    }
    settings.proxy_port = get_i32("settings.proxy_port", 0);
    if let Some(v) = get("settings.proxy_username") {
        if !v.is_empty() { settings.proxy_username = v; }
    }
    if let Some(v) = get("settings.proxy_password") {
        if !v.is_empty() { settings.proxy_password = v; }
    }
    settings.crawler_enabled = get_bool("settings.crawler_enabled", true);
    if let Some(v) = get("settings.crawler_api_url") {
        if !v.is_empty() { settings.crawler_api_url = v; }
    }
    if let Some(v) = get("settings.readme_update_frequency") {
        if !v.is_empty() { settings.readme_update_frequency = v; }
    }
    if let Some(v) = get("settings.translate_engine") {
        if !v.is_empty() { settings.translate_engine = v; }
    }
    if let Some(v) = get("settings.google_api_key") {
        if !v.is_empty() { settings.google_api_key = v; }
    }
    settings.google_proxy_enabled = get_bool("settings.google_proxy_enabled", false);
    if let Some(v) = get("settings.google_proxy_protocol") {
        if !v.is_empty() { settings.google_proxy_protocol = v; }
    }
    if let Some(v) = get("settings.google_proxy_host") {
        if !v.is_empty() { settings.google_proxy_host = v; }
    }
    settings.google_proxy_port = get_i32("settings.google_proxy_port", 0);
    if let Some(v) = get("settings.google_proxy_username") {
        if !v.is_empty() { settings.google_proxy_username = v; }
    }
    if let Some(v) = get("settings.google_proxy_password") {
        if !v.is_empty() { settings.google_proxy_password = v; }
    }

    settings
}

pub fn save_settings(settings: &AppSettings) -> Result<(), String> {
    let set = |key: &str, value: &str, is_secret: bool| {
        system_db::set_system_setting(key, value, is_secret)
    };

    set("settings.theme", &settings.theme, false)?;
    set("settings.default_language", &settings.default_language, false)?;
    set("settings.auto_translate_readme", if settings.auto_translate_readme { "1" } else { "0" }, false)?;
    set("settings.auto_translate_report", if settings.auto_translate_report { "1" } else { "0" }, false)?;
    set("settings.download_path", &settings.download_path, false)?;
    set("settings.default_editor", &settings.default_editor, false)?;
    set("settings.llm_provider", &settings.llm_provider, false)?;
    set("settings.llm_api_base", &settings.llm_api_base, false)?;
    if !settings.llm_api_key.is_empty() {
        set("settings.llm_api_key", &settings.llm_api_key, true)?;
    }
    set("settings.llm_model", &settings.llm_model, false)?;
    set("settings.llm_proxy_enabled", if settings.llm_proxy_enabled { "1" } else { "0" }, false)?;
    set("settings.llm_proxy_protocol", &settings.llm_proxy_protocol, false)?;
    set("settings.llm_proxy_host", &settings.llm_proxy_host, false)?;
    set("settings.llm_proxy_port", &settings.llm_proxy_port.to_string(), false)?;
    set("settings.llm_proxy_username", &settings.llm_proxy_username, false)?;
    set("settings.llm_proxy_password", &settings.llm_proxy_password, false)?;
    set("settings.proxy_enabled", if settings.proxy_enabled { "1" } else { "0" }, false)?;
    set("settings.proxy_protocol", &settings.proxy_protocol, false)?;
    set("settings.proxy_host", &settings.proxy_host, false)?;
    set("settings.proxy_port", &settings.proxy_port.to_string(), false)?;
    set("settings.proxy_username", &settings.proxy_username, false)?;
    set("settings.proxy_password", &settings.proxy_password, false)?;
    set("settings.crawler_enabled", if settings.crawler_enabled { "1" } else { "0" }, false)?;
    set("settings.crawler_api_url", &settings.crawler_api_url, false)?;
    set("settings.readme_update_frequency", &settings.readme_update_frequency, false)?;
    set("settings.translate_engine", &settings.translate_engine, false)?;
    if !settings.google_api_key.is_empty() {
        set("settings.google_api_key", &settings.google_api_key, true)?;
    }
    set("settings.google_proxy_enabled", if settings.google_proxy_enabled { "1" } else { "0" }, false)?;
    set("settings.google_proxy_protocol", &settings.google_proxy_protocol, false)?;
    set("settings.google_proxy_host", &settings.google_proxy_host, false)?;
    set("settings.google_proxy_port", &settings.google_proxy_port.to_string(), false)?;
    set("settings.google_proxy_username", &settings.google_proxy_username, false)?;
    set("settings.google_proxy_password", &settings.google_proxy_password, false)?;

    Ok(())
}

fn apply_legacy_settings(settings: &mut AppSettings, config: &LegacyConfig) {
    // 安全原则：只迁移用户偏好类非敏感设置
    // 禁止迁移任何凭据类字段（API Key、Token、Proxy 凭据等），避免跨仓库泄露
    if !config.theme.is_empty() { settings.theme = config.theme.clone(); }
    if !config.default_language.is_empty() { settings.default_language = config.default_language.clone(); }
    settings.auto_translate_readme = config.auto_translate_readme;
    settings.auto_translate_report = config.auto_translate_report;
    if !config.download_path.is_empty() { settings.download_path = config.download_path.clone(); }
    if !config.default_editor.is_empty() { settings.default_editor = config.default_editor.clone(); }
    settings.crawler_enabled = config.crawler_enabled;
    if !config.readme_update_frequency.is_empty() { settings.readme_update_frequency = config.readme_update_frequency.clone(); }
    if !config.translate_engine.is_empty() { settings.translate_engine = config.translate_engine.clone(); }

    // 以下字段一律不迁移（凭据/代理/LLM 连接信息），新仓库应保持空白，由用户按需重新配置
    // llm_provider, llm_api_base, llm_api_key, llm_model
    // llm_proxy_*, proxy_*, google_api_key, google_proxy_*
    // crawler_api_url
}

/// 从旧 settings.db 读取（早期版本的存储方式）
fn read_legacy_settings_db() -> Option<LegacyConfig> {
    let base_dirs = directories::BaseDirs::new()?;
    let app_data_dir = base_dirs.data_dir();
    // 兼容多个可能的目录名
    let candidates = [
        app_data_dir.join("com.administrator.os-compass").join("settings.db"),
        app_data_dir.join("com.os-compass.os-compass").join("settings.db"),
        app_data_dir.join("os-compass").join("settings.db"),
    ];

    for path in &candidates {
        if !path.exists() { continue; }
        // 复制到临时文件避免锁定
        let tmp = std::env::temp_dir().join(format!("oscompass_settings_{}.db", std::process::id()));
        if std::fs::copy(path, &tmp).is_err() { continue; }

        if let Ok(conn) = rusqlite::Connection::open(&tmp) {
            let mut config = LegacyConfig::default();
            let get = |key: &str| -> String {
                conn.query_row(
                    "SELECT value FROM settings WHERE key = ?",
                    [key],
                    |row| row.get::<_, String>(0),
                ).unwrap_or_default()
            };
            let get_bool = |key: &str, default: bool| -> bool {
                let v = get(key);
                if v.is_empty() { default } else { v == "1" || v.to_lowercase() == "true" }
            };
            let get_i32 = |key: &str| -> i32 {
                get(key).parse().unwrap_or(0)
            };

            config.theme = get("theme");
            config.default_language = get("default_language");
            config.auto_translate_readme = get_bool("auto_translate_readme", true);
            config.auto_translate_report = get_bool("auto_translate_report", true);
            config.download_path = get("download_path");
            config.default_editor = get("default_editor");
            config.llm_provider = get("llm_provider");
            config.llm_api_base = get("llm_api_base");
            config.llm_api_key = get("llm_api_key");
            config.llm_model = get("llm_model");
            config.llm_proxy_enabled = get_bool("llm_proxy_enabled", false);
            config.llm_proxy_protocol = get("llm_proxy_protocol");
            config.llm_proxy_host = get("llm_proxy_host");
            config.llm_proxy_port = get_i32("llm_proxy_port");
            config.llm_proxy_username = get("llm_proxy_username");
            config.llm_proxy_password = get("llm_proxy_password");
            config.proxy_enabled = get_bool("proxy_enabled", false);
            config.proxy_protocol = get("proxy_protocol");
            config.proxy_host = get("proxy_host");
            config.proxy_port = get_i32("proxy_port");
            config.proxy_username = get("proxy_username");
            config.proxy_password = get("proxy_password");
            config.crawler_enabled = get_bool("crawler_enabled", false);
            config.crawler_api_url = get("crawler_api_url");
            config.readme_update_frequency = get("readme_update_frequency");
            config.translate_engine = get("translate_engine");
            config.google_api_key = get("google_api_key");
            config.google_proxy_enabled = get_bool("google_proxy_enabled", false);
            config.google_proxy_protocol = get("google_proxy_protocol");
            config.google_proxy_host = get("google_proxy_host");
            config.google_proxy_port = get_i32("google_proxy_port");
            config.google_proxy_username = get("google_proxy_username");
            config.google_proxy_password = get("google_proxy_password");

            let _ = std::fs::remove_file(&tmp);
            return Some(config);
        }
        let _ = std::fs::remove_file(&tmp);
    }
    None
}

/// 读取旧的 config.json（兼容旧版本）
fn read_legacy_config() -> Option<LegacyConfig> {
    let base_dirs = directories::BaseDirs::new()?;
    let app_data_dir = base_dirs.data_dir();
    let candidates = [
        app_data_dir.join("com.os-compass.os-compass").join("config.json"),
        app_data_dir.join("os-compass").join("config.json"),
    ];

    for path in &candidates {
        if path.exists() {
            if let Ok(content) = std::fs::read_to_string(path) {
                if let Ok(config) = serde_json::from_str::<LegacyConfig>(&content) {
                    return Some(config);
                }
            }
        }
    }
    None
}

#[derive(serde::Deserialize, Default)]
struct LegacyConfig {
    #[serde(default)]
    theme: String,
    #[serde(default)]
    default_language: String,
    #[serde(default)]
    auto_translate_readme: bool,
    #[serde(default)]
    auto_translate_report: bool,
    #[serde(default)]
    download_path: String,
    #[serde(default)]
    default_editor: String,
    #[serde(default)]
    llm_provider: String,
    #[serde(default)]
    llm_api_base: String,
    #[serde(default)]
    llm_api_key: String,
    #[serde(default)]
    llm_model: String,
    #[serde(default)]
    llm_proxy_enabled: bool,
    #[serde(default)]
    llm_proxy_protocol: String,
    #[serde(default)]
    llm_proxy_host: String,
    #[serde(default)]
    llm_proxy_port: i32,
    #[serde(default)]
    llm_proxy_username: String,
    #[serde(default)]
    llm_proxy_password: String,
    #[serde(default)]
    proxy_enabled: bool,
    #[serde(default)]
    proxy_protocol: String,
    #[serde(default)]
    proxy_host: String,
    #[serde(default)]
    proxy_port: i32,
    #[serde(default)]
    proxy_username: String,
    #[serde(default)]
    proxy_password: String,
    #[serde(default)]
    crawler_enabled: bool,
    #[serde(default)]
    crawler_api_url: String,
    #[serde(default)]
    readme_update_frequency: String,
    #[serde(default)]
    translate_engine: String,
    #[serde(default)]
    google_api_key: String,
    #[serde(default)]
    google_proxy_enabled: bool,
    #[serde(default)]
    google_proxy_protocol: String,
    #[serde(default)]
    google_proxy_host: String,
    #[serde(default)]
    google_proxy_port: i32,
    #[serde(default)]
    google_proxy_username: String,
    #[serde(default)]
    google_proxy_password: String,
}

pub fn get_app_data_dir(app: &tauri::AppHandle) -> PathBuf {
    app.path().app_data_dir().unwrap_or_default()
}
