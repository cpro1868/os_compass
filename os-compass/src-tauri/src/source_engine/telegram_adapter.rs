use crate::source_engine::{RawContent, SourceAdapter, SourceError, SourceType};
use async_trait::async_trait;
use regex::Regex;
use std::time::Duration;

pub struct TelegramAdapter;

impl TelegramAdapter {
    pub fn new() -> Self { TelegramAdapter }

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

    fn decode_html_entities(&self, text: &str) -> String {
        let mut result = text.to_string();
        result = result.replace("&lt;", "<");
        result = result.replace("&gt;", ">");
        result = result.replace("&amp;", "&");
        result = result.replace("&quot;", "\"");
        result = result.replace("&apos;", "'");
        result = result.replace("&nbsp;", " ");
        result
    }

    fn strip_html_tags(&self, html: &str) -> String {
        let mut result = String::new();
        let mut in_tag = false;
        for c in html.chars() {
            if c == '<' { in_tag = true; }
            else if c == '>' { in_tag = false; }
            else if !in_tag { result.push(c); }
        }
        result.split_whitespace().collect::<Vec<_>>().join(" ")
    }

    fn extract_messages_with_next(&self, html: &str) -> (Vec<RawContent>, Option<i64>) {
        let decoded = self.decode_html_entities(html);
        
        let msg_id_re = Regex::new(r#"data-post="[^/]+/(\d+)""#).unwrap();
        let mut msg_ids: Vec<i64> = Vec::new();
        for cap in msg_id_re.captures_iter(&decoded) {
            if let Ok(id) = cap[1].parse::<i64>() {
                msg_ids.push(id);
            }
        }
        
        let next_last_id = msg_ids.iter().min().copied();
        
        let text_re = Regex::new(r#"class="tgme_widget_message_text[^"]*"[^>]*>([\s\S]*?)</div>"#).unwrap();
        let time_re = Regex::new(r#"<time datetime="([^"]+)""#).unwrap();
        
        let mut results = Vec::new();
        
        for cap in text_re.captures_iter(&decoded) {
            let raw_text = &cap[1];
            let text = self.strip_html_tags(raw_text);
            
            if text.len() < 10 {
                continue;
            }
            
            let published_at = time_re.captures(&decoded).and_then(|c| {
                let utc_time = &c[1];
                chrono::DateTime::parse_from_rfc3339(utc_time)
                    .ok()
                    .map(|dt| dt.with_timezone(&chrono::Local).format("%Y-%m-%d %H:%M:%S").to_string())
            });
            
            let url_re = Regex::new(r#"https?://[^\s<>"']+[^<>\s.,;:!?]""#).unwrap();
            let urls: Vec<String> = url_re.find_iter(&text)
                .filter_map(|m| {
                    let url = m.as_str();
                    if url.contains("telegram.org") || url.contains("t.me/iyouport") {
                        None
                    } else {
                        Some(url.to_string())
                    }
                })
                .collect();
            
            let title = text.lines()
                .next()
                .map(|l| l.chars().take(80).collect::<String>())
                .unwrap_or_default();
            
            results.push(RawContent {
                title,
                url: urls.join(", "),
                content: Some(text),
                published_at,
            });
            
            if results.len() >= 30 {
                break;
            }
        }
        
        (results, next_last_id)
    }

    async fn http_get(&self, url: &str, proxy: Option<&str>) -> Result<String, SourceError> {
        let proxy_info = proxy.unwrap_or("none");
        println!("[telegram] http_get: url={}, proxy={}", url, proxy_info);
        
        let mut builder = reqwest::Client::builder()
            .timeout(Duration::from_secs(60))
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36");

        if let Some(proxy_url) = proxy {
            if !proxy_url.is_empty() {
                println!("[telegram] Setting proxy: {}", proxy_url);
                let proxy = reqwest::Proxy::all(proxy_url)
                    .map_err(|e| SourceError::ConfigError(e.to_string()))?;
                builder = builder.proxy(proxy);
            }
        }

        let client = builder.build()
            .map_err(|e| SourceError::ConfigError(e.to_string()))?;

        println!("[telegram] Sending request...");
        let response = client.get(url)
            .header("Accept", "text/html; charset=utf-8")
            .header("Accept-Language", "zh-CN,zh;q=0.9,en;q=0.8")
            .send()
            .await
            .map_err(|e| {
                println!("[telegram] Request failed: {}", e);
                SourceError::NetworkError(e.to_string())
            })?;

        let status = response.status();
        println!("[telegram] Response status: {}", status);
        
        if status.as_u16() == 429 {
            return Err(SourceError::NetworkError("Rate limited".to_string()));
        }
        if !status.is_success() {
            return Err(SourceError::NetworkError(format!("HTTP {}", status)));
        }

        let text = response.text().await.map_err(|e| {
            println!("[telegram] Read body failed: {}", e);
            SourceError::NetworkError(e.to_string())
        })?;
        
        println!("[telegram] Received {} bytes", text.len());
        Ok(text)
    }
}

#[async_trait]
impl SourceAdapter for TelegramAdapter {
    fn adapter_type(&self) -> SourceType {
        SourceType::Telegram
    }

    async fn fetch(&mut self, url: &str, proxy: Option<&str>) -> Result<Vec<RawContent>, SourceError> {
        let base_url = Self::normalize_url(url)
            .ok_or_else(|| SourceError::ParseError("Invalid Telegram URL".to_string()))?;

        println!("[telegram] Fetching: {}", base_url);
        tokio::time::sleep(Duration::from_millis(500)).await;

        let mut all_results = Vec::new();
        let max_pages = 10;
        let mut last_msg_id: Option<i64> = None;

        for page in 1..=max_pages {
            let fetch_url = match last_msg_id {
                Some(id) => format!("{}?before={}", base_url, id),
                None => base_url.clone(),
            };

            println!("[telegram] Page {}: {}", page, fetch_url);
            
            let html = self.http_get(&fetch_url, proxy).await?;
            if html.len() < 1000 {
                println!("[telegram] Page {} too short, stopping", page);
                break;
            }

            let (results, new_last_id) = self.extract_messages_with_next(&html);
            println!("[telegram] Page {} extracted {} messages", page, results.len());
            
            if results.is_empty() {
                println!("[telegram] No more messages, stopping");
                break;
            }

            all_results.extend(results);
            last_msg_id = new_last_id;
            
            if last_msg_id.is_none() {
                println!("[telegram] No more pages, stopping");
                break;
            }

            tokio::time::sleep(Duration::from_secs(1)).await;
        }

        let mut seen = std::collections::HashSet::new();
        all_results.retain(|r| seen.insert(r.title.clone()));

        println!("[telegram] Total: {} messages", all_results.len());
        Ok(all_results)
    }
}
