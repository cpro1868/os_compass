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

    fn extract_messages(&self, html: &str) -> Vec<RawContent> {
        let mut results = Vec::new();

        let message_wrap_re = Regex::new(r#"<div class="tgme_widget_message_wrap[^>]*>(.*?)</div>\s*<div class="tgme_widget_message_footer"#).unwrap();

        for cap in message_wrap_re.captures_iter(html) {
            let message_html = &cap[1];

            let text_re = Regex::new(r#"<div class="tgme_widget_message_text[^>]*>(.*?)</div>"#).unwrap();
            let text = text_re.captures(message_html)
                .and_then(|c| Some(self.html_to_text(&c[1])))
                .unwrap_or_default();

            if text.len() < 5 {
                continue;
            }

            let datetime_re = Regex::new(r#"datetime="([^"]+)""#).unwrap();
            let published_at = datetime_re.captures(&cap[0])
                .map(|c| c[1].to_string());

            let link_re = Regex::new(r#"href="([^"]*)""#).unwrap();
            let urls: Vec<String> = link_re.captures_iter(message_html)
                .filter_map(|c| {
                    let href = &c[1];
                    if href.starts_with("http") {
                        Some(href.to_string())
                    } else {
                        None
                    }
                })
                .collect();

            let title = text.lines()
                .next()
                .unwrap_or(&text)
                .chars()
                .take(100)
                .collect::<String>();

            results.push(RawContent {
                title,
                url: urls.join(", "),
                content: Some(text),
                published_at,
            });

            if results.len() >= 100 {
                break;
            }
        }

        if results.is_empty() {
            results = self.extract_messages_fallback(html);
        }

        println!("[telegram] Extracted {} messages", results.len());
        results
    }

    fn extract_messages_fallback(&self, html: &str) -> Vec<RawContent> {
        let mut results = Vec::new();

        let msg_re = Regex::new(r#"tgme_widget_message[^>]*>(.*?)</div>\s*<div class="tgme_widget_message"#).unwrap();

        for cap in msg_re.captures_iter(html) {
            let content = &cap[1];

            let text = self.html_to_text(content);
            if text.len() < 10 {
                continue;
            }

            let title = text.lines()
                .next()
                .unwrap_or(&text)
                .chars()
                .take(100)
                .collect::<String>();

            let url_re = Regex::new(r#"https?://[^\s<>"']+""#).unwrap();
            let urls: Vec<String> = url_re.find_iter(content)
                .map(|m| m.as_str().to_string())
                .collect();

            results.push(RawContent {
                title,
                url: urls.join(", "),
                content: Some(text),
                published_at: None,
            });

            if results.len() >= 100 {
                break;
            }
        }

        println!("[telegram] Fallback extraction: {} messages", results.len());
        results
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
            .header("Accept", "text/html,application/xhtml+xml")
            .header("Accept-Language", "en-US,en;q=0.9")
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

        Ok(results)
    }
}
