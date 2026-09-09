use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DbMode {
    Main,
    Global,
    Vault,
    None,
}

impl DbMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            DbMode::Main => "main",
            DbMode::Global => "global",
            DbMode::Vault => "vault",
            DbMode::None => "none",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "main" => Some(DbMode::Main),
            "global" => Some(DbMode::Global),
            "vault" => Some(DbMode::Vault),
            "none" => Some(DbMode::None),
            _ => None,
        }
    }
}

impl Default for DbMode {
    fn default() -> Self {
        DbMode::None
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FeaturePluginType {
    Radar,
    Search,
    Webhook,
    Importer,
}

impl FeaturePluginType {
    pub fn as_str(&self) -> &'static str {
        match self {
            FeaturePluginType::Radar => "radar",
            FeaturePluginType::Search => "search",
            FeaturePluginType::Webhook => "webhook",
            FeaturePluginType::Importer => "importer",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "radar" => Some(FeaturePluginType::Radar),
            "search" => Some(FeaturePluginType::Search),
            "webhook" => Some(FeaturePluginType::Webhook),
            "importer" => Some(FeaturePluginType::Importer),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResultStatus {
    Success,
    Failed,
    Partial,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureResult {
    pub status: ResultStatus,
    pub data: Option<serde_json::Value>,
    pub message: Option<String>,
}

impl FeatureResult {
    pub fn success() -> Self {
        FeatureResult {
            status: ResultStatus::Success,
            data: None,
            message: None,
        }
    }

    pub fn success_with_data(data: serde_json::Value) -> Self {
        FeatureResult {
            status: ResultStatus::Success,
            data: Some(data),
            message: None,
        }
    }

    pub fn failed(message: String) -> Self {
        FeatureResult {
            status: ResultStatus::Failed,
            data: None,
            message: Some(message),
        }
    }

    pub fn partial(message: String) -> Self {
        FeatureResult {
            status: ResultStatus::Partial,
            data: None,
            message: Some(message),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginInfo {
    pub id: String,
    pub name: String,
    pub plugin_type: String,
    pub enabled: bool,
    pub config: Option<String>,
    pub version: String,
    pub db_mode: String,
    pub db_path_template: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginContext {
    pub vault_dir: PathBuf,
    pub app_data_dir: PathBuf,
    pub config: serde_json::Value,
    pub llm_config: serde_json::Value,
    pub plugin_db_path: Option<PathBuf>,
}

impl PluginContext {
    pub fn new(
        vault_dir: PathBuf,
        app_data_dir: PathBuf,
        config: serde_json::Value,
        llm_config: serde_json::Value,
        plugin_db_path: Option<PathBuf>,
    ) -> Self {
        PluginContext {
            vault_dir,
            app_data_dir,
            config,
            llm_config,
            plugin_db_path,
        }
    }
}

#[derive(Debug)]
pub enum PluginError {
    NotSupported(String),
    NotFound(String),
    InitFailed(String),
    ExecutionFailed(String),
    DbError(String),
    ConfigError(String),
}

impl fmt::Display for PluginError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PluginError::NotSupported(msg) => write!(f, "Not supported: {}", msg),
            PluginError::NotFound(msg) => write!(f, "Not found: {}", msg),
            PluginError::InitFailed(msg) => write!(f, "Initialization failed: {}", msg),
            PluginError::ExecutionFailed(msg) => write!(f, "Execution failed: {}", msg),
            PluginError::DbError(msg) => write!(f, "Database error: {}", msg),
            PluginError::ConfigError(msg) => write!(f, "Config error: {}", msg),
        }
    }
}

impl std::error::Error for PluginError {}

pub fn resolve_db_path(
    db_mode: DbMode,
    db_path_template: Option<&str>,
    vault_dir: &PathBuf,
    app_data_dir: &PathBuf,
    plugin_id: &str,
) -> Option<PathBuf> {
    match db_mode {
        DbMode::Main => None,
        DbMode::None => None,
        DbMode::Global | DbMode::Vault => {
            let template = db_path_template?;
            let mut path = template.to_string();
            path = path.replace("${vault_dir}", &vault_dir.to_string_lossy());
            path = path.replace("${app_data_dir}", &app_data_dir.to_string_lossy());
            path = path.replace("${plugin_id}", plugin_id);
            Some(PathBuf::from(path))
        }
    }
}

#[async_trait]
pub trait FeaturePlugin: Send + Sync {
    fn id(&self) -> &str;
    fn name(&self) -> &str;
    fn plugin_type(&self) -> FeaturePluginType;
    fn version(&self) -> &str;

    fn db_mode(&self) -> DbMode {
        DbMode::None
    }

    fn db_path_template(&self) -> Option<&str> {
        None
    }

    async fn init(&self, context: &PluginContext) -> Result<(), PluginError>;
    async fn on_enable(&self, context: &PluginContext) -> Result<(), PluginError>;
    async fn on_disable(&self, context: &PluginContext) -> Result<(), PluginError>;
    async fn execute(&self, context: &PluginContext) -> Result<FeatureResult, PluginError>;
}
