//! 项目探索（M24）：README 分块索引 + 上下文组装 + 会话管理核心逻辑
//! 安全红线：禁止读取 project_user_info 表任何字段注入 Prompt。

use crate::db::DATABASE;
use sha2::{Digest, Sha256};

/// README 分块目标大小（字符）
pub const README_CHUNK_TARGET: usize = 800;
/// 分块硬切上限（字符）
pub const README_CHUNK_MAX: usize = 1000;
/// 降级模式 README 截断长度（字符）
pub const README_FALLBACK_TRUNCATE: usize = 8000;
/// 探索检索相似度阈值（低于该值的分块不注入）
pub const EXPLORE_RETRIEVAL_MIN_SCORE: f32 = 0.35;
/// 检索注入的最大分块数
pub const EXPLORE_TOP_K: usize = 4;
/// 注入的对话历史轮数
pub const EXPLORE_HISTORY_TURNS: usize = 6;

/// 知识库状态（返回前端状态条）
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExploreKbStatus {
    pub ready: bool,
    pub building: bool,
    pub degraded: bool,
    pub readme_chunks: usize,
    pub has_ai_report: bool,
    pub has_runbook: bool,
    pub notes_count: i64,
}

/// README 内容 SHA-256（十六进制）
pub fn readme_hash(readme: &str) -> String {
    let mut h = Sha256::new();
    h.update(readme.as_bytes());
    hex::encode(h.finalize())
}

/// 按 Markdown 标题/段落切分 README。
/// 规则：标题行且当前块 ≥400 字符时切分；块超 1000 字符硬切；过小的尾块并入前一块。
pub fn chunk_readme(readme: &str) -> Vec<String> {
    let mut chunks: Vec<String> = Vec::new();
    let mut current = String::new();
    for line in readme.lines() {
        let is_heading = line.trim_start().starts_with('#');
        if is_heading && current.chars().count() >= README_CHUNK_TARGET / 2 {
            let trimmed = current.trim();
            if !trimmed.is_empty() {
                chunks.push(trimmed.to_string());
            }
            current = String::new();
        }
        current.push_str(line);
        current.push('\n');
        if current.chars().count() >= README_CHUNK_MAX {
            let trimmed = current.trim();
            if !trimmed.is_empty() {
                chunks.push(trimmed.to_string());
            }
            current = String::new();
        }
    }
    let trimmed = current.trim();
    if !trimmed.is_empty() {
        // 尾块过小则并入前一块，避免碎块
        if trimmed.chars().count() < README_CHUNK_TARGET / 2 && !chunks.is_empty() {
            let last = chunks.pop().unwrap_or_default();
            chunks.push(format!("{}\n{}", last, trimmed));
        } else {
            chunks.push(trimmed.to_string());
        }
    }
    chunks
}

/// 确保项目 README 分块索引就绪。
/// hash 一致 → 直接返回；不一致/缺失 → 重建分块并逐块向量化（embedding 未启用则只存文本块）。
/// 返回 (分块数, 是否降级)
pub async fn ensure_readme_index(project_id: i64) -> Result<(usize, bool), String> {
    // 1. 读 README（短临界区，不跨 await 持锁）
    let readme = {
        let db_guard = DATABASE.lock().map_err(|e| e.to_string())?;
        let db = db_guard.as_ref().ok_or("Database not initialized")?;
        let conn = db.get_connection();
        conn.query_row(
            "SELECT readme_content FROM projects WHERE id = ?",
            [project_id],
            |row| row.get::<_, Option<String>>(0),
        )
        .map_err(|e| e.to_string())?
        .unwrap_or_default()
    };
    if readme.trim().is_empty() {
        return Ok((0, !crate::embedding::is_enabled()));
    }
    let hash = readme_hash(&readme);

    // 2. hash 比对
    let existing = {
        let db_guard = DATABASE.lock().map_err(|e| e.to_string())?;
        let db = db_guard.as_ref().ok_or("Database not initialized")?;
        let conn = db.get_connection();
        conn.query_row(
            "SELECT COUNT(*), COALESCE(MAX(readme_hash), '') FROM project_readme_chunks WHERE project_id = ?",
            [project_id],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)),
        )
        .unwrap_or((0, String::new()))
    };
    if existing.0 > 0 && existing.1 == hash {
        return Ok((existing.0 as usize, !crate::embedding::is_enabled()));
    }

    // 3. 重建分块
    let chunks = chunk_readme(&readme);
    {
        let db_guard = DATABASE.lock().map_err(|e| e.to_string())?;
        let db = db_guard.as_ref().ok_or("Database not initialized")?;
        let conn = db.get_connection();
        conn.execute(
            "DELETE FROM project_readme_chunks WHERE project_id = ?",
            [project_id],
        )
        .map_err(|e| e.to_string())?;
        for (idx, content) in chunks.iter().enumerate() {
            conn.execute(
                "INSERT INTO project_readme_chunks (project_id, chunk_index, content, readme_hash) VALUES (?, ?, ?, ?)",
                rusqlite::params![project_id, idx as i64, content, hash],
            )
            .map_err(|e| e.to_string())?;
        }
    }

    // 4. 向量化（embedding 未启用则跳过，标记降级）
    let degraded = !crate::embedding::is_enabled();
    if !degraded {
        for (idx, content) in chunks.iter().enumerate() {
            match crate::embedding::generate_embedding(content).await {
                Ok(vec) => {
                    let json = serde_json::to_string(&vec).map_err(|e| e.to_string())?;
                    let db_guard = DATABASE.lock().map_err(|e| e.to_string())?;
                    let db = db_guard.as_ref().ok_or("Database not initialized")?;
                    let conn = db.get_connection();
                    conn.execute(
                        "UPDATE project_readme_chunks SET embedding = ? WHERE project_id = ? AND chunk_index = ?",
                        rusqlite::params![json, project_id, idx as i64],
                    )
                    .map_err(|e| e.to_string())?;
                }
                Err(e) => {
                    log::warn!("[explore] chunk {} embedding failed: {}", idx, e);
                }
            }
        }
    }
    Ok((chunks.len(), degraded))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunk_empty_readme() {
        assert!(chunk_readme("").is_empty());
        assert!(chunk_readme("   \n  ").is_empty());
    }

    #[test]
    fn chunk_short_readme_single_block() {
        let chunks = chunk_readme("# Title\n\nSome intro text.");
        assert_eq!(chunks.len(), 1);
        assert!(chunks[0].contains("Some intro text."));
    }

    #[test]
    fn chunk_splits_on_headings() {
        let body = "x".repeat(600);
        let readme = format!("# A\n{}\n# B\n{}", body, body);
        let chunks = chunk_readme(&readme);
        assert!(
            chunks.len() >= 2,
            "headings should split chunks: got {}",
            chunks.len()
        );
    }

    #[test]
    fn chunk_hard_cut_over_max() {
        let readme = "y".repeat(5000);
        let chunks = chunk_readme(&readme);
        assert!(chunks.len() >= 5);
        assert!(chunks.iter().all(|c| c.chars().count() <= README_CHUNK_MAX));
    }

    #[test]
    fn readme_hash_stable() {
        assert_eq!(readme_hash("abc"), readme_hash("abc"));
        assert_ne!(readme_hash("abc"), readme_hash("abd"));
    }
}
