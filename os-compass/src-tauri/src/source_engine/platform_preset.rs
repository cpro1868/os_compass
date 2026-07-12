use crate::source_engine::{rss_adapter::RssAdapter, RawContent, SourceAdapter, SourceError, SourceType};
use async_trait::async_trait;
use std::collections::HashMap;

pub struct PlatformPresetAdapter {
    presets: HashMap<String, String>,
}

impl PlatformPresetAdapter {
    pub fn new() -> Self {
        let mut presets = HashMap::new();
        presets.insert("github_trending_daily".to_string(), "https://rsshub.app/github/trending/daily".to_string());
        presets.insert("github_trending_weekly".to_string(), "https://rsshub.app/github/trending/weekly".to_string());
        presets.insert("github_trending_monthly".to_string(), "https://rsshub.app/github/trending/monthly".to_string());
        presets.insert("gitee_trending".to_string(), "https://rsshub.app/gitee/trending".to_string());
        PlatformPresetAdapter { presets }
    }

    fn resolve_url(&self, url: &str) -> String {
        for (key, preset_url) in &self.presets {
            if url.contains(key) {
                return preset_url.clone();
            }
        }
        url.to_string()
    }
}

#[async_trait]
impl SourceAdapter for PlatformPresetAdapter {
    fn adapter_type(&self) -> SourceType {
        SourceType::PlatformPreset
    }

    async fn fetch(&self, url: &str, proxy: Option<&str>) -> Result<Vec<RawContent>, SourceError> {
        let resolved_url = self.resolve_url(url);
        let rss_adapter = RssAdapter::new();
        rss_adapter.fetch(&resolved_url, proxy).await
    }
}
