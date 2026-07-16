use crate::source_engine::{build_client, RawContent, SourceAdapter, SourceError, SourceType};
use async_trait::async_trait;
use regex::Regex;
use std::time::Duration;

pub struct TelegramAdapter {
    client: Option<reqwest::Client>,
}

impl TelegramAdapter {
    pub fn new() -> Self {
        TelegramAdapter { client: None }
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

    async fn build_session(&mut self, proxy: Option<&str>) -> Result<&reqwest::Client, SourceError> {
        if self.client.is_none() {
            let mut builder = reqwest::Client::builder()
                .timeout(Duration::from_secs(60))
                .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36");

            if let Some(proxy_url) = proxy {
                if !proxy_url.is_empty() {
                    let proxy = reqwest::Proxy::all(proxy_url)
                        .map_err(|e| SourceError::ConfigError(e.to_string()))?;
                    builder = builder.proxy(proxy);
                }
            }

            let client = builder.build()
                .map_err(|e| SourceError::ConfigError(e.to_string()))?;
            self.client = Some(client);
        }
        Ok(self.client.as_ref().unwrap())
    }

    fn extract_links(&self, html: &str) -> Vec<RawContent> {
        let mut results = Vec::new();

        let patterns = [
            r#"https?://(?:www\.)?github\.com/[\w-]+/[\w.-]+"#,
            r#"https?://(?:www\.)?gitee\.com/[\w-]+/[\w.-]+"#,
            r#"https?://(?:www\.)?gitlab\.com/[\w-]+/[\w.-]+"#,
        ];

        for pattern in &patterns {
            if let Ok(re) = Regex::new(pattern) {
                let matches: Vec<String> = re.find_iter(html)
                    .map(|m| m.as_str().to_string())
                    .collect();
                for url in matches {
                    if !results.iter().any(|r: &RawContent| r.url == url) {
                        results.push(RawContent {
                            title: url.clone(),
                            url,
                            content: None,
                            published_at: None,
                        });
                    }
                }
            }
        }

        results
    }
}

#[async_trait]
impl SourceAdapter for TelegramAdapter {
    fn adapter_type(&self) -> SourceType {
        SourceType::Telegram
    }

    async fn fetch(&mut self, url: &str, proxy: Option<&str>) -> Result<Vec<RawContent>, SourceError> {
        let channel_url = Self::to_channel_url(url)
            .ok_or_else(|| SourceError::ParseError("Invalid Telegram URL".to_string()))?;

        let channel_name = channel_url.split("/s/").nth(1).unwrap_or("unknown");
        println!("[telegram] Fetching channel: {} -> {}", channel_name, channel_url);

        let client = self.build_session(proxy).await?;

        tokio::time::sleep(Duration::from_millis(500)).await;

        let main_page = client
            .get(format!("https://t.me/{}", channel_name))
            .header("Accept", "text/html,application/xhtml+xml")
            .header("Accept-Language", "en-US,en;q=0.9")
            .send()
            .await
            .map_err(|e| {
                println!("[telegram] Main page request failed: {}", e);
                SourceError::NetworkError(e.to_string())
            })?;

        println!("[telegram] Main page status: {}", main_page.status());

        tokio::time::sleep(Duration::from_millis(300)).await;

        let response = client
            .get(&channel_url)
            .header("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9")
            .header("Accept-Language", "en-US,en;q=0.9")
            .header("Cache-Control", "no-cache")
            .header("Pragma", "no-cache")
            .send()
            .await
            .map_err(|e| {
                println!("[telegram] Channel page request failed: {}", e);
                SourceError::NetworkError(e.to_string())
            })?;

        let status = response.status();
        println!("[telegram] Channel page status: {}", status);

        if !status.is_success() {
            let error_msg = format!("HTTP error: {}", status);
            println!("[telegram] {}", error_msg);
            return Err(SourceError::NetworkError(error_msg));
        }

        let html = response.text().await.map_err(|e| {
            println!("[telegram] Failed to read response: {}", e);
            SourceError::NetworkError(e.to_string())
        })?;

        println!("[telegram] Received HTML: {} bytes", html.len());

        if html.len() < 5000 {
            println!("[telegram] HTML too short ({}) - might be blocked:", html.len());
            println!("{}", &html[..html.len().min(2000)]);
        }

        let results = self.extract_links(&html);
        println!("[telegram] Extracted {} links", results.len());

        Ok(results)
    }
}
