use serde::{Deserialize, Serialize};
use crate::settings;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Serialize)]
struct OpenAIRequest {
    model: String,
    messages: Vec<OpenAIMessage>,
}

#[derive(Debug, Serialize)]
struct OpenAIMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct OpenAIResponse {
    choices: Vec<OpenAIChoice>,
}

#[derive(Debug, Deserialize)]
struct OpenAIChoice {
    message: OpenAIMessageResponse,
}

#[derive(Debug, Deserialize)]
struct OpenAIMessageResponse {
    content: String,
}

pub struct LlmClient {
    api_url: String,
    api_key: String,
    model: String,
    proxy_url: Option<String>,
}

fn build_proxy_url(protocol: &str, host: &str, port: i32, username: &str, password: &str) -> Option<String> {
    if host.is_empty() {
        return None;
    }
    let auth = if !username.is_empty() {
        format!("{}:{}@", username, password)
    } else {
        String::new()
    };
    Some(format!("{}://{}{}:{}", protocol, auth, host, port))
}

impl LlmClient {
    pub fn from_settings() -> Option<Self> {
        let s = settings::get_settings();

        if s.llm_api_key.is_empty() {
            println!("[LLM] API key is empty, LLM not configured");
            return None;
        }

        println!("[LLM] Configured: provider={}, base={}, model={}, key_len={}, proxy_enabled={}",
            s.llm_provider, s.llm_api_base, s.llm_model, s.llm_api_key.len(), s.llm_proxy_enabled);

        let proxy_url = if s.llm_proxy_enabled {
            build_proxy_url(
                &s.llm_proxy_protocol,
                &s.llm_proxy_host,
                s.llm_proxy_port,
                &s.llm_proxy_username,
                &s.llm_proxy_password,
            )
        } else {
            None
        };

        Some(Self {
            api_url: s.llm_api_base,
            api_key: s.llm_api_key,
            model: s.llm_model,
            proxy_url,
        })
    }

    pub async fn chat(&self, messages: Vec<LlmMessage>) -> Result<String, String> {
        let openai_messages: Vec<OpenAIMessage> = messages
            .into_iter()
            .map(|m| OpenAIMessage {
                role: m.role,
                content: m.content,
            })
            .collect();

        let request = OpenAIRequest {
            model: self.model.clone(),
            messages: openai_messages,
        };

        let mut client_builder = reqwest::Client::builder();

        if let Some(ref proxy) = self.proxy_url {
            let proxy = reqwest::Proxy::all(proxy)
                .map_err(|e| format!("Invalid proxy URL: {}", e))?;
            client_builder = client_builder.proxy(proxy);
        }

        let client = client_builder
            .timeout(std::time::Duration::from_secs(60))
            .build()
            .map_err(|e| format!("Failed to create client: {}", e))?;

        let response = client
            .post(format!("{}/chat/completions", self.api_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(format!("API error {}: {}", status, body));
        }

        let chat_response: OpenAIResponse = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        Ok(chat_response.choices[0].message.content.clone())
    }
}

pub fn build_system_prompt() -> LlmMessage {
    LlmMessage {
        role: "system".to_string(),
        content: r#"You are an expert at analyzing open-source projects. Analyze the provided project information and return a JSON object with the following fields (all values must be plain text strings, NOT arrays):
- summary: A brief 2-3 sentence summary of what this project does
- use_cases: A comma-separated list of 3-5 ideal use cases for this project
- risks: A comma-separated list of 3-5 potential risks or concerns with using this project
- dependencies: A comma-separated list of key dependencies or requirements

Return ONLY valid JSON without any other text. Example: {"summary": "React is a JavaScript library...", "use_cases": "Web apps, Mobile apps, SPAs", "risks": "Learning curve, State management complexity", "dependencies": "Node.js, npm"}"#.to_string(),
    }
}

#[derive(Debug, Deserialize)]
struct OpenAIModelsResponse {
    data: Vec<OpenAIModel>,
}

#[derive(Debug, Deserialize)]
struct OpenAIModel {
    id: String,
}

pub async fn list_models(api_base: &str, api_key: &str) -> Result<Vec<String>, String> {
    let client = reqwest::Client::new();
    
    let response = client
        .get(format!("{}/models", api_base.trim_end_matches('/')))
        .header("Authorization", format!("Bearer {}", api_key))
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("API error: {}", response.status()));
    }

    let models: OpenAIModelsResponse = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    Ok(models.data.into_iter().map(|m| m.id).take(20).collect())
}

const README_FULL_THRESHOLD: usize = 8000;
const README_ANALYSIS_MAX: usize = 6000;
const README_TAGS_MAX: usize = 2000;
const README_RUNBOOK_MAX: usize = 12000;

pub fn preprocess_readme_for_analysis(readme: &str) -> String {
    let cleaned = clean_readme(readme);
    truncate_chars(&cleaned, README_ANALYSIS_MAX)
}

pub fn preprocess_readme_for_tags(readme: &str) -> String {
    let cleaned = clean_readme(readme);
    truncate_chars(&cleaned, README_TAGS_MAX)
}

pub fn preprocess_readme_for_runbook(readme: &str) -> String {
    let cleaned = clean_readme(readme);
    truncate_chars(&cleaned, README_RUNBOOK_MAX)
}

fn clean_readme(readme: &str) -> String {
    let mut result = String::with_capacity(readme.len());
    for line in readme.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("![") || trimmed.starts_with("<img ") {
            continue;
        }
        let cleaned_line = remove_inline_images(line);
        if cleaned_line.trim().is_empty() && result.ends_with("\n\n") {
            continue;
        }
        result.push_str(&cleaned_line);
        result.push('\n');
    }
    result.trim().to_string()
}

fn remove_inline_images(line: &str) -> String {
    let mut result = line.to_string();
    while let Some(start) = result.find("![") {
        if let Some(alt_end) = result[start..].find("](") {
            let alt_end = start + alt_end;
            if let Some(url_end) = result[alt_end..].find(')') {
                let url_end = alt_end + url_end;
                result = format!("{}{}", &result[..start], &result[url_end + 1..]);
                continue;
            }
        }
        break;
    }
    result
}

fn truncate_chars(text: &str, max_chars: usize) -> String {
    let chars: Vec<char> = text.chars().collect();
    if chars.len() <= max_chars {
        return text.to_string();
    }
    let truncated: String = chars.iter().take(max_chars).collect();
    format!("{}\n\n[... README truncated, showing first {} chars ...]", truncated, max_chars)
}

pub fn readme_size_category(readme_len: usize) -> &'static str {
    if readme_len <= README_FULL_THRESHOLD {
        "small"
    } else if readme_len <= README_RUNBOOK_MAX {
        "medium"
    } else {
        "large"
    }
}
