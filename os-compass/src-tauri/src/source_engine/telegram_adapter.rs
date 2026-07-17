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

    fn normalize_url(url: &str) -> Option<String> {
        let url = url.trim();
        if url.contains("t.me/s/") {
            return Some(url.to_string());
        }
        if url.contains("t.me/") {
            let channel_path = url.split("t.me/").nth(1)?;
            if channel_path.contains('/') || channel_path.contains('?') {
                return None;
            }
            return Some(format!("https://t.me/s/{}", channel_path));
        }
        None
    }

    fn html_to_text(&self, html: &str) -> String {
        let mut text = html
            .replace("&nbsp;", " ")
            .replace("&amp;", "&")
            .replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&quot;", "\"")
            .replace("&#39;", "'")
            .replace("<br>", "\n")
            .replace("<br/>", "\n")
            .replace("<br />", "\n")
            .replace("</p>", "\n")
            .replace("</div>", "\n");

        let re = Regex::new("<[^>]+>").unwrap();
        text = re.replace_all(&text, " ").to_string();

        let re2 = Regex::new("[ \\t]+").unwrap();
        text = re2.replace_all(&text, " ").to_string();

        let re3 = Regex::new("\\n{3,}").unwrap();
        text = re3.replace_all(&text, "\n\n").to_string();

        text.trim().to_string()
    }

    fn extract_all_urls(&self, text: &str) -> String {
        let re = Regex::new("https?://[^\\s<>]+").unwrap();
        let urls: Vec<String> = re.find_iter(text)
            .map(|m| m.as_str().to_string())
            .collect();
        urls.join(", ")
    }

    fn extract_datetime_from_block(&self, block: &str) -> Option<String> {
        let re = Regex::new("\\d{4}-\\d{2}-\\d{2}[T ]\\d{2}:\\d{2}:\\d{2}").ok()?;
        re.find(block).map(|m| m.as_str().to_string())
    }

    fn extract_messages(&self, html: &str) -> Vec<RawContent> {
        let mut results = Vec::new();
        let text = self.html_to_text(html);

        let message_blocks: Vec<&str> = text.split("\n\n")
            .filter(|block| block.len() > 20)
            .collect();

        for block in message_blocks {
            let content = block.trim();
            if content.len() < 20 {
                continue;
            }

            let title = content.lines()
                .next()
                .unwrap_or(content)
                .chars()
                .take(100)
                .collect::<String>();

            let published_at = self.extract_datetime_from_block(content);
            let links = self.extract_all_urls(content);

            results.push(RawContent {
                title,
                url: links,
                content: Some(content.to_string()),
                published_at,
            });

            if results.len() >= 100 {
                break;
            }
        }

        if results.is_empty() {
            results = self.extract_from_html_direct(html);
        }

        println!("[telegram] Extracted {} messages", results.len());
        results
    }

    fn extract_from_html_direct(&self, html: &str) -> Vec<RawContent> {
        let mut results = Vec::new();
        let text = self.html_to_text(html);

        let re = match Regex::new("https?://[^\\s<>]+") {
            Ok(r) => r,
            Err(_) => return results,
        };
        let matches: Vec<_> = re.find_iter(&text).collect();

        let chunk_size = 500;
        let mut i = 0;

        while i < matches.len() {
            let start = matches[i].start().saturating_sub(200);
            let end = if i + chunk_size < matches.len() {
                matches[i + chunk_size].end() + 200
            } else {
                text.len()
            };

            let snippet = &text[start..end.min(text.len())];

            if snippet.len() > 30 {
                let title = snippet.lines()
                    .next()
                    .unwrap_or(snippet)
                    .chars()
                    .take(80)
                    .collect::<String>();

                let urls = self.extract_all_urls(snippet);

                results.push(RawContent {
                    title,
                    url: urls,
                    content: Some(snippet.to_string()),
                    published_at: None,
                });
            }

            i += chunk_size;
        }

        results.truncate(100);
        println!("[telegram] Direct HTML extraction: {} messages", results.len());
        results
    }

    async fn http_get(&self, url: &str, proxy: Option<&str>) -> Result<String, SourceError> {
        let cookie_value = {
            let guard = self.cookie_store.lock().unwrap();
            guard.clone()
        };

        let mut builder = reqwest::Client::builder()
            .timeout(Duration::from_secs(60))
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
            .tcp_keepalive(Duration::from_secs(30));

        if let Some(proxy_url) = proxy {
            if !proxy_url.is_empty() {
                let proxy = reqwest::Proxy::all(proxy_url)
                    .map_err(|e| SourceError::ConfigError(e.to_string()))?;
                builder = builder.proxy(proxy);
            }
        }

        let client = builder.build()
            .map_err(|e| SourceError::ConfigError(e.to_string()))?;

        let mut request = client.get(url)
            .header("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9")
            .header("Accept-Language", "en-US,en;q=0.9")
            .header("Accept-Encoding", "gzip, deflate, br")
            .header("DNT", "1")
            .header("Connection", "keep-alive");

        if let Some(cookie) = cookie_value {
            request = request.header("Cookie", cookie);
        }

        let response = request.send().await
            .map_err(|e| SourceError::NetworkError(e.to_string()))?;

        let status = response.status();
        if status.as_u16() == 429 {
            return Err(SourceError::NetworkError("Rate limited".to_string()));
        }
        if !status.is_success() {
            return Err(SourceError::NetworkError(format!("HTTP {}", status)));
        }

        if let Some(set_cookie) = response.headers().get("set-cookie") {
            if let Ok(cookie_str) = set_cookie.to_str() {
                let re = Regex::new("(stel_ssid|hash)=([^;]+)").ok();
                if let Some(re) = re {
                    if let Some(cap) = re.captures(cookie_str) {
                        let mut guard = self.cookie_store.lock().unwrap();
                        *guard = Some(format!("{}={};", &cap[1], &cap[2]));
                    }
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
        let channel_url = Self::normalize_url(url)
            .ok_or_else(|| SourceError::ParseError("Invalid Telegram URL".to_string()))?;

        println!("[telegram] Fetching: {}", channel_url);
        if let Some(p) = proxy {
            println!("[telegram] Proxy: {}", p);
        }

        tokio::time::sleep(Duration::from_millis(500)).await;

        let html = self.http_get(&channel_url, proxy).await?;
        println!("[telegram] Received {} bytes", html.len());

        if html.len() < 1000 {
            println!("[telegram] WARNING: Response too short");
            return Ok(Vec::new());
        }

        let results = self.extract_messages(&html);
        println!("[telegram] Extracted {} messages", results.len());

        Ok(results)
    }
}
