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

    fn decode_html_entities(&self, text: &str) -> String {
        let mut result = text.to_string();

        // HTML entity decode
        result = result.replace("&lt;", "<");
        result = result.replace("&gt;", ">");
        result = result.replace("&amp;", "&");
        result = result.replace("&quot;", "\"");
        result = result.replace("&apos;", "'");
        result = result.replace("&nbsp;", " ");
        result = result.replace("&#39;", "'");
        result = result.replace("&mdash;", "—");
        result = result.replace("&ndash;", "–");
        result = result.replace("&hellip;", "…");

        // Decode numeric HTML entities &#XXXX;
        let chars: Vec<char> = result.chars().collect();
        let mut final_result = String::new();
        let mut i = 0;
        while i < chars.len() {
            if chars[i] == '&' && i + 1 < chars.len() && chars[i + 1] == '#' {
                let mut j = i + 2;
                let mut num_str = String::new();
                while j < chars.len() && (chars[j].is_ascii_digit() || chars[j] == 'x' || chars[j] == 'X') {
                    num_str.push(chars[j]);
                    j += 1;
                }
                if j < chars.len() && chars[j] == ';' {
                    let parsed = if num_str.starts_with("x") || num_str.starts_with("X") {
                        u32::from_str_radix(&num_str[1..], 16).ok()
                    } else {
                        num_str.parse::<u32>().ok()
                    };
                    if let Some(val) = parsed.and_then(char::from_u32) {
                        final_result.push(val);
                        i = j + 1;
                        continue;
                    }
                }
            }
            final_result.push(chars[i]);
            i += 1;
        }

        final_result
    }

    fn strip_html_tags(&self, html: &str) -> String {
        let mut result = String::new();
        let mut in_tag = false;
        let mut in_quote = false;
        let mut last_was_space = false;
        
        for c in html.chars() {
            match c {
                '<' => { in_tag = true; }
                '>' => { in_tag = false; }
                '"' | '\'' => { in_quote = !in_quote; }
                _ if in_tag || in_quote => {}
                ' ' | '\t' | '\n' | '\r' => {
                    if !last_was_space {
                        result.push(' ');
                        last_was_space = true;
                    }
                }
                _ => {
                    result.push(c);
                    last_was_space = false;
                }
            }
        }
        
        // 合并多余空格并清理
        let mut cleaned = String::new();
        let mut space = false;
        for c in result.chars() {
            if c == ' ' || c == '\n' {
                if !space {
                    cleaned.push(' ');
                    space = true;
                }
            } else {
                cleaned.push(c);
                space = false;
            }
        }
        
        cleaned.trim().to_string()
    }

    fn extract_messages(&self, html: &str) -> Vec<RawContent> {
        let mut results = Vec::new();
        
        // 先解码 HTML 实体
        let decoded = self.decode_html_entities(html);
        
        // 提取消息文本块
        let text_blocks_re = regex::Regex::new(r#"class="tgme_widget_message_text[^"]*"[^>]*>([\s\S]*?)</div>"#).unwrap();
        
        for cap in text_blocks_re.captures_iter(&decoded) {
            let raw_text = &cap[1];
            let text = self.strip_html_tags(raw_text);
            
            if text.len() < 20 {
                continue;
            }
            
            // 提取外部链接
            let url_re = regex::Regex::new(r#"https?://[^\s<>"']+[^<>\s.,;:!?]"#).unwrap();
            let urls: Vec<String> = url_re.find_iter(&text)
                .filter(|m| {
                    let url = m.as_str();
                    !url.contains("telegram.org") && !url.contains("t.me/iyouport")
                })
                .map(|m| m.as_str().to_string())
                .collect();
            
            // 提取标题（第一行或前100字符）
            let title = text.lines()
                .next()
                .unwrap_or(&text)
                .chars()
                .take(80)
                .collect::<String>();
            
            // 提取时间戳
            let time_re = regex::Regex::new(r#"\d{4}-\d{2}-\d{2}[T ]\d{2}:\d{2}:\d{2}"#).unwrap();
            let published_at = time_re.find(&decoded).map(|m| m.as_str().to_string());
            
            results.push(RawContent {
                title,
                url: urls.join(", "),
                content: Some(text),
                published_at,
            });
            
            if results.len() >= 50 {
                break;
            }
        }
        
        println!("[telegram] Extracted {} messages", results.len());
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
            .header("Accept", "text/html; charset=utf-8")
            .header("Accept-Language", "zh-CN,zh;q=0.9,en;q=0.8")
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

        response.text()
            .await
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

        tokio::time::sleep(Duration::from_millis(500)).await;

        let html = self.http_get(&channel_url, proxy).await?;
        println!("[telegram] Received {} bytes", html.len());

        if html.len() < 1000 {
            return Ok(Vec::new());
        }

        let results = self.extract_messages(&html);
        Ok(results)
    }
}
