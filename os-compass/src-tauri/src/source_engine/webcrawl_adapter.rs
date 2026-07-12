use crate::source_engine::{build_client, RawContent, SourceAdapter, SourceError, SourceType};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
struct CrawlResponse {
    #[serde(rename = "markdown")]
    markdown: Option<String>,
    #[serde(rename = "content")]
    content: Option<String>,
    #[serde(rename = "text")]
    text: Option<String>,
    #[serde(rename = "title")]
    title: Option<String>,
}

pub struct WebCrawlAdapter;

impl WebCrawlAdapter {
    pub fn new() -> Self {
        WebCrawlAdapter
    }
}

#[async_trait]
impl SourceAdapter for WebCrawlAdapter {
    fn adapter_type(&self) -> SourceType {
        SourceType::WebCrawl
    }

    async fn fetch(&self, url: &str, proxy: Option<&str>) -> Result<Vec<RawContent>, SourceError> {
        let settings = crate::settings::get_settings();
        let crawler_api_url = if settings.crawler_api_url.is_empty() {
            "http://localhost:8080/crawl".to_string()
        } else {
            settings.crawler_api_url.clone()
        };

        let client = build_client(proxy)?;
        let response = client
            .post(&crawler_api_url)
            .json(&serde_json::json!({ "url": url }))
            .send()
            .await?;

        let crawl_response: CrawlResponse = response.json().await?;

        let markdown = crawl_response.markdown
            .or(crawl_response.content)
            .or(crawl_response.text)
            .unwrap_or_default();

        let title = crawl_response.title.unwrap_or_default();

        Ok(vec![RawContent {
            title,
            url: url.to_string(),
            content: Some(markdown),
            published_at: None,
        }])
    }
}
