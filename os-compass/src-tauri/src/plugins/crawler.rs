use crate::crawler_service;
use crate::plugins::ImportResult;

pub async fn fetch_with_crawler(url: &str) -> ImportResult {
    let settings = crate::settings::get_settings();
    let api_url = settings.crawler_api_url;
    
    let base_url = if api_url.is_empty() {
        crawler_service::get_crawler_url().unwrap_or_else(|| "http://127.0.0.1:8080".to_string())
    } else {
        api_url.trim_end_matches('/').to_string()
    };
    
    let convert_url = format!("{}/convert", base_url);
    
    #[derive(serde::Serialize)]
    struct ConvertRequest {
        url: String,
    }
    
    #[derive(serde::Deserialize)]
    struct ConvertResponse {
        title: Option<String>,
        markdown: Option<String>,
        #[allow(dead_code)]
        source: Option<String>,
        #[allow(dead_code)]
        fetched_at: Option<String>,
        error: Option<String>,
    }
    
    let client = reqwest::Client::new();
    
    match client.post(&convert_url)
        .header("Content-Type", "application/json")
        .json(&ConvertRequest { url: url.to_string() })
        .send()
        .await
    {
        Ok(resp) => {
            if resp.status() == 200 {
                if let Ok(data) = resp.json::<ConvertResponse>().await {
                    if let Some(err_msg) = data.error {
                        return ImportResult {
                            success: false,
                            project_id: None,
                            error: Some(err_msg),
                            project_data: None,
                            readme_variants: None,
                        };
                    }
                    
                    let name = data.title.clone().unwrap_or_else(|| "Unknown".to_string());
                    let markdown = data.markdown.clone().unwrap_or_default();
                    
                    return ImportResult {
                        success: true,
                        project_id: None,
                        error: None,
                        project_data: Some(crate::plugins::ProjectData {
                            name,
                            url: url.to_string(),
                            source: "unknown".to_string(),
                            source_id: url.to_string(),
                            description: None,
                            languages: None,
                            stars: 0,
                            forks: 0,
                            open_issues: 0,
                            license: None,
                            homepage: None,
                            latest_commit: None,
                            latest_release: None,
                            readme_content: Some(markdown),
                        }),
                        readme_variants: None,
                    };
                }
            }
            
            ImportResult {
                success: false,
                project_id: None,
                error: Some("Crawler service returned invalid response".to_string()),
                project_data: None,
                readme_variants: None,
            }
        }
        Err(e) => ImportResult {
            success: false,
            project_id: None,
            error: Some(format!("Crawler error: {}", e)),
            project_data: None,
            readme_variants: None,
        },
    }
}
