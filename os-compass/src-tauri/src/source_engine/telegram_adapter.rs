use crate::source_engine::{RawContent, SourceAdapter, SourceError, SourceType};
use async_trait::async_trait;
use regex::Regex;
use std::time::Duration;

pub struct TelegramAdapter {
    cookie_store: std::sync::Mutex<Option<String>>,
}

impl TelegramAdapter {
    pub fn new() -> Self {
        TelegramAdapter {
            cookie_store: std::sync::Mutex::new(None),
        }
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

    fn strip_html(&self, html: &str) -> String {
        let text = html
            .replace("&nbsp;", " ")
            .replace("&amp;", "&")
            .replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&quot;", "\"")
            .replace("<br>", "\n")
            .replace("<br/>", "\n")
            .replace("<br />", "\n")
            .replace("</p>", "\n")
            .replace("</div>", "\n");

        let re = Regex::new(r"<[^>]+>").unwrap();
        let text = re.replace_all(&text, " ");
        let re2 = Regex::new(r"\n{3,}").unwrap();
        let text = re2.replace_all(&text, "\n\n");
        text.trim().to_string()
    }

    fn extract_messages(&self, html: &str) -> Vec<RawContent> {
        let mut results = Vec::new();

        let message_re = Regex::new(r#"<div[^>]*class="[^"]*message[^"]*"[^>]*>(.*?)</div>\s*<a[^>]*class="[^"]*date[^"]*""#).unwrap();

        for cap in message_re.captures_iter(html) {
            let message_html = &cap[1];

            let text = self.strip_html(message_html);

            if text.len() < 5 {
                continue;
            }

            let link_re = Regex::new(r#"href="([^"]+)""#).unwrap();
            let link = link_re.captures(message_html)
                .and_then(|c| {
                    let href = &c[1];
                    if href.starts_with("/") {
                        Some(format!("https://t.me{}", href))
                    } else if href.starts_with("http") {
                        Some(href.to_string())
                    } else {
                        None
                    }
                })
                .unwrap_or_default();

            let date_re = Regex::new(r#"datetime="([^"]+)""#).unwrap();
            let datetime = date_re.captures(&cap[0])
                .map(|c| c[1].to_string());

            let title = text.lines()
                .take(1)
                .next()
                .unwrap_or(&text)
                .chars()
                .take(80)
                .collect::<String>();

            results.push(RawContent {
                title,
                url: link,
                content: Some(text),
                published_at: datetime,
            });
        }

        if results.is_empty() {
            let alt_re = Regex::new(r#"<div[^>]*class="[^"]*tgme_widget_message[^"]*"[^>]*>(.*?)</div>"#).unwrap();
            for cap in alt_re.captures_iter(html) {
                let content = &cap[1];
                let text = self.strip_html(content);

                if text.len() < 10 {
                    continue;
                }

                let title = text.lines()
                    .take(1)
                    .next()
                    .unwrap_or(&text)
                    .chars()
                    .take(80)
                    .collect::<String>();

                results.push(RawContent {
                    title,
                    url: String::new(),
                    content: Some(text),
                    published_at: None,
                });
            }
        }

        println!("[telegram] Extracted {} messages", results.len());
        results
    }

    async fn fetch_url(&self, url: &str, proxy: Option<&str>) -> Result<String, SourceError> {
        let cookie_header = {
            let cookies = self.cookie_store.lock().unwrap();
            cookies.clone()
        };

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

        let mut req = client.get(url)
            .header("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8")
            .header("Accept-Language", "en-US,en;q=0.9")
            .header("Cache-Control", "no-cache")
            .header("Pragma", "no-cache");

        if let Some(cookie) = cookie_header {
            req = req.header("Cookie", cookie);
        }

        let response = req.send().await
            .map_err(|e| {
                println!("[telegram] Request failed: {}", e);
                SourceError::NetworkError(e.to_string())
            })?;

        let status = response.status();
        println!("[telegram] Response status: {}", status);

        if !status.is_success() {
            return Err(SourceError::NetworkError(format!("HTTP error: {}", status)));
        }

        if let Some(cookie_header_val) = response.headers().get("set-cookie") {
            if let Ok(cookie_str) = cookie_header_val.to_str() {
                let re = Regex::new(r"stel_ssid=([^;]+)").unwrap();
                if let Some(cap) = re.captures(cookie_str) {
                    let mut cookies = self.cookie_store.lock().unwrap();
                    *cookies = Some(format!("stel_ssid={};", &cap[1]));
                }
            }
        }

        response.text().await
            .map_err(|e| SourceError::NetworkError(e.to_string()))
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

        println!("[telegram] Fetching channel: {}", channel_url);

        tokio::time::sleep(Duration::from_millis(500)).await;

        let html = self.fetch_url(&channel_url, proxy).await?;

        println!("[telegram] Received HTML: {} bytes", html.len());

        if html.len() < 5000 {
            println!("[telegram] HTML too short - might be blocked or empty");
            return Ok(Vec::new());
        }

        let results = self.extract_messages(&html);
        println!("[telegram] Total messages extracted: {}", results.len());

        Ok(results)
    }
}
