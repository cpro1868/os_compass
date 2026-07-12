use crate::feature_plugin::{
    DbMode, FeaturePlugin, FeaturePluginType, FeatureResult, PluginContext, PluginError,
};
use async_trait::async_trait;

pub struct RadarPlugin;

impl RadarPlugin {
    pub fn new() -> Self {
        RadarPlugin
    }
}

#[async_trait]
impl FeaturePlugin for RadarPlugin {
    fn id(&self) -> &str {
        "radar"
    }

    fn name(&self) -> &str {
        "情报雷达"
    }

    fn plugin_type(&self) -> FeaturePluginType {
        FeaturePluginType::Radar
    }

    fn version(&self) -> &str {
        "1.0.0"
    }

    fn db_mode(&self) -> DbMode {
        DbMode::Vault
    }

    fn db_path_template(&self) -> Option<&str> {
        Some("${vault_dir}/plugin_${plugin_id}.db")
    }

    async fn init(&self, _context: &PluginContext) -> Result<(), PluginError> {
        Ok(())
    }

    async fn on_enable(&self, _context: &PluginContext) -> Result<(), PluginError> {
        Ok(())
    }

    async fn on_disable(&self, _context: &PluginContext) -> Result<(), PluginError> {
        Ok(())
    }

    async fn execute(&self, _context: &PluginContext) -> Result<FeatureResult, PluginError> {
        Ok(FeatureResult::success())
    }
}
