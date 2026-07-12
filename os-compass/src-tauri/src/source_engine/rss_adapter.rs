use crate::source_engine::{build_client, RawContent, SourceAdapter, SourceError, SourceType};
use async_trait::async_trait;

pub struct RssAdapter;

impl RssAdapter {
    pub fn new() -> Self {
        RssAdapter
    }
}

fn extract_rss_items(xml: &str) -> Vec<RawContent> {
    let mut results = Vec::new();
    let items: Vec<_> = regex::Regex::new(r"<item>(.*?)</item>")
        .unwrap()
        .captures_iter(xml)
        .collect();

    for item in items {
        let item_content = item.get(1).map(|m| m.as_str()).unwrap_or("");
        let title = regex::Regex::new(r"<title[^>]*>(.*?)</title>")
            .unwrap()
            .captures(item_content)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().trim().to_string())
            .unwrap_or_default();

        let url = regex::Regex::new(r"<link[^>]*>(.*?)</link>")
            .unwrap()
            .captures(item_content)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().trim().to_string())
            .unwrap_or_default();

        if url.is_empty() {
            continue;
        }

        let content = regex::Regex::new(r"<description[^>]*>(.*?)</description>")
            .unwrap()
            .captures(item_content)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().to_string());

        let published_at = regex::Regex::new(r"<pubDate[^>]*>(.*?)</pubDate>")
            .unwrap()
            .captures(item_content)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().trim().to_string());

        results.push(RawContent {
            title,
            url,
            content,
            published_at,
        });
    }

    results
}

#[async_trait]
impl SourceAdapter for RssAdapter {
    fn adapter_type(&self) -> SourceType {
        SourceType::Rss
    }

    async fn fetch(&self, url: &str, proxy: Option<&str>) -> Result<Vec<RawContent>, SourceError> {
        let client = build_client(proxy)?;
        let response = client.get(url).send().await?;
        let xml = response.text().await?;

        let results = extract_rss_items(&xml);
        Ok(results)
    }
}
