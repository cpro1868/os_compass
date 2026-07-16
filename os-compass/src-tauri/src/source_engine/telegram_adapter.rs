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
            .replace("</div>", "\n")
            .replace("</li>", "\n");

        let re = Regex::new(r"<[^>]+>").unwrap();
        text = re.replace_all(&text, " ").to_string();

        let re2 = Regex::new(r"[ \t]+").unwrap();
        text = re2.replace_all(&text, " ").to_string();

        let re3 = Regex::new(r"\n{3,}").unwrap();
        text = re3.replace_all(&text, "\n\n").to_string();

        text.trim().to_string()
    }

    fn extract_links_from_text(&self, text: &str) -> Vec<String> {
        let patterns = [
            r"https?://(?:www\.)?github\.com/[\w\-]+/[\w\.\-]+",
            r"https?://(?:www\.)?gitee\.com/[\w\-]+/[\w\.\-]+",
            r"https?://(?:www\.)?gitlab\.com/[\w\-]+/[\w\.\-]+",
        ];

        let mut links = Vec::new();
        for pattern in &patterns {
            if let Ok(re) = Regex::new(pattern) {
                for cap in re.find_iter(text) {
                    links.push(cap.as_str().to_string());
                }
            }
        }
        links.dedup();
        links
    }

    fn parse_messages_from_html(&self, html: &str) -> Vec<RawContent> {
        let mut results = Vec::new();

        let parsers = [
            self.parse_widget_format(html),
            self.parse_legacy_format(html),
            self.parse_simple_format(html),
            self.parse_json_format(html),
        ];

        for parser_results in parsers {
            if !parser_results.is_empty() {
                results = parser_results;
                break;
            }
        }

        if results.is_empty() {
            println!("[telegram] No messages parsed with standard methods, trying text extraction...");
            results = self.parse_text_as_messages(html);
        }

        results
    }

    fn parse_widget_format(&self, html: &str) -> Vec<RawContent> {
        let mut results = Vec::new();

        let date_link_re = Regex::new(r#"<a[^>]*class="[^"]*tgme_widget_message_date[^"]*"[^>]*href="([^"]*)"[^>]*>.*?<time[^>]*datetime="([^"]*)""#).unwrap();

        for cap in date_link_re.captures_iter(html) {
            let link = cap.get(1).map(|m| m.as_str()).unwrap_or("");
            let datetime = cap.get(2).map(|m| m.as_str()).unwrap_or("");

            let full_match = cap.get(0).map(|m| m.as_str()).unwrap_or("");
            let start_pos = html.find(full_match).unwrap_or(0);
            let snippet_start = start_pos.saturating_sub(3000);
            let snippet = &html[snippet_start..start_pos + full_match.len()];

            let message_text_re = Regex::new(r#"class="[^"]*tgme_widget_message_text[^"]*"[^>]*>(.*?)</div>"#).unwrap();
            let text = message_text_re.captures(snippet)
                .and_then(|c| Some(self.html_to_text(&c.get(1).map(|m| m.as_str()).unwrap_or(""))))
                .filter(|t| !t.is_empty())
                .unwrap_or_default();

            if text.is_empty() {
                continue;
            }

            let links = self.extract_links_from_text(&text);
            let title = text.lines().next().unwrap_or(&text).chars().take(80).collect::<String>();
            let url = if link.starts_with("/") {
                format!("https://t.me{}", link)
            } else {
                link.to_string()
            };

            results.push(RawContent {
                title,
                url: if links.is_empty() { url } else { links.join(", ") },
                content: Some(text),
                published_at: Some(datetime.to_string()),
            });
        }

        println!("[telegram] Widget format: extracted {} messages", results.len());
        results
    }

    fn parse_legacy_format(&self, html: &str) -> Vec<RawContent> {
        let mut results = Vec::new();

        let re = Regex::new(r#"class="[^"]*message[^"]*"[^>]*data-date="([^"]*)"[^>]*>(.*?)</div>"#).unwrap();

        for cap in re.captures_iter(html) {
            let datetime = cap.get(1).map(|m| m.as_str()).unwrap_or("");
            let content = cap.get(2).map(|m| m.as_str()).unwrap_or("");
            let text = self.html_to_text(content);

            if text.len() < 10 {
                continue;
            }

            let links = self.extract_links_from_text(&text);
            let title = text.lines().next().unwrap_or(&text).chars().take(80).collect::<String>();

            results.push(RawContent {
                title,
                url: links.join(", "),
                content: Some(text),
                published_at: Some(datetime.to_string()),
            });
        }

        println!("[telegram] Legacy format: extracted {} messages", results.len());
        results
    }

    fn parse_simple_format(&self, html: &str) -> Vec<RawContent> {
        let mut results = Vec::new();

        let re = Regex::new(r#"<div[^>]*class="[^"]*message[^"]*"[^>]*>(.*?)</div>"#).unwrap();

        for cap in re.captures_iter(html) {
            let content = cap.get(1).map(|m| m.as_str()).unwrap_or("");
            let text = self.html_to_text(content);

            if text.len() < 20 {
                continue;
            }

            let links = self.extract_links_from_text(&text);
            if links.is_empty() {
                continue;
            }

            let title = text.lines().next().unwrap_or(&text).chars().take(80).collect::<String>();

            results.push(RawContent {
                title,
                url: links.join(", "),
                content: Some(text),
                published_at: None,
            });
        }

        println!("[telegram] Simple format: extracted {} messages", results.len());
        results
    }

    fn parse_json_format(&self, html: &str) -> Vec<RawContent> {
        let mut results = Vec::new();

        let re = Regex::new(r#"<script[^>]*type="application/json"[^>]*>(.*?)</script>"#).unwrap();

        for cap in re.captures_iter(html) {
            let json = cap.get(1).map(|m| m.as_str()).unwrap_or("");
            if json.contains("\"messages\"") || json.contains("\"text\"") {
                println!("[telegram] Found JSON data with messages");
            }
        }

        results
    }

    fn parse_text_as_messages(&self, html: &str) -> Vec<RawContent> {
        let mut results = Vec::new();
        let text = self.html_to_text(html);

        let lines: Vec<&str> = text.lines()
            .filter(|l| l.len() > 30)
            .collect();

        for line in lines {
            let links = self.extract_links_from_text(line);
            if !links.is_empty() {
                results.push(RawContent {
                    title: line.chars().take(80).collect(),
                    url: links.join(", "),
                    content: Some(line.to_string()),
                    published_at: None,
                });
            }

            if results.len() >= 50 {
                break;
            }
        }

        println!("[telegram] Text extraction: extracted {} messages", results.len());
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
            .header("Connection", "keep-alive")
            .header("Upgrade-Insecure-Requests", "1");

        if let Some(cookie) = cookie_value {
            request = request.header("Cookie", cookie);
        }

        let response = request.send().await
            .map_err(|e| SourceError::NetworkError(e.to_string()))?;

        let status = response.status();
        if status.as_u16() == 429 {
            return Err(SourceError::NetworkError("Rate limited, too many requests".to_string()));
        }
        if !status.is_success() {
            return Err(SourceError::NetworkError(format!("HTTP {}", status)));
        }

        if let Some(set_cookie) = response.headers().get("set-cookie") {
            if let Ok(cookie_str) = set_cookie.to_str() {
                if let Ok(re) = Regex::new(r"(stel_ssid|hash|zlgeo)=([^;]+)") {
                    for cap in re.captures_iter(cookie_str) {
                        let name = &cap[1];
                        let value = &cap[2];
                        let mut guard = self.cookie_store.lock().unwrap();
                        let current = guard.clone().unwrap_or_default();
                        if !current.contains(&format!("{}={}", name, value)) {
                            let new_cookie = if current.is_empty() {
                                format!("{}={};", name, value)
                            } else {
                                format!("{} {}={};", current, name, value)
                            };
                            *guard = Some(new_cookie);
                        }
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

        println!("[telegram] === Starting fetch ===");
        println!("[telegram] URL: {}", channel_url);
        if let Some(p) = proxy {
            println!("[telegram] Proxy: {}", p);
        }

        tokio::time::sleep(Duration::from_millis(800)).await;

        let html = self.http_get(&channel_url, proxy).await?;
        let html_len = html.len();
        println!("[telegram] Received {} bytes", html_len);

        if html_len < 1000 {
            println!("[telegram] ERROR: Response too short, might be blocked");
            println!("[telegram] First 500 chars: {}", &html[..html_len.min(500)]);
            return Ok(Vec::new());
        }

        let results = self.parse_messages_from_html(&html);

        if results.is_empty() {
            println!("[telegram] WARNING: No messages extracted!");
            println!("[telegram] HTML preview (first 1000 chars):");
            println!("{}", &html[..html_len.min(1000)]);
        } else {
            println!("[telegram] SUCCESS: Extracted {} messages", results.len());
        }

        Ok(results)
    }
}
