use crate::llm::{LlmClient, LlmMessage};
use crate::settings::AppSettings;
use crate::source_engine::RawContent;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedProjectInfo {
    pub project_name: Option<String>,
    pub project_url: Option<String>,
    pub description: Option<String>,
    pub language: Option<String>,
    pub keywords: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct LlmParseResponse {
    projects: Vec<LlmProjectItem>,
}

#[derive(Debug, Deserialize)]
struct LlmProjectItem {
    #[serde(rename = "project_name")]
    project_name: Option<String>,
    #[serde(rename = "project_url")]
    project_url: Option<String>,
    description: Option<String>,
    language: Option<String>,
    keywords: Option<Vec<String>>,
}

pub fn parse_content_with_llm(raw_content: &RawContent, settings: &AppSettings) -> Result<Vec<ParsedProjectInfo>, String> {
    if settings.llm_api_key.is_empty() || settings.llm_api_base.is_empty() {
        return Ok(Vec::new());
    }

    let client = LlmClient::from_settings()
        .ok_or_else(|| "LLM not configured".to_string())?;

    let prompt = build_parse_prompt(raw_content);

    let messages = vec![LlmMessage {
        role: "user".to_string(),
        content: prompt,
    }];

    let response = tauri::async_runtime::block_on(client.chat(messages))
        .map_err(|e| e.to_string())?;

    let parsed: LlmParseResponse = serde_json::from_str(&response)
        .map_err(|e| e.to_string())?;

    let results: Vec<ParsedProjectInfo> = parsed.projects.into_iter()
        .map(|p| ParsedProjectInfo {
            project_name: p.project_name,
            project_url: p.project_url,
            description: p.description,
            language: p.language,
            keywords: p.keywords.unwrap_or_default(),
        })
        .collect();

    Ok(results)
}

fn build_parse_prompt(raw_content: &RawContent) -> String {
    let content = raw_content.content.as_ref()
        .map(|c| c.chars().take(3000).collect::<String>())
        .unwrap_or_default();

    format!(
        r#"请从以下内容中提取开源项目信息，返回JSON数组格式：

内容来源：{}
内容：{}

请提取所有开源项目，提取格式：
{{
  "projects": [
    {{
      "project_name": "项目名称",
      "project_url": "GitHub或Gitee URL（如果有）",
      "description": "项目描述",
      "language": "主要编程语言（如果有）",
      "keywords": ["关键词1", "关键词2"]
    }}
  ]
}}

如果没有找到项目，返回空数组：{{"projects": []}}

只返回JSON，不要有其他内容。"#,
        raw_content.url,
        content
    )
}
