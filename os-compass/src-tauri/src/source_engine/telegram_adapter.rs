use crate::source_engine::{RawContent, SourceAdapter, SourceError, SourceType};
use async_trait::async_trait;
use std::time::Duration;

pub struct TelegramAdapter;

impl TelegramAdapter {
    pub fn new() -> Self {
        TelegramAdapter
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
        let mut text = html.to_string();
        text = text.replace("&nbsp;", " ");
        text = text.replace("&amp;", "&");
        text = text.replace("&lt;", "<");
        text = text.replace("&gt;", ">");
        text = text.replace("&quot;", "\"");
        text = text.replace("&#39;", "'");
        text = text.replace("<br>", "\n");
        text = text.replace("<br/>", "\n");
        text = text.replace("<br />", "\n");
        text = text.replace("</p>", "\n");
        text = text.replace("</div>", "\n");
        text = text.replace("</li>", "\n");
        
        // 移除所有 HTML 标签
        let mut result = String::new();
        let mut in_tag = false;
        for c in text.chars() {
            if c == '<' {
                in_tag = true;
            } else if c == '>' {
                in_tag = false;
            } else if !in_tag {
                result.push(c);
            }
        }
        
        // 清理多余空白
        let mut cleaned = String::new();
        let mut last_was_space = false;
        for c in result.chars() {
            if c.is_whitespace() {
                if !last_was_space {
                    cleaned.push(' ');
                    last_was_space = true;
                }
            } else {
                cleaned.push(c);
                last_was_space = false;
            }
        }
        
        cleaned.trim().to_string()
    }

    fn extract_messages(&self, html: &str) -> Vec<RawContent> {
        let mut results = Vec::new();
        let text = self.html_to_text(html);

        // 按消息分隔符分割文本
        let parts: Vec<&str> = text.split("\n\n\n").collect();
        let total_parts = parts.len();
        
        for part in parts {
            let part = part.trim();
            if part.len() < 30 {
                continue;
            }
            
            // 提取链接
            let mut urls = Vec::new();
            let url_parts: Vec<&str> = part.split_whitespace().collect();
            for wp in url_parts {
                if wp.starts_with("http://") || wp.starts_with("https://") {
                    let clean_url = wp.trim_matches(|c| c == ',' || c == '.' || c == '"' || c == '\'' || c == ')' || c == '(' || c == '<' || c == '>' || c == ';');
                    if !clean_url.contains("telegram.org") && !clean_url.contains("t.me/iyouport") && clean_url.len() > 10 {
                        urls.push(clean_url.to_string());
                    }
                }
            }
            
            // 提取第一行作为标题
            let first_line = part.lines().next().unwrap_or(part);
            let title = if first_line.len() > 100 {
                first_line.chars().take(100).collect::<String>()
            } else {
                first_line.to_string()
            };
            
            // 提取时间（如果有）
            let time_parts: Vec<&str> = part.lines()
                .filter(|l| l.len() >= 8 && l.len() <= 25)
                .collect();
            let published_at = time_parts.first().map(|s| s.to_string());
            
            results.push(RawContent {
                title,
                url: urls.join(", "),
                content: Some(part.to_string()),
                published_at,
            });
            
            if results.len() >= 50 {
                break;
            }
        }
        
        println!("[telegram] Extracted {} messages from {} parts", results.len(), total_parts);
        results
    }

    async fn http_get(&self, url: &str, proxy: Option<&str>) -> Result<String, SourceError> {
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

        let response = client.get(url)
            .header("Accept", "text/html,application/xhtml+xml")
            .header("Accept-Language", "en-US,en;q=0.9")
            .header("DNT", "1")
            .send()
            .await
            .map_err(|e| SourceError::NetworkError(e.to_string()))?;

        let status = response.status();
        if status.as_u16() == 429 {
            return Err(SourceError::NetworkError("Rate limited".to_string()));
        }
        if !status.is_success() {
            return Err(SourceError::NetworkError(format!("HTTP {}", status)));
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

    async fn fetch(&mut self, url: &str, _proxy: Option<&str>) -> Result<Vec<RawContent>, SourceError> {
        let channel_url = Self::normalize_url(url)
            .ok_or_else(|| SourceError::ParseError("Invalid Telegram URL".to_string()))?;

        println!("[telegram] Fetching: {}", channel_url);

        tokio::time::sleep(Duration::from_millis(500)).await;

        let html = self.http_get(&channel_url, _proxy).await?;
        println!("[telegram] Received {} bytes", html.len());

        if html.len() < 1000 {
            println!("[telegram] WARNING: Response too short");
            return Ok(Vec::new());
        }

        let results = self.extract_messages(&html);
        Ok(results)
    }
}
