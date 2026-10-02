//! 项目探索（M24）Tauri 命令层

use crate::db::DATABASE;
use crate::explore;
use serde::Serialize;
use tauri::command;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExploreSource {
    #[serde(rename = "type")]
    pub source_type: String,   // readme_chunk / ai_report / runbook / notes
    pub chunk_index: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExploreAnswer {
    pub session_id: i64,
    pub answer: String,
    pub sources: Vec<ExploreSource>,
    pub degraded: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExploreSession {
    pub id: i64,
    pub project_id: i64,
    pub title: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExploreMessage {
    pub id: i64,
    pub session_id: i64,
    pub role: String,
    pub content: String,
    pub sources: Option<String>,
    pub created_at: String,
}

/// 核心问答：检索 → 组装 → LLM → 落库
#[command]
pub async fn ask_explore(project_id: i64, session_id: Option<i64>, question: String) -> Result<ExploreAnswer, String> {
    let question = question.trim().to_string();
    if question.is_empty() {
        return Err("问题不能为空".into());
    }
    // LLM 检查
    let llm = crate::llm::LlmClient::from_settings()
        .ok_or_else(|| "LLM 未配置，请先在设置中配置大模型".to_string())?;

    // 1. 索引就绪（内部不跨 await 持锁）
    let (_chunks, degraded) = explore::ensure_readme_index(project_id).await?;

    // 2. 会话解析/新建
    let sid = {
        let db_guard = DATABASE.lock().map_err(|e| e.to_string())?;
        let db = db_guard.as_ref().ok_or("Database not initialized")?;
        let conn = db.get_connection();
        match session_id {
            Some(id) => id,
            None => {
                let title: String = question.chars().take(30).collect();
                conn.execute(
                    "INSERT INTO explore_sessions (project_id, title) VALUES (?, ?)",
                    rusqlite::params![project_id, title],
                ).map_err(|e| e.to_string())?;
                conn.last_insert_rowid()
            }
        }
    };

    // 3. 资料 + 检索
    let materials = explore::load_materials(project_id)?;
    let hits = explore::retrieve_readme_chunks(project_id, &question).await;
    let readme_section = if !hits.is_empty() {
        let indices: Vec<i64> = hits.iter().map(|h| h.0).collect();
        explore::fetch_chunks_text(project_id, &indices)
    } else if degraded || !materials.readme.trim().is_empty() {
        // 降级：README 头部截断
        materials.readme.chars().take(explore::README_FALLBACK_TRUNCATE).collect()
    } else {
        String::new()
    };
    let system_prompt = explore::build_system_prompt(&materials, &readme_section, degraded);

    // 4. 历史（最近 6 轮 = 12 条）
    let history = {
        let db_guard = DATABASE.lock().map_err(|e| e.to_string())?;
        let db = db_guard.as_ref().ok_or("Database not initialized")?;
        let conn = db.get_connection();
        let mut stmt = conn.prepare(
            "SELECT role, content FROM explore_messages WHERE session_id = ? ORDER BY id DESC LIMIT ?"
        ).map_err(|e| e.to_string())?;
        let rows = stmt.query_map(rusqlite::params![sid, (explore::EXPLORE_HISTORY_TURNS * 2) as i64], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        }).map_err(|e| e.to_string())?;
        let mut v: Vec<(String, String)> = rows.flatten().collect();
        v.reverse();
        v
    };

    // 5. 调 LLM
    let mut messages = vec![crate::llm::LlmMessage { role: "system".into(), content: system_prompt }];
    for (role, content) in history {
        messages.push(crate::llm::LlmMessage { role, content });
    }
    messages.push(crate::llm::LlmMessage { role: "user".into(), content: question.clone() });
    let answer = llm.chat(messages).await?;

    // 6. 来源标注
    let mut sources: Vec<ExploreSource> = hits.iter().map(|h| ExploreSource {
        source_type: "readme_chunk".into(), chunk_index: Some(h.0),
    }).collect();
    if !materials.ai_report.trim().is_empty() {
        sources.push(ExploreSource { source_type: "ai_report".into(), chunk_index: None });
    }
    if !materials.runbook.trim().is_empty() {
        sources.push(ExploreSource { source_type: "runbook".into(), chunk_index: None });
    }
    if !materials.notes.trim().is_empty() {
        sources.push(ExploreSource { source_type: "notes".into(), chunk_index: None });
    }

    // 7. 落库（user + assistant）
    {
        let db_guard = DATABASE.lock().map_err(|e| e.to_string())?;
        let db = db_guard.as_ref().ok_or("Database not initialized")?;
        let conn = db.get_connection();
        conn.execute(
            "INSERT INTO explore_messages (session_id, role, content) VALUES (?, 'user', ?)",
            rusqlite::params![sid, question],
        ).map_err(|e| e.to_string())?;
        let sources_json = serde_json::to_string(&sources).map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT INTO explore_messages (session_id, role, content, sources) VALUES (?, 'assistant', ?, ?)",
            rusqlite::params![sid, answer, sources_json],
        ).map_err(|e| e.to_string())?;
        conn.execute(
            "UPDATE explore_sessions SET updated_at = datetime('now','localtime') WHERE id = ?",
            [sid],
        ).map_err(|e| e.to_string())?;
    }

    Ok(ExploreAnswer { session_id: sid, answer, sources, degraded })
}

/// 会话列表（updated_at 倒序）
#[command]
pub fn list_explore_sessions(project_id: i64) -> Result<Vec<ExploreSession>, String> {
    let db_guard = DATABASE.lock().map_err(|e| e.to_string())?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();
    let mut stmt = conn.prepare(
        "SELECT id, project_id, title, created_at, updated_at FROM explore_sessions WHERE project_id = ? ORDER BY updated_at DESC"
    ).map_err(|e| e.to_string())?;
    let rows = stmt.query_map([project_id], |row| Ok(ExploreSession {
        id: row.get(0)?, project_id: row.get(1)?, title: row.get(2)?,
        created_at: row.get(3)?, updated_at: row.get(4)?,
    })).map_err(|e| e.to_string())?;
    Ok(rows.flatten().collect())
}

/// 会话消息（时间升序）
#[command]
pub fn get_explore_messages(session_id: i64) -> Result<Vec<ExploreMessage>, String> {
    let db_guard = DATABASE.lock().map_err(|e| e.to_string())?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();
    let mut stmt = conn.prepare(
        "SELECT id, session_id, role, content, sources, created_at FROM explore_messages WHERE session_id = ? ORDER BY id ASC"
    ).map_err(|e| e.to_string())?;
    let rows = stmt.query_map([session_id], |row| Ok(ExploreMessage {
        id: row.get(0)?, session_id: row.get(1)?, role: row.get(2)?,
        content: row.get(3)?, sources: row.get(4)?, created_at: row.get(5)?,
    })).map_err(|e| e.to_string())?;
    Ok(rows.flatten().collect())
}

/// 删除会话（级联删消息）
#[command]
pub fn delete_explore_session(session_id: i64) -> Result<(), String> {
    let db_guard = DATABASE.lock().map_err(|e| e.to_string())?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();
    conn.execute("DELETE FROM explore_messages WHERE session_id = ?", [session_id]).map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM explore_sessions WHERE id = ?", [session_id]).map_err(|e| e.to_string())?;
    Ok(())
}

/// 最近活跃会话（重开恢复）
#[command]
pub fn get_latest_explore_session(project_id: i64) -> Result<Option<ExploreSession>, String> {
    let db_guard = DATABASE.lock().map_err(|e| e.to_string())?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();
    let mut stmt = conn.prepare(
        "SELECT id, project_id, title, created_at, updated_at FROM explore_sessions WHERE project_id = ? ORDER BY updated_at DESC LIMIT 1"
    ).map_err(|e| e.to_string())?;
    let mut rows = stmt.query_map([project_id], |row| Ok(ExploreSession {
        id: row.get(0)?, project_id: row.get(1)?, title: row.get(2)?,
        created_at: row.get(3)?, updated_at: row.get(4)?,
    })).map_err(|e| e.to_string())?;
    Ok(rows.next().and_then(|r| r.ok()))
}

/// 知识库状态（状态条）
#[command]
pub fn get_explore_kb_status(project_id: i64) -> Result<explore::ExploreKbStatus, String> {
    let db_guard = DATABASE.lock().map_err(|e| e.to_string())?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();
    let chunks: i64 = conn.query_row(
        "SELECT COUNT(*) FROM project_readme_chunks WHERE project_id = ?", [project_id], |r| r.get(0),
    ).unwrap_or(0);
    let (has_ai, has_rb): (i64, i64) = conn.query_row(
        "SELECT CASE WHEN COALESCE(ai_summary,'')<>'' THEN 1 ELSE 0 END, CASE WHEN COALESCE(runbook,'')<>'' THEN 1 ELSE 0 END FROM projects WHERE id = ?",
        [project_id], |r| Ok((r.get(0)?, r.get(1)?)),
    ).unwrap_or((0, 0));
    let notes: i64 = conn.query_row(
        "SELECT COUNT(*) FROM project_notes WHERE project_id = ?", [project_id], |r| r.get(0),
    ).unwrap_or(0);
    let embedded: i64 = conn.query_row(
        "SELECT COUNT(*) FROM project_readme_chunks WHERE project_id = ? AND embedding IS NOT NULL", [project_id], |r| r.get(0),
    ).unwrap_or(0);
    Ok(explore::ExploreKbStatus {
        ready: chunks > 0,
        building: false,
        degraded: !crate::embedding::is_enabled() || (chunks > 0 && embedded == 0),
        readme_chunks: chunks as usize,
        has_ai_report: has_ai == 1,
        has_runbook: has_rb == 1,
        notes_count: notes,
    })
}

/// 手动重建索引（删除后下次 ask 自动重建）
#[command]
pub fn rebuild_explore_index(project_id: i64) -> Result<(), String> {
    let db_guard = DATABASE.lock().map_err(|e| e.to_string())?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();
    conn.execute("DELETE FROM project_readme_chunks WHERE project_id = ?", [project_id])
        .map_err(|e| e.to_string())?;
    Ok(())
}
