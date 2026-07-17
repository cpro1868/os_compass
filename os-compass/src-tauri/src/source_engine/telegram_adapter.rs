use crate::source_engine::{RawContent, SourceAdapter, SourceError, SourceType};
use crate::system_db;
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

        let re = Regex::new(r"<[^>]+>").unwrap();
        text = re.replace_all(&text, " ").to_string();

        let re2 = Regex::new(r"[ \t]+").unwrap();
        text = re2.replace_all(&text, " ").to_string();

        let re3 = Regex::new(r"\n{3,}").unwrap();
        text = re3.replace_all(&text, "\n\n").to_string();

        text.trim().to_string()
    }

    fn extract_platform_links(&self, text: &str) -> Vec<String> {
        let patterns = Self::get_url_patterns();
        let mut links = Vec::new();

        for domain in &patterns {
            let escaped_domain = domain.replace(".", r"\.");
            let pattern = format!(r"https?://(?:www\.)?{}/[\w\-]+/[\w\.\-]+", escaped_domain);
            if let Ok(re) = Regex::new(&pattern) {
                for cap in re.find_iter(text) {
                    links.push(cap.as_str().to_string());
                }
            }
        }
        links.dedup();
        links
    }

    fn get_url_patterns() -> Vec<String> {
        if let Ok(plugins) = system_db::list_source_plugins() {
            let mut patterns = Vec::new();
            for plugin in plugins {
                if let Some(urls) = plugin.url_patterns {
                    for url in urls {
                        patterns.push(url);
                    }
                }
            }
            if patterns.is_empty() {
                patterns.push("github.com".to_string());
                patterns.push("gitee.com".to_string());
            }
            return patterns;
        }
        vec!["github.com".to_string(), "gitee.com".to_string()]
    }

    fn extract_messages(&self, html: &str) -> Vec<RawContent> {
        let mut results = Vec::new();
        let text = self.html_to_text(html);

        // 模式1：提取 Telegram widget 格式的消息
        // 每条消息包含：datetime、链接、内容
        let datetime_link_re = Regex::new(
            r#"<time[^>]*datetime="([^"]*)"[^>]*>.*?</time>.*?<a[^>]*href="([^"]*)""#
        ).unwrap();

        let message_blocks: Vec<&str> = text.split("\n\n")
            .filter(|block| block.len() > 20)
            .collect();

        for (idx, block) in message_blocks.iter().enumerate() {
            let content = block.trim();

            // 提取链接
            let links = self.extract_platform_links(content);

            // 跳过不包含目标平台链接的消息
            if links.is_empty() {
                continue;
            }

            // 提取标题（取第一行或前100字符）
            let title = content.lines()
                .next()
                .unwrap_or(content)
                .chars()
                .take(100)
                .collect::<String>();

            // 生成时间戳（如果能从 block 中提取）
            let published_at = self.extract_datetime_from_block(block);

            results.push(RawContent {
                title,
                url: links.join(", "),
                content: Some(content.to_string()),
                published_at,
            });

            // 限制每页提取数量
            if results.len() >= 50 {
                break;
            }
        }

        // 模式2：如果模式1没有结果，尝试从 HTML 中直接提取
        if results.is_empty() {
            results = self.extract_from_html_direct(html);
        }

        println!("[telegram] Extracted {} messages", results.len());
        results
    }

    fn extract_datetime_from_block(&self, block: &str) -> Option<String> {
        let re = Regex::new(r"\d{4}-\d{2}-\d{2}[T ]\d{2}:\d{2}:\d{2}").ok()?;
        re.find(block).map(|m| m.as_str().to_string())
    }

    fn extract_from_html_direct(&self, html: &str) -> Vec<RawContent> {
        let mut results = Vec::new();

        // 提取所有包含目标平台链接的段落
        let patterns = Self::get_url_patterns();
        let mut link_patterns = Vec::new();
        for domain in &patterns {
            let escaped = domain.replace(".", r"\.");
            link_patterns.push(format!(r"https?://(?:www\.)?{}/[\w\-]+/[\w\.\-]+", escaped));
        }

        for pattern in &link_patterns {
            if let Ok(re) = Regex::new(pattern) {
                let mut last_pos = 0;
                for cap in re.find_iter(html) {
                    let start = cap.start().saturating_sub(500);
                    let end = (cap.end() + 500).min(html.len());
                    let snippet = &html[start..end];
                    let text = self.html_to_text(snippet);

                    if text.len() > 30 {
                        let title = text.lines()
                            .next()
                            .unwrap_or(&text)
                            .chars()
                            .take(80)
                            .collect::<String>();

                        let links = self.extract_platform_links(&text);

                        results.push(RawContent {
                            title,
                            url: links.join(", "),
                            content: Some(text),
                            published_at: None,
                        });
                    }

                    last_pos = cap.end();
                }
            }
        }

        results.dedup_by(|a, b| a.url == b.url);
        results.truncate(50);

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

        // 保存 cookies
        if let Some(set_cookie) = response.headers().get("set-cookie") {
            if let Ok(cookie_str) = set_cookie.to_str() {
                let re = Regex::new(r"(stel_ssid|hash)=([^;]+)").ok();
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

        // 请求延迟，避免被限流
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
