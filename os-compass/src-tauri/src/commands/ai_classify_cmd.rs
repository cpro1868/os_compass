use crate::db::DATABASE;
use crate::llm::LlmMessage;
use serde::{Deserialize, Serialize};

const MAX_DESC_LENGTH: usize = 2000;
const MAX_SUMMARY_LENGTH: usize = 1000;

#[derive(Debug, Serialize, Deserialize)]
pub struct ClassifyResult {
    pub category_id: i64,
    pub category_path: String,
    pub confidence: u8,
}

#[derive(Debug, Deserialize)]
struct CategoryInfo {
    pub id: i64,
    pub name: String,
    pub parent_id: Option<i64>,
}

fn build_category_path(categories: &[CategoryInfo], id: i64) -> String {
    let mut parts: Vec<String> = Vec::new();
    let mut current_id = Some(id);
    let mut visited = std::collections::HashSet::new();

    while let Some(cid) = current_id {
        if visited.contains(&cid) {
            break;
        }
        visited.insert(cid);
        
        if let Some(cat) = categories.iter().find(|c| c.id == cid) {
            parts.push(cat.name.clone());
            current_id = cat.parent_id;
        } else {
            break;
        }
    }

    parts.reverse();
    if parts.is_empty() {
        "未分类".to_string()
    } else {
        parts.join(" / ")
    }
}

#[tauri::command]
pub async fn ai_classify_project(project_id: i64) -> Result<ClassifyResult, String> {
    let (project_name, description, ai_summary, ai_use_cases, categories) = {
        let db_guard = DATABASE.try_lock().map_err(|e| e.to_string())?;
        let db = db_guard.as_ref().ok_or("Database not initialized")?;
        let conn = db.get_connection();

        let project_name: String = conn.query_row(
            "SELECT name FROM projects WHERE id = ?",
            [project_id],
            |row| row.get(0),
        ).map_err(|_| "PROJECT_INFO_INSUFFICIENT".to_string())?;

        let description: String = conn.query_row(
            "SELECT COALESCE(description, '') FROM projects WHERE id = ?",
            [project_id],
            |row| row.get(0),
        ).unwrap_or_default();

        let ai_summary: String = conn.query_row(
            "SELECT COALESCE(ai_summary, '') FROM projects WHERE id = ?",
            [project_id],
            |row| row.get(0),
        ).unwrap_or_default();

        let ai_use_cases: String = conn.query_row(
            "SELECT COALESCE(ai_use_cases, '') FROM projects WHERE id = ?",
            [project_id],
            |row| row.get(0),
        ).unwrap_or_default();

        let mut stmt = conn.prepare(
            "SELECT id, name, parent_id FROM categories ORDER BY id"
        ).map_err(|e| e.to_string())?;

        let categories: Vec<CategoryInfo> = stmt.query_map([], |row| {
            Ok(CategoryInfo {
                id: row.get(0)?,
                name: row.get(1)?,
                parent_id: row.get(2)?,
            })
        }).map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

        if categories.is_empty() {
            return Err("NO_CATEGORIES".to_string());
        }

        (project_name, description, ai_summary, ai_use_cases, categories)
    };

    let desc_truncated = if description.len() > MAX_DESC_LENGTH {
        description[..MAX_DESC_LENGTH].to_string()
    } else {
        description
    };

    let summary_truncated = if ai_summary.len() > MAX_SUMMARY_LENGTH {
        ai_summary[..MAX_SUMMARY_LENGTH].to_string()
    } else {
        ai_summary
    };

    let categories_list: Vec<String> = categories.iter()
        .map(|c| {
            let path = build_category_path(&categories, c.id);
            format!("- {} (ID: {})", path, c.id)
        })
        .collect();

    let prompt = format!(
        r#"你是一个开源项目分类助手。请根据项目信息，从给定的分类列表中选择最合适的分类。

项目信息：
- 名称：{}
- 描述：{}
- 适用场景：{}
- AI 评估摘要：{}

可用分类：
{}

请分析项目特征，从分类列表中选择最匹配的一个，返回分类 ID 和置信度（0-100）。

返回格式（JSON）：
{{"category_id": 数字, "confidence": 数字}}"#,
        project_name,
        desc_truncated,
        ai_use_cases,
        summary_truncated,
        categories_list.join("\n")
    );

    let client = crate::llm::LlmClient::from_settings()
        .ok_or_else(|| "LLM_NOT_CONFIGURED".to_string())?;

    let messages = vec![
        LlmMessage { role: "system".to_string(), content: "你是一个专业的开源项目分类助手。".to_string() },
        LlmMessage { role: "user".to_string(), content: prompt },
    ];

    let response = client.chat(messages)
        .await
        .map_err(|e| format!("LLM_API_ERROR: {}", e))?;

    let response_text = response.trim();

    let mut category_id: Option<i64> = None;
    let mut confidence: Option<u8> = None;

    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(response_text) {
        category_id = parsed["category_id"].as_i64().map(|v| v as i64);
        confidence = parsed["confidence"].as_u64().map(|v| v as u8);
    } else {
        let cleaned = response_text
            .trim_start_matches("```json")
            .trim_start_matches("```")
            .trim_end_matches("```")
            .trim()
            .to_string();

        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&cleaned) {
            category_id = parsed["category_id"].as_i64().map(|v| v as i64);
            confidence = parsed["confidence"].as_u64().map(|v| v as u8);
        } else {
            if let Some(id_start) = cleaned.find("category_id") {
                let slice = &cleaned[id_start..];
                for (i, c) in slice.char_indices() {
                    if c.is_ascii_digit() && i < 20 {
                        let num_str: String = slice.chars()
                            .skip(i)
                            .take_while(|c| c.is_ascii_digit())
                            .collect();
                        if !num_str.is_empty() {
                            category_id = num_str.parse().ok();
                            break;
                        }
                    }
                }
            }
            if let Some(conf_start) = cleaned.find("confidence") {
                let slice = &cleaned[conf_start..];
                for (i, c) in slice.char_indices() {
                    if c.is_ascii_digit() && i < 20 {
                        let num_str: String = slice.chars()
                            .skip(i)
                            .take_while(|c| c.is_ascii_digit())
                            .collect();
                        if !num_str.is_empty() {
                            confidence = num_str.parse().ok();
                            break;
                        }
                    }
                }
            }
        }
    }

    let category_id = category_id.ok_or("LLM_PARSE_ERROR: 无法解析 category_id")?;
    let confidence = confidence.unwrap_or(50);

    let category_path = build_category_path(&categories, category_id);

    {
        let db_guard = DATABASE.try_lock().map_err(|e| e.to_string())?;
        let db = db_guard.as_ref().ok_or("Database not initialized")?;
        let conn = db.get_connection();

        conn.execute(
            "UPDATE projects SET category_id = ?, updated_at = datetime('now', 'localtime') WHERE id = ?",
            rusqlite::params![category_id, project_id],
        ).map_err(|e| e.to_string())?;
    }

    Ok(ClassifyResult {
        category_id,
        category_path,
        confidence,
    })
}
