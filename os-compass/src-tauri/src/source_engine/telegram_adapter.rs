use crate::source_engine::{build_client, RawContent, SourceAdapter, SourceError, SourceType};
use async_trait::async_trait;
use regex::Regex;

pub struct TelegramAdapter;

impl TelegramAdapter {
    pub fn new() -> Self {
        TelegramAdapter
    }

    fn to_channel_url(url: &str) -> Option<String> {
        let url = url.trim();
        if url.contains("t.me/s/") {
            return Some(url.to_string());
        }
        if url.contains("t.me/") {
            let channel_path = url.split("t.me/").nth(1)?;
            if channel_path.contains('/') {
                return None;
            }
            return Some(format!("https://t.me/s/{}", channel_path));
        }
        None
    }
}

#[async_trait]
impl SourceAdapter for TelegramAdapter {
    fn adapter_type(&self) -> SourceType {
        SourceType::Telegram
    }

    async fn fetch(&self, url: &str, proxy: Option<&str>) -> Result<Vec<RawContent>, SourceError> {
        let channel_url = Self::to_channel_url(url)
            .ok_or_else(|| SourceError::ParseError("Invalid Telegram URL".to_string()))?;

        let client = build_client(proxy)?;
        let response = client.get(&channel_url).send().await?;
        let html = response.text().await?;

        let github_regex = Regex::new(r"https?://(?:www\.)?github\.com/[\w-]+/[\w.-]+").unwrap();
        let gitee_regex = Regex::new(r"https?://(?:www\.)?gitee\.com/[\w-]+/[\w.-]+").unwrap();

        let github_urls: Vec<String> = github_regex
            .find_iter(&html)
            .map(|m| m.as_str().to_string())
            .collect();

        let gitee_urls: Vec<String> = gitee_regex
            .find_iter(&html)
            .map(|m| m.as_str().to_string())
            .collect();

        let mut all_urls: Vec<String> = github_urls;
        all_urls.extend(gitee_urls);
        all_urls.sort();
        all_urls.dedup();

        let results: Vec<RawContent> = all_urls
            .into_iter()
            .map(|url| RawContent {
                title: url.clone(),
                url,
                content: None,
                published_at: None,
            })
            .collect();

        Ok(results)
    }
}
