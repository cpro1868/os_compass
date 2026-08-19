use crate::system_db::SYSTEM_DB;
use crate::llm::{LlmClient, LlmMessage};
use rusqlite::params;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use log;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdJudgment {
    pub is_ad: bool,
    pub confidence: f64,
    #[serde(default)]
    pub ad_keywords: Vec<String>,
    pub reasoning: String,
}

const AD_DETECTION_PROMPT: &str = r#"你是一个专业的广告识别专家。

请分析以下内容是否为广告：

标题：{title}
来源：{source_url}
摘要：{summary}

判断标准：
1. 商业推广内容（产品促销、服务推销）
2. 夸大宣传（绝对化用语、虚假承诺）
3. 诱导行为（限时优惠、立即行动）
4. 低质量内容（标题党、内容农场）

请返回 JSON 格式的分析结果：
{
  "is_ad": true/false,
  "confidence": 0.0-1.0,
  "ad_keywords": ["关键词1", "关键词2"],
  "reasoning": "判定理由"
}

请直接返回 JSON，不要包含其他内容。"#;

pub async fn detect_ad(
    title: &str,
    summary: &str,
    source_url: &str,
) -> Result<AdJudgment, String> {
    log::info!("[AdDetector] Starting ad detection for: {}", title);

    let client = LlmClient::from_settings().ok_or_else(|| {
        log::warn!("[AdDetector] LLM not configured");
        "LLM not configured".to_string()
    })?;

    let prompt = AD_DETECTION_PROMPT
        .replace("{title}", title)
        .replace("{source_url}", source_url)
        .replace("{summary}", summary);

    let messages = vec![LlmMessage {
        role: "user".to_string(),
        content: prompt,
    }];

    log::info!("[AdDetector] Calling LLM...");
    let response = client.chat(messages).await.map_err(|e| {
        log::error!("[AdDetector] LLM call failed: {}", e);
        format!("LLM request failed: {}", e)
    })?;

    log::info!("[AdDetector] LLM response: {}", response);

    parse_llm_response(&response)
}

fn parse_llm_response(response: &str) -> Result<AdJudgment, String> {
    let trimmed = response.trim();

    let json_start = trimmed.find('{').unwrap_or(0);
    let json_end = trimmed.rfind('}').map(|i| i + 1).unwrap_or(trimmed.len());

    let json_str = &trimmed[json_start..json_end];

    let parsed: serde_json::Value =
        serde_json::from_str(json_str).map_err(|e| {
            log::error!("[AdDetector] Failed to parse JSON: {}", e);
            format!("Failed to parse LLM response: {}", e)
        })?;

    let is_ad = parsed
        .get("is_ad")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let confidence = parsed
        .get("confidence")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.5);

    let reasoning = parsed
        .get("reasoning")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let ad_keywords: Vec<String> = parsed
        .get("ad_keywords")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();

    log::info!(
        "[AdDetector] Detection result: is_ad={}, confidence={}",
        is_ad,
        confidence
    );

    Ok(AdJudgment {
        is_ad,
        confidence,
        ad_keywords,
        reasoning,
    })
}

fn compute_url_hash(url: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(url.as_bytes());
    hex::encode(hasher.finalize())
}

pub fn url_in_whitelist(url: &str) -> bool {
    let result = (|| {
        let db_guard = SYSTEM_DB.lock().ok()?;
        let db = db_guard.as_ref()?;
        let conn = db.get_connection();
        let hash = compute_url_hash(url);
        let mut stmt = conn.prepare("SELECT 1 FROM ad_whitelist WHERE url_hash = ?1").ok()?;
        Some(stmt.exists(params![hash]).unwrap_or(false))
    })();
    result.unwrap_or(false)
}

pub fn add_pattern_internal(
    pattern_type: &str,
    pattern_value: &str,
    confidence: i32,
    source: &str,
) -> Option<i64> {
    let db_guard = SYSTEM_DB.lock().ok()?;
    let db = db_guard.as_ref()?;
    let conn = db.get_connection();

    conn.execute(
        "INSERT INTO ad_patterns (pattern_type, pattern_value, confidence, source) VALUES (?1, ?2, ?3, ?4)",
        params![pattern_type, pattern_value, confidence, source],
    ).ok()?;

    Some(conn.last_insert_rowid())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdMarkResult {
    pub success: bool,
    pub pattern_id: Option<i64>,
    pub llm_judgment: Option<AdJudgment>,
    pub message: String,
}

pub async fn mark_as_ad_internal(
    item_id: &str,
    title: &str,
    summary: &str,
    source_url: &str,
) -> Result<AdMarkResult, String> {
    log::info!(
        "[AdDetector] mark_as_ad_internal called: item_id={}, title={}",
        item_id,
        title
    );

    if url_in_whitelist(source_url) {
        log::info!("[AdDetector] URL is in whitelist, skipping");
        return Ok(AdMarkResult {
            success: true,
            pattern_id: None,
            llm_judgment: None,
            message: "该 URL 已在白名单中".to_string(),
        });
    }

    let judgment = detect_ad(title, summary, source_url).await?;

    if judgment.is_ad {
        let mut added_pattern_id = None;

        for keyword in &judgment.ad_keywords {
            if let Some(pattern_id) = add_pattern_internal(
                "keyword",
                keyword,
                (judgment.confidence * 100.0) as i32,
                "llm_learned",
            ) {
                added_pattern_id = Some(pattern_id);
                log::info!(
                    "[AdDetector] Added pattern: {} (confidence: {})",
                    keyword,
                    judgment.confidence
                );
            }
        }

        Ok(AdMarkResult {
            success: true,
            pattern_id: added_pattern_id,
            llm_judgment: Some(judgment),
            message: if added_pattern_id.is_some() {
                "已添加到屏蔽列表".to_string()
            } else {
                "LLM 判定为广告，但未能提取有效关键词".to_string()
            },
        })
    } else {
        log::info!("[AdDetector] LLM determined not an ad");
        Ok(AdMarkResult {
            success: true,
            pattern_id: None,
            llm_judgment: Some(judgment),
            message: "LLM 判定这不是广告".to_string(),
        })
    }
}
