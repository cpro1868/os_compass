use crate::source_engine::{build_client, RawContent, SourceAdapter, SourceError, SourceType};
use async_trait::async_trait;
use log;

pub struct RssAdapter;

impl RssAdapter {
    pub fn new() -> Self {
        RssAdapter
    }
}

fn extract_text(xml: &str, tag: &str) -> Option<String> {
    let pattern = format!(r#"<{}[^>]*>([^<]*)</{}>"#, tag, tag);
    regex::Regex::new(&pattern)
        .ok()?
        .captures(xml)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().trim().to_string())
        .filter(|s| !s.is_empty())
}

fn extract_cdata(xml: &str, tag: &str) -> Option<String> {
    let pattern = format!(r#"<{}[^>]*><!\[CDATA\[([^\]]*)\]\]></{}>"#, tag, tag);
    regex::Regex::new(&pattern)
        .ok()?
        .captures(xml)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().to_string())
        .filter(|s| !s.is_empty())
}

fn extract_link_from_href(xml: &str) -> Option<String> {
    let pattern = r#"<link[^>]*href=["']([^"']*)["'][^>]*>"#;
    regex::Regex::new(pattern)
        .ok()?
        .captures(xml)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().to_string())
}

fn extract_rss_items(xml: &str) -> Vec<RawContent> {
    let mut results = Vec::new();

    let xml = xml.replace("\n", " ").replace("\r", " ");

    let item_patterns = [
        r"<item[^>]*>(.*?)</item>",
        r"<entry[^>]*>(.*?)</entry>",
    ];

    for pattern in &item_patterns {
        if let Ok(re) = regex::Regex::new(pattern) {
            let items: Vec<_> = re.captures_iter(&xml).collect();
            if !items.is_empty() {
                for item in items {
                    let item_content = item.get(1).map(|m| m.as_str()).unwrap_or("");

                    let title = extract_cdata(item_content, "title")
                        .or_else(|| extract_text(item_content, "title"))
                        .unwrap_or_default();

                    let url = extract_link_from_href(item_content)
                        .or_else(|| extract_text(item_content, "link"))
                        .unwrap_or_default();

                    if url.is_empty() {
                        continue;
                    }

                    let content = extract_cdata(item_content, "description")
                        .or_else(|| extract_cdata(item_content, "content:encoded"))
                        .or_else(|| extract_text(item_content, "description"))
                        .or_else(|| extract_text(item_content, "content:encoded"));

                    let published_at = extract_text(item_content, "pubDate")
                        .or_else(|| extract_text(item_content, "published"))
                        .or_else(|| extract_text(item_content, "updated"));

                    results.push(RawContent {
                        title,
                        url,
                        content,
                        published_at,
                    });
                }
                break;
            }
        }
    }

    log::debug!("[rss] Parsed {} items from XML ({} chars)", results.len(), xml.len());
    results
}

#[async_trait]
impl SourceAdapter for RssAdapter {
    fn adapter_type(&self) -> SourceType {
        SourceType::Rss
    }

    async fn fetch(&mut self, url: &str, proxy: Option<&str>, _time_range: Option<&str>) -> Result<Vec<RawContent>, SourceError> {
        log::debug!("[rss] Fetching: {}", url);

        const MAX_RETRIES: u32 = 3;
        const RETRY_DELAY_MS: u64 = 2000;

        for attempt in 1..=MAX_RETRIES {
            match self.try_fetch(url, proxy).await {
                Ok(results) => return Ok(results),
                Err(e) if attempt < MAX_RETRIES => {
                    log::warn!("[rss] Request failed (attempt {}/{}): {}, retrying in {}ms...",
                        attempt, MAX_RETRIES, e, RETRY_DELAY_MS);
                    tokio::time::sleep(std::time::Duration::from_millis(RETRY_DELAY_MS)).await;
                }
                Err(e) => {
                    log::error!("[rss] Request failed after {} attempts: {}", MAX_RETRIES, e);
                    return Err(e);
                }
            }
        }

        unreachable!()
    }
}

impl RssAdapter {
    async fn try_fetch(&self, url: &str, proxy: Option<&str>) -> Result<Vec<RawContent>, SourceError> {
        let client = build_client(proxy)?;
        let response = client.get(url).send().await
            .map_err(|e| SourceError::NetworkError(e.to_string()))?;
        log::debug!("[rss] Response status: {}", response.status());
        let xml = response.text().await
            .map_err(|e| SourceError::NetworkError(e.to_string()))?;
        log::debug!("[rss] XML length: {} bytes", xml.len());

        let results = extract_rss_items(&xml);
        log::debug!("[rss] Extracted {} items", results.len());
        Ok(results)
    }
}
