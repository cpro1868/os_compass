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

/// 项目资料快照（全部来自 projects / project_notes 表）
pub struct ProjectMaterials {
    pub name: String,
    pub ai_report: String, // ai_summary + ai_use_cases + ai_risks 拼接，≤3000 字符
    pub runbook: String,   // ≤6000 字符
    pub notes: String,     // 全部笔记拼接，≤3000 字符
    pub readme: String,
}

pub fn truncate_chars(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        s.chars().take(max).collect::<String>() + "\n...(截断)"
    }
}

/// 加载项目资料（用户信息保险箱绝不读取）。短临界区，不跨 await 持锁。
pub fn load_materials(project_id: i64) -> Result<ProjectMaterials, String> {
    let db_guard = DATABASE.lock().map_err(|e| e.to_string())?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();
    let (name, ai_s, ai_u, ai_r, runbook, readme) = conn
        .query_row(
            "SELECT name, COALESCE(ai_summary,''), COALESCE(ai_use_cases,''), COALESCE(ai_risks,''), COALESCE(runbook,''), COALESCE(readme_content,'') FROM projects WHERE id = ?",
            [project_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                ))
            },
        )
        .map_err(|e| e.to_string())?;
    let notes = {
        let mut stmt = conn
            .prepare("SELECT content FROM project_notes WHERE project_id = ? ORDER BY updated_at DESC")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([project_id], |row| row.get::<_, String>(0))
            .map_err(|e| e.to_string())?;
        rows.flatten().collect::<Vec<_>>().join("\n\n---\n\n")
    };
    let ai_report = truncate_chars(
        &format!("总结：{}\n\n适用场景：{}\n\n风险：{}", ai_s, ai_u, ai_r),
        3000,
    );
    Ok(ProjectMaterials {
        name,
        ai_report,
        runbook: truncate_chars(&runbook, 6000),
        notes: truncate_chars(&notes, 3000),
        readme,
    })
}

/// 检索与问题最相关的 README 分块（Top-K 且 ≥ 阈值）。
/// 返回 (chunk_index, score) 列表；embedding 未启用或任一步失败返回空 Vec（走降级）。
pub async fn retrieve_readme_chunks(project_id: i64, question: &str) -> Vec<(i64, f32)> {
    if !crate::embedding::is_enabled() {
        return Vec::new();
    }
    let query_vec = match crate::embedding::generate_embedding(question).await {
        Ok(v) => v,
        Err(e) => {
            log::warn!("[explore] question embedding failed: {}", e);
            return Vec::new();
        }
    };
    let rows: Vec<(i64, String)> = {
        let db_guard = match DATABASE.lock() {
            Ok(g) => g,
            Err(_) => return Vec::new(),
        };
        let db = match db_guard.as_ref() {
            Some(d) => d,
            None => return Vec::new(),
        };
        let conn = db.get_connection();
        let mut stmt = match conn.prepare(
            "SELECT chunk_index, embedding FROM project_readme_chunks WHERE project_id = ? AND embedding IS NOT NULL",
        ) {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        };
        stmt.query_map([project_id], |row| Ok((row.get(0)?, row.get(1)?)))
            .map(|r| r.flatten().collect())
            .unwrap_or_default()
    };
    let mut scored: Vec<(i64, f32)> = rows
        .iter()
        .filter_map(|(idx, json)| {
            let vec: Vec<f32> = serde_json::from_str(json).ok()?;
            if vec.len() != query_vec.len() {
                return None; // 维度不一致跳过
            }
            let score = crate::embedding::cosine_similarity(&query_vec, &vec);
            if score >= EXPLORE_RETRIEVAL_MIN_SCORE {
                Some((*idx, score))
            } else {
                None
            }
        })
        .collect();
    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    scored.truncate(EXPLORE_TOP_K);
    scored
}

/// 组装探索对话 System Prompt。
/// degraded = true 时 README 段落使用头部截断（无向量检索）。
pub fn build_system_prompt(
    materials: &ProjectMaterials,
    readme_section: &str,
    degraded: bool,
) -> String {
    let mut prompt = format!(
        "你是开源项目「{}」的资深专家。以下是与该项目相关的本地资料。\n\
         规则：\n\
         1. 仅基于这些资料回答用户问题；资料未涉及时明确说明「项目资料中未提及」，再补充通用知识并标注为通用经验。\n\
         2. 回答使用 Markdown 格式，代码示例用代码块。\n\
         3. 承接多轮对话上下文，不机械重复已回答内容。\n",
        materials.name
    );
    if !materials.ai_report.trim().is_empty() {
        prompt.push_str(&format!("\n【AI 分析报告】\n{}\n", materials.ai_report));
    }
    if !materials.runbook.trim().is_empty() {
        prompt.push_str(&format!("\n【Runbook】\n{}\n", materials.runbook));
    }
    if !materials.notes.trim().is_empty() {
        prompt.push_str(&format!("\n【用户本地笔记】\n{}\n", materials.notes));
    }
    if !readme_section.trim().is_empty() {
        let label = if degraded {
            "README 摘要（截断）"
        } else {
            "README 相关片段"
        };
        prompt.push_str(&format!("\n【{}】\n{}\n", label, readme_section));
    }
    prompt
}

/// 按 chunk_index 列表取回分块文本并拼接
pub fn fetch_chunks_text(project_id: i64, indices: &[i64]) -> String {
    if indices.is_empty() {
        return String::new();
    }
    let db_guard = match DATABASE.lock() {
        Ok(g) => g,
        Err(_) => return String::new(),
    };
    let db = match db_guard.as_ref() {
        Some(d) => d,
        None => return String::new(),
    };
    let conn = db.get_connection();
    let mut out = Vec::new();
    for idx in indices {
        if let Ok(content) = conn.query_row(
            "SELECT content FROM project_readme_chunks WHERE project_id = ? AND chunk_index = ?",
            rusqlite::params![project_id, idx],
            |row| row.get::<_, String>(0),
        ) {
            out.push(content);
        }
    }
    out.join("\n\n...\n\n")
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

    #[test]
    fn prompt_omits_empty_sections() {
        let m = ProjectMaterials {
            name: "demo".into(),
            ai_report: "".into(),
            runbook: "run".into(),
            notes: "".into(),
            readme: "r".into(),
        };
        let p = build_system_prompt(&m, "chunk1", false);
        assert!(p.contains("demo"));
        assert!(p.contains("【Runbook】"));
        assert!(!p.contains("【AI 分析报告】"));
        assert!(!p.contains("【用户本地笔记】"));
        assert!(p.contains("【README 相关片段】"));
    }

    #[test]
    fn prompt_degraded_label() {
        let m = ProjectMaterials {
            name: "d".into(),
            ai_report: "".into(),
            runbook: "".into(),
            notes: "".into(),
            readme: "r".into(),
        };
        let p = build_system_prompt(&m, "x", true);
        assert!(p.contains("README 摘要（截断）"));
    }

    #[test]
    fn truncate_chars_multibyte() {
        let s = truncate_chars("你好世界", 2);
        assert!(s.starts_with("你好"));
        assert!(s.contains("截断"));
    }
}
