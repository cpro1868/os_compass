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

pub async fn parse_content_with_llm(raw_content: &RawContent, settings: &AppSettings) -> Result<Vec<ParsedProjectInfo>, String> {
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

    let response = client.chat(messages).await
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

pub async fn ask_llm_direct(prompt: &str, settings: &AppSettings) -> Result<String, String> {
    if settings.llm_api_key.is_empty() || settings.llm_api_base.is_empty() {
        return Err("LLM 未配置 API Key 或 Base URL".to_string());
    }
    let client = LlmClient::from_settings()
        .ok_or_else(|| "LLM not configured".to_string())?;
    let messages = vec![LlmMessage {
        role: "user".to_string(),
        content: prompt.to_string(),
    }];
    client.chat(messages).await.map_err(|e| format!("LLM 调用失败: {}", e))
}

pub async fn recommend_projects_with_llm(query: &str, settings: &AppSettings, limit: usize) -> Result<Vec<ParsedProjectInfo>, String> {
    if settings.llm_api_key.is_empty() || settings.llm_api_base.is_empty() {
        return Ok(Vec::new());
    }

    let client = LlmClient::from_settings()
        .ok_or_else(|| "LLM not configured".to_string())?;

    let prompt = format!(
        r#"你是开源项目推荐助手。用户搜索：「{query}」。

请直接根据你的知识推荐 {limit} 个真实存在的开源项目（优先 GitHub 上活跃维护的项目）。
要求：
- 项目必须真实存在，URL 必须是 GitHub/Gitee 上的实际仓库地址
- 按推荐顺序输出，name 可使用 "owner/repo" 格式
- description 简洁说明项目用途，不超过 80 字
- language 填写主要编程语言（如不知道写 "Unknown"）
- keywords 抽取 3-5 个搜索相关关键词

严格按以下 JSON 格式返回，不要包含任何其它内容（不要 Markdown 代码块，不要解释）：
{{
  "projects": [
    {{
      "project_name": "owner/repo",
      "project_url": "https://github.com/owner/repo",
      "description": "项目描述",
      "language": "Rust",
      "keywords": ["k1", "k2", "k3"]
    }}
  ]
}}"#
    );

    let messages = vec![LlmMessage {
        role: "user".to_string(),
        content: prompt,
    }];

    let response = client.chat(messages).await
        .map_err(|e| format!("LLM 推荐失败: {}", e))?;

    let cleaned = strip_code_fence(&response);

    let parsed: LlmParseResponse = serde_json::from_str(cleaned)
        .map_err(|e| format!("解析 LLM 推荐结果失败: {} | 内容: {}", e, response.chars().take(200).collect::<String>()))?;

    let results: Vec<ParsedProjectInfo> = parsed.projects.into_iter()
        .take(limit)
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

fn strip_code_fence(s: &str) -> &str {
    let trimmed = s.trim();
    let s = if trimmed.starts_with("```") {
        if let Some(end) = trimmed.find('\n') {
            &trimmed[end + 1..]
        } else {
            trimmed
        }
    } else {
        trimmed
    };
    if let Some(pos) = s.rfind("```") {
        &s[..pos]
    } else {
        s
    }
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
