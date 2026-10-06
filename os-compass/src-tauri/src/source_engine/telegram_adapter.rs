use crate::source_engine::{RawContent, SourceAdapter, SourceError, SourceType};
use async_trait::async_trait;
use regex::Regex;
use std::time::Duration;
use log;

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

    /// 将消息 HTML 中的 <a> 链接转换为保留边界的文本形式，避免剥标签后
    /// URL 与紧随其后的文本（如 "AI探索"）粘连丢失边界：
    /// - t.me 的 @/# 提及 → 只保留锚文本
    /// - 锚文本与 href 相同（客户端自动链接的 URL）→ 两侧补空格
    /// - 其他 → Markdown 链接 [锚文本](href)
    fn html_links_to_text(&self, html: &str) -> String {
        let a_re = Regex::new(r#"<a[^>]+href="([^"]+)"[^>]*>([\s\S]*?)</a>"#).unwrap();
        a_re.replace_all(html, |caps: &regex::Captures| {
            let href = caps[1].trim();
            let inner = self.strip_html_tags(&caps[2]);
            if href.contains("t.me/") && (inner.starts_with('@') || inner.starts_with('#')) {
                return inner;
            }
            if inner == href {
                return format!(" {} ", href);
            }
            format!("[{}]({})", inner, href)
        }).to_string()
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

    /// 提取正文中的链接。正文字符集排除 @，避免把链接后面紧跟的
    /// Telegram 用户名（如 https://pan.quark.cn/s/xxx@meiriyishu）吞进 URL。
    fn extract_urls(text: &str) -> Vec<String> {
        let url_re = Regex::new(r#"https?://[^\s<>"'@\u{3000}-\u{30ff}\u{3400}-\u{4dbf}\u{4e00}-\u{9fff}\uff00-\uffef]+[^<>\s.,;:!?@)。，；：！？”）】\u{3000}-\u{30ff}\u{3400}-\u{4dbf}\u{4e00}-\u{9fff}\uff00-\uffef]"#).unwrap();
        url_re.find_iter(text)
            .filter_map(|m| {
                let url = m.as_str();
                if url.contains("telegram.org") || url.contains("t.me/iyouport") {
                    None
                } else {
                    Some(url.to_string())
                }
            })
            .collect()
    }

    /// 提取消息块中的图片：照片/视频缩略图（background-image 形式）。
    /// 消息正文里的 <img> 是 emoji 表情，不属于内容图片，不提取。
    fn extract_images(block: &str) -> Vec<String> {
        let wrap_re = Regex::new(r#"tgme_widget_message_(?:photo|video)_wrap[^>]*?background-image:url\('([^']+)'\)"#).unwrap();
        wrap_re.captures_iter(block)
            .map(|cap| cap[1].to_string())
            .collect()
    }

    fn extract_messages(&self, html: &str, time_range: Option<&str>, _first_page: bool) -> (Vec<RawContent>, Option<i64>, bool) {
        let decoded = self.decode_html_entities(html);

        // 按消息块切分；类名前缀需排除 tgme_widget_message_date 等衍生类名。
        // Rust regex 不支持 lookahead，改为先找所有块起始位置再切片。
        let marker_re = Regex::new(r#"<div class="tgme_widget_message[\s"]"#).unwrap();
        let starts: Vec<usize> = marker_re.find_iter(&decoded).map(|m| m.start()).collect();
        let msg_id_re = Regex::new(r#"data-post="[^/]+/(\d+)""#).unwrap();
        let time_re = Regex::new(r#"<time datetime="([^"]+)""#).unwrap();
        let text_re = Regex::new(r#"class="tgme_widget_message_text[^"]*"[^>]*>([\s\S]*?)</div>"#).unwrap();

        let max_seconds = Self::get_time_range_seconds(time_range);

        let mut results = Vec::new();
        let mut all_msg_ids: Vec<i64> = Vec::new();
        let mut exceeds_range = false;

        // HTML 中顺序是：旧的在前，新的在后
        for (i, &start) in starts.iter().enumerate() {
            let end = starts.get(i + 1).copied().unwrap_or(decoded.len());
            let block = &decoded[start..end];

            if let Some(id_cap) = msg_id_re.captures(block) {
                if let Ok(id) = id_cap[1].parse::<i64>() {
                    all_msg_ids.push(id);
                }
            }

            let text = text_re.captures(block)
                .map(|c| self.strip_html_tags(&self.html_links_to_text(&c[1])))
                .unwrap_or_default();
            let images = Self::extract_images(block);

            // 纯图片消息（正文过短但有图）也保留
            if text.len() < 10 && images.is_empty() {
                continue;
            }

            let msg_dt = time_re.captures(block).and_then(|c| Self::parse_rfc3339(&c[1]));
            let published_at = msg_dt.map(|dt| {
                dt.with_timezone(&chrono::Local).format("%Y-%m-%d %H:%M:%S").to_string()
            });

            // 时间范围检查
            if let Some(max_sec) = max_seconds {
                if let Some(ref dt) = msg_dt {
                    let now = chrono::Utc::now();
                    let diff = now.signed_duration_since(*dt);
                    if diff.num_seconds() > max_sec {
                        exceeds_range = true;
                        continue;
                    }
                }
            }

            let urls = Self::extract_urls(&text);

            // 标题取第一行；纯图片消息退化为图片地址，避免空标题被去重合并
            let title = text.lines()
                .next()
                .map(|l| l.chars().take(80).collect::<String>())
                .filter(|l| !l.is_empty())
                .unwrap_or_else(|| images.first().cloned().unwrap_or_default());

            // 图片以 Markdown 语法附在正文末尾，前端识别后渲染缩略图
            let content = if images.is_empty() {
                text.clone()
            } else {
                let img_md = images.iter()
                    .map(|u| format!("![]({})", u))
                    .collect::<Vec<_>>()
                    .join("
");
                if text.is_empty() {
                    img_md
                } else {
                    format!("{}

{}", text, img_md)
                }
            };

            results.push(RawContent {
                title,
                url: urls.join(", "),
                content: Some(content),
                published_at,
            });

            if results.len() >= 30 {
                break;
            }
        }

        // 翻页用的 ID（最小ID = 最旧的消息），不受时间过滤影响
        let next_last_id = all_msg_ids.iter().min().copied();

        (results, next_last_id, exceeds_range)
    }

    async fn http_get(&self, url: &str, proxy: Option<&str>) -> Result<String, SourceError> {
        let proxy_info = proxy.unwrap_or("none");
        log::debug!("[telegram] http_get: url={}, proxy={}", url, proxy_info);

        const MAX_RETRIES: u32 = 3;
        const RETRY_DELAY_MS: u64 = 2000;

        for attempt in 1..=MAX_RETRIES {
            match self.http_get_once(url, proxy).await {
                Ok(result) => return Ok(result),
                Err(e) if attempt < MAX_RETRIES => {
                    log::warn!("[telegram] Request failed (attempt {}/{}): {}, retrying in {}ms...",
                        attempt, MAX_RETRIES, e, RETRY_DELAY_MS);
                    tokio::time::sleep(Duration::from_millis(RETRY_DELAY_MS)).await;
                }
                Err(e) => {
                    log::error!("[telegram] Request failed after {} attempts: {}", MAX_RETRIES, e);
                    return Err(e);
                }
            }
        }

        unreachable!()
    }

    async fn http_get_once(&self, url: &str, proxy: Option<&str>) -> Result<String, SourceError> {
        let mut builder = reqwest::Client::builder()
            .timeout(Duration::from_secs(60))
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36");

        if let Some(proxy_url) = proxy {
            if !proxy_url.is_empty() {
                log::debug!("[telegram] Setting proxy: {}", proxy_url);
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
                log::warn!("[telegram] Request failed: {}", e);
                SourceError::NetworkError(e.to_string())
            })?;

        let status = response.status();
        log::debug!("[telegram] Response status: {}", status);

        if status.as_u16() == 429 {
            return Err(SourceError::NetworkError("Rate limited".to_string()));
        }
        if !status.is_success() {
            return Err(SourceError::NetworkError(format!("HTTP {}", status)));
        }

        let text = response.text().await.map_err(|e| {
            log::warn!("[telegram] Read body failed: {}", e);
            SourceError::NetworkError(e.to_string())
        })?;
        
        log::debug!("[telegram] Received {} bytes", text.len());
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

        log::debug!("[telegram] Fetching: {}, time_range={:?}", base_url, time_range);
        tokio::time::sleep(Duration::from_millis(500)).await;

        let mut all_results = Vec::new();
        let max_pages = 5;
        let mut last_msg_id: Option<i64> = None;
        let mut page_num = 0;

        loop {
            page_num += 1;
            if page_num > max_pages {
                log::debug!("[telegram] Max pages reached, stopping");
                break;
            }
            
            let fetch_url = match last_msg_id {
                Some(id) => format!("{}?before={}", base_url, id),
                None => base_url.clone(),
            };

            log::debug!("[telegram] Page {}: {}", page_num, fetch_url);
            
            let html = self.http_get(&fetch_url, proxy).await?;
            if html.len() < 1000 {
                log::debug!("[telegram] Page {} too short, stopping", page_num);
                break;
            }

            let first_page = page_num == 1;
            let (results, new_last_id, exceeds_range) = self.extract_messages(&html, time_range, first_page);
            log::debug!("[telegram] Page {} extracted {} messages, exceeds_range={}", page_num, results.len(), exceeds_range);
            
            if results.is_empty() {
                log::debug!("[telegram] No messages in range, stopping");
                break;
            }

            all_results.extend(results);
            last_msg_id = new_last_id;
            
            // 如果设置了时间范围，且已达到范围边界，停止
            if time_range.is_some() && exceeds_range {
                log::debug!("[telegram] Reached time range limit, stopping");
                break;
            }
            
            if last_msg_id.is_none() {
                log::debug!("[telegram] No more pages, stopping");
                break;
            }

            tokio::time::sleep(Duration::from_secs(1)).await;
        }

        // 去重
        let mut seen = std::collections::HashSet::new();
        all_results.retain(|r| seen.insert(r.title.clone()));

        log::debug!("[telegram] Total: {} messages", all_results.len());
        Ok(all_results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_urls_splits_trailing_telegram_mention() {
        let text = "每日一书 分享 https://pan.quark.cn/s/e0ea9b70ef05@meiriyishu 欢迎关注";
        let urls = TelegramAdapter::extract_urls(text);
        assert_eq!(urls, vec!["https://pan.quark.cn/s/e0ea9b70ef05".to_string()]);
    }

    #[test]
    fn extract_urls_keeps_multiple_clean_urls() {
        let text = "项目地址 https://github.com/foo/bar 和镜像 https://gitee.com/foo/bar。";
        let urls = TelegramAdapter::extract_urls(text);
        assert_eq!(urls, vec![
            "https://github.com/foo/bar".to_string(),
            "https://gitee.com/foo/bar".to_string(),
        ]);
    }

    #[test]
    fn extract_urls_filters_telegram_org() {
        let text = "见 https://telegram.org/blog 和 https://example.com/a";
        let urls = TelegramAdapter::extract_urls(text);
        assert_eq!(urls, vec!["https://example.com/a".to_string()]);
    }

    #[test]
    fn extract_images_from_photo_wrap() {
        let block = r#"<a class="tgme_widget_message_photo_wrap" style="width:1200px;padding-top:56.3%;background-image:url('https://cdn4.cdn-telegram.org/file/abc123.jpg')"></a>"#;
        let imgs = TelegramAdapter::extract_images(block);
        assert_eq!(imgs, vec!["https://cdn4.cdn-telegram.org/file/abc123.jpg".to_string()]);
    }

    #[test]
    fn extract_images_multiple_and_ignore_userpic() {
        let block = r#"<div class="tgme_widget_message_userpic"><i style="background-image:url('https://cdn4.cdn-telegram.org/file/avatar.jpg')"></i></div>
<a class="tgme_widget_message_photo_wrap" style="background-image:url('https://cdn1.cdn-telegram.org/file/p1.jpg')"></a>
<a class="tgme_widget_message_photo_wrap" style="background-image:url('https://cdn1.cdn-telegram.org/file/p2.jpg')"></a>"#;
        let imgs = TelegramAdapter::extract_images(block);
        assert_eq!(imgs, vec![
            "https://cdn1.cdn-telegram.org/file/p1.jpg".to_string(),
            "https://cdn1.cdn-telegram.org/file/p2.jpg".to_string(),
        ]);
    }

    #[test]
    fn extract_messages_keeps_image_only_message() {
        let html = r#"<div class="tgme_widget_message js-widget_message" data-post="chan/101"><div class="tgme_widget_message_text"></div><a class="tgme_widget_message_photo_wrap" style="background-image:url('https://cdn4.cdn-telegram.org/file/pic.jpg')"></a><time datetime="2099-01-01T00:00:00+00:00"></time></div>"#;
        let adapter = TelegramAdapter::new();
        let (results, next_id, _) = adapter.extract_messages(html, None, true);
        assert_eq!(next_id, Some(101));
        assert_eq!(results.len(), 1);
        assert!(results[0].content.as_deref().unwrap().contains("![](https://cdn4.cdn-telegram.org/file/pic.jpg)"));
        assert_eq!(results[0].title, "https://cdn4.cdn-telegram.org/file/pic.jpg");
    }

    #[test]
    fn extract_urls_stops_at_cjk() {
        let text = "项目地址 https://example.com/abc探索频道 欢迎关注";
        let urls = TelegramAdapter::extract_urls(text);
        assert_eq!(urls, vec!["https://example.com/abc".to_string()]);
    }

    #[test]
    fn extract_messages_uses_anchor_boundary_for_url() {
        // Telegram 自动链接的 URL 在 HTML 中是 <a> 标签，边界以标签为准：
        // VoiceStudio 是链接，后面的 "AI探索" 是正文
        let html = r#"<div class="tgme_widget_message js-widget_message" data-post="chan/103"><div class="tgme_widget_message_text">推荐 <a href="https://github.com/debpalash/VoiceStudio">https://github.com/debpalash/VoiceStudio</a>AI探索 项目</div><time datetime="2099-01-01T00:00:00+00:00"></time></div>"#;
        let adapter = TelegramAdapter::new();
        let (results, _, _) = adapter.extract_messages(html, None, true);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].url, "https://github.com/debpalash/VoiceStudio");
        let content = results[0].content.as_deref().unwrap();
        assert!(content.contains("https://github.com/debpalash/VoiceStudio AI探索"));
    }

    #[test]
    fn html_links_to_text_keeps_mention_text_only() {
        let adapter = TelegramAdapter::new();
        let out = adapter.html_links_to_text(r#"链接 <a href="https://t.me/meiriyishu">@meiriyishu</a> 尾"#);
        assert_eq!(out, "链接 @meiriyishu 尾");
    }

    #[test]
    fn extract_messages_splits_mention_from_url_in_text() {
        let html = r#"<div class="tgme_widget_message js-widget_message" data-post="chan/102"><div class="tgme_widget_message_text">每日一书推荐 https://pan.quark.cn/s/e0ea9b70ef05@meiriyishu</div><time datetime="2099-01-01T00:00:00+00:00"></time></div>"#;
        let adapter = TelegramAdapter::new();
        let (results, _, _) = adapter.extract_messages(html, None, true);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].url, "https://pan.quark.cn/s/e0ea9b70ef05");
    }
}
