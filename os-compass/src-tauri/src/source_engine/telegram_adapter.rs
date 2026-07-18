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

    fn get_time_range_seconds(time_range: Option<&str>) -> Option<i64> {
        match time_range {
            Some("1d") => Some(86400),
            Some("7d") => Some(604800),
            Some("30d") => Some(2592000),
            _ => None,
        }
    }

    fn parse_rfc3339(datetime_str: &str) -> Option<chrono::DateTime<chrono::Utc>> {
        chrono::DateTime::parse_from_rfc3339(datetime_str)
            .ok()
            .map(|dt| dt.with_timezone(&chrono::Utc))
    }

    fn extract_messages(&self, html: &str, time_range: Option<&str>, first_page: bool) -> (Vec<RawContent>, Option<i64>, bool) {
        let decoded = self.decode_html_entities(html);
        
        // 提取所有消息ID
        let msg_id_re = Regex::new(r#"data-post="[^/]+/(\d+)""#).unwrap();
        let msg_ids: Vec<i64> = msg_id_re.captures_iter(&decoded)
            .filter_map(|cap| cap[1].parse().ok())
            .collect();
        
        // 提取所有时间（按 HTML 顺序）
        let time_re = Regex::new(r#"<time datetime="([^"]+)""#).unwrap();
        let all_times: Vec<String> = time_re.captures_iter(&decoded)
            .map(|cap| cap[1].to_string())
            .collect();
        
        // 确定翻页用的 ID（最小ID = 最旧的消息）
        let next_last_id = msg_ids.iter().min().copied();
        
        // 确定时间范围
        let max_seconds = Self::get_time_range_seconds(time_range);
        
        // HTML 中顺序是：旧的在前，新的在后
        // first_page = true 时，all_times[0] 是最旧的，all_times[last] 是最新的
        
        // 提取消息内容
        let text_re = Regex::new(r#"class="tgme_widget_message_text[^"]*"[^>]*>([\s\S]*?)</div>"#).unwrap();
        let mut results = Vec::new();
        let mut earliest_time_in_page: Option<chrono::DateTime<chrono::Utc>> = None;
        let mut exceeds_range = false;
        
        // 时间数组的索引
        let mut time_idx = 0;
        
        for cap in text_re.captures_iter(&decoded) {
            let raw_text = &cap[1];
            let text = self.strip_html_tags(raw_text);
            
            if text.len() < 10 {
                continue;
            }
            
            // 获取对应的时间
            let published_at = if time_idx < all_times.len() {
                let utc_time = &all_times[time_idx];
                if let Some(dt) = Self::parse_rfc3339(utc_time) {
                    earliest_time_in_page = Some(dt);
                    Some(dt.with_timezone(&chrono::Local).format("%Y-%m-%d %H:%M:%S").to_string())
                } else {
                    None
                }
            } else {
                None
            };
            time_idx += 1;
            
            // 时间范围检查
            if let Some(max_sec) = max_seconds {
                if let Some(ref dt) = earliest_time_in_page {
                    let now = chrono::Utc::now();
                    let diff = now.signed_duration_since(*dt);
                    if diff.num_seconds() > max_sec {
                        exceeds_range = true;
                        continue;
                    }
                }
            }
            
            // 提取链接
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
            
            // 标题取第一行
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
        
        (results, next_last_id, exceeds_range)
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

    async fn fetch(&mut self, url: &str, proxy: Option<&str>, time_range: Option<&str>) -> Result<Vec<RawContent>, SourceError> {
        let base_url = Self::normalize_url(url)
            .ok_or_else(|| SourceError::ParseError("Invalid Telegram URL".to_string()))?;

        println!("[telegram] Fetching: {}, time_range={:?}", base_url, time_range);
        tokio::time::sleep(Duration::from_millis(500)).await;

        let mut all_results = Vec::new();
        let max_pages = 5;
        let mut last_msg_id: Option<i64> = None;
        let mut page_num = 0;

        loop {
            page_num += 1;
            if page_num > max_pages {
                println!("[telegram] Max pages reached, stopping");
                break;
            }
            
            let fetch_url = match last_msg_id {
                Some(id) => format!("{}?before={}", base_url, id),
                None => base_url.clone(),
            };

            println!("[telegram] Page {}: {}", page_num, fetch_url);
            
            let html = self.http_get(&fetch_url, proxy).await?;
            if html.len() < 1000 {
                println!("[telegram] Page {} too short, stopping", page_num);
                break;
            }

            let first_page = page_num == 1;
            let (results, new_last_id, exceeds_range) = self.extract_messages(&html, time_range, first_page);
            println!("[telegram] Page {} extracted {} messages, exceeds_range={}", page_num, results.len(), exceeds_range);
            
            if results.is_empty() {
                println!("[telegram] No messages in range, stopping");
                break;
            }

            all_results.extend(results);
            last_msg_id = new_last_id;
            
            // 如果设置了时间范围，且已达到范围边界，停止
            if time_range.is_some() && exceeds_range {
                println!("[telegram] Reached time range limit, stopping");
                break;
            }
            
            if last_msg_id.is_none() {
                println!("[telegram] No more pages, stopping");
                break;
            }

            tokio::time::sleep(Duration::from_secs(1)).await;
        }

        // 去重
        let mut seen = std::collections::HashSet::new();
        all_results.retain(|r| seen.insert(r.title.clone()));

        println!("[telegram] Total: {} messages", all_results.len());
        Ok(all_results)
    }
}
