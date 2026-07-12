pub mod rss_adapter;
pub mod webcrawl_adapter;
pub mod telegram_adapter;
pub mod platform_preset;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawContent {
    pub title: String,
    pub url: String,
    pub content: Option<String>,
    pub published_at: Option<String>,
}

#[derive(Debug)]
pub enum SourceError {
    NetworkError(String),
    ParseError(String),
    ConfigError(String),
}

impl fmt::Display for SourceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SourceError::NetworkError(msg) => write!(f, "Network error: {}", msg),
            SourceError::ParseError(msg) => write!(f, "Parse error: {}", msg),
            SourceError::ConfigError(msg) => write!(f, "Config error: {}", msg),
        }
    }
}

impl std::error::Error for SourceError {}

impl From<String> for SourceError {
    fn from(s: String) -> Self {
        SourceError::NetworkError(s)
    }
}

impl From<reqwest::Error> for SourceError {
    fn from(e: reqwest::Error) -> Self {
        SourceError::NetworkError(e.to_string())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceType {
    Rss,
    WebCrawl,
    Telegram,
    PlatformPreset,
}

impl SourceType {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "rss" | "atom" => Some(SourceType::Rss),
            "web_crawl" => Some(SourceType::WebCrawl),
            "telegram" => Some(SourceType::Telegram),
            "platform_preset" => Some(SourceType::PlatformPreset),
            _ => None,
        }
    }
}

#[async_trait]
pub trait SourceAdapter: Send + Sync {
    fn adapter_type(&self) -> SourceType;
    async fn fetch(&self, url: &str, proxy: Option<&str>) -> Result<Vec<RawContent>, SourceError>;
}

pub fn build_client(proxy: Option<&str>) -> Result<reqwest::Client, SourceError> {
    let mut builder = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .user_agent("OS-Compass/1.0");

    if let Some(proxy_url) = proxy {
        if !proxy_url.is_empty() {
            let proxy = reqwest::Proxy::all(proxy_url)
                .map_err(|e| SourceError::ConfigError(e.to_string()))?;
            builder = builder.proxy(proxy);
        }
    }

    builder.build().map_err(|e| SourceError::ConfigError(e.to_string()))
}

pub fn get_adapter(source_type: SourceType) -> Box<dyn SourceAdapter> {
    match source_type {
        SourceType::Rss => Box::new(rss_adapter::RssAdapter::new()),
        SourceType::WebCrawl => Box::new(webcrawl_adapter::WebCrawlAdapter::new()),
        SourceType::Telegram => Box::new(telegram_adapter::TelegramAdapter::new()),
        SourceType::PlatformPreset => Box::new(platform_preset::PlatformPresetAdapter::new()),
    }
}
