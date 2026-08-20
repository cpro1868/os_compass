use crate::system_db::{SYSTEM_DB, SystemDb};
use crate::services::ad_detector;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tauri::command;

pub use ad_detector::AdJudgment;
pub use ad_detector::AdMarkResult;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdPattern {
    pub id: i64,
    pub pattern_type: String,
    pub pattern_value: String,
    pub confidence: i32,
    pub source: String,
    pub hit_count: i32,
    #[serde(default)]
    pub consecutive_hits: i32,
    pub last_hit_at: Option<String>,
    pub enabled: bool,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdWhitelist {
    pub id: i64,
    pub url_hash: String,
    pub source_url: Option<String>,
    pub note: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdCheckResult {
    pub is_blocked: bool,
    pub matched_patterns: Vec<MatchedPattern>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchedPattern {
    pub id: i64,
    #[serde(rename = "type")]
    pub pattern_type: String,
    pub value: String,
    pub confidence: i32,
}

fn compute_url_hash(url: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(url.as_bytes());
    hex::encode(hasher.finalize())
}

#[command]
pub async fn mark_as_ad(
    item_id: String,
    title: String,
    summary: String,
    source_url: String,
) -> Result<AdMarkResult, String> {
    log::info!("[mark_as_ad] item_id={}, title={}", item_id, title);
    ad_detector::mark_as_ad_internal(&item_id, &title, &summary, &source_url).await
}

fn row_to_pattern(row: &rusqlite::Row) -> rusqlite::Result<AdPattern> {
    Ok(AdPattern {
        id: row.get(0)?,
        pattern_type: row.get(1)?,
        pattern_value: row.get(2)?,
        confidence: row.get(3)?,
        source: row.get(4)?,
        hit_count: row.get(5)?,
        consecutive_hits: row.get(6)?,
        last_hit_at: row.get(7)?,
        enabled: row.get::<_, i32>(8)? == 1,
        created_at: row.get(9)?,
    })
}

#[command]
pub fn list_ad_patterns(
    pattern_type: Option<String>,
    enabled: Option<bool>,
) -> Result<Vec<AdPattern>, String> {
    let db_guard = SYSTEM_DB.lock().map_err(|e| e.to_string())?;
    let db = db_guard.as_ref().ok_or("System DB not initialized")?;
    let conn = db.get_connection();

    let mut sql = String::from(
        "SELECT id, pattern_type, pattern_value, confidence, source, hit_count, consecutive_hits, last_hit_at, enabled, created_at FROM ad_patterns WHERE 1=1",
    );

    if pattern_type.is_some() {
        sql.push_str(" AND pattern_type = ?1");
    }
    if enabled.is_some() {
        sql.push_str(" AND enabled = ?2");
    }
    sql.push_str(" ORDER BY confidence DESC, hit_count DESC");

    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;

    let patterns: Vec<AdPattern> = match (pattern_type.as_deref(), enabled) {
        (Some(t), Some(e)) => stmt
            .query_map(params![t, if e { 1 } else { 0 }], row_to_pattern)
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect(),
        (Some(t), None) => stmt
            .query_map(params![t], row_to_pattern)
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect(),
        (None, Some(e)) => stmt
            .query_map(params![if e { 1 } else { 0 }], row_to_pattern)
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect(),
        (None, None) => stmt
            .query_map([], row_to_pattern)
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect(),
    };

    Ok(patterns)
}

#[command]
pub fn add_ad_pattern(
    pattern_type: String,
    pattern_value: String,
    confidence: Option<i32>,
    source: Option<String>,
) -> Result<AdPattern, String> {
    let db_guard = SYSTEM_DB.lock().map_err(|e| e.to_string())?;
    let db = db_guard.as_ref().ok_or("System DB not initialized")?;
    let conn = db.get_connection();

    let conf = confidence.unwrap_or(50);
    let src = source.unwrap_or_else(|| "manual".to_string());

    conn.execute(
        "INSERT INTO ad_patterns (pattern_type, pattern_value, confidence, source) VALUES (?1, ?2, ?3, ?4)",
        params![pattern_type, pattern_value, conf, src],
    ).map_err(|e| e.to_string())?;

    let id = conn.last_insert_rowid();

    let mut stmt = conn
        .prepare("SELECT id, pattern_type, pattern_value, confidence, source, hit_count, consecutive_hits, last_hit_at, enabled, created_at FROM ad_patterns WHERE id = ?1")
        .map_err(|e| e.to_string())?;

    stmt.query_row(params![id], row_to_pattern).map_err(|e| e.to_string())
}

#[command]
pub fn update_ad_pattern(id: i64, enabled: Option<bool>, confidence: Option<i32>) -> Result<(), String> {
    let db_guard = SYSTEM_DB.lock().map_err(|e| e.to_string())?;
    let db = db_guard.as_ref().ok_or("System DB not initialized")?;
    let conn = db.get_connection();

    if let Some(e) = enabled {
        conn.execute(
            "UPDATE ad_patterns SET enabled = ?1, updated_at = datetime('now', 'localtime') WHERE id = ?2",
            params![if e { 1 } else { 0 }, id],
        ).map_err(|e| e.to_string())?;
    }

    if let Some(c) = confidence {
        conn.execute(
            "UPDATE ad_patterns SET confidence = ?1, updated_at = datetime('now', 'localtime') WHERE id = ?2",
            params![c, id],
        ).map_err(|e| e.to_string())?;
    }

    Ok(())
}

#[command]
pub fn delete_ad_pattern(id: i64) -> Result<(), String> {
    let db_guard = SYSTEM_DB.lock().map_err(|e| e.to_string())?;
    let db = db_guard.as_ref().ok_or("System DB not initialized")?;
    let conn = db.get_connection();

    conn.execute("DELETE FROM ad_patterns WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;

    Ok(())
}

pub fn check_ad_pattern_internal(title: &str, summary: &str, source_url: &str) -> AdCheckResult {
    if ad_detector::url_in_whitelist(source_url) {
        return AdCheckResult {
            is_blocked: false,
            matched_patterns: vec![],
        };
    }

    let db_guard = match SYSTEM_DB.lock() {
        Ok(guard) => guard,
        Err(_) => return AdCheckResult { is_blocked: false, matched_patterns: vec![] },
    };
    let db = match db_guard.as_ref() {
        Some(db) => db,
        None => return AdCheckResult { is_blocked: false, matched_patterns: vec![] },
    };
    let conn = db.get_connection();

    let mut stmt = match conn.prepare("SELECT id, pattern_type, pattern_value, confidence FROM ad_patterns WHERE enabled = 1 AND confidence >= 50") {
        Ok(stmt) => stmt,
        Err(_) => return AdCheckResult { is_blocked: false, matched_patterns: vec![] },
    };

    let text = format!("{} {}", title, summary).to_lowercase();
    let mut matched = Vec::new();

    let rows: Vec<(i64, String, String, i32)> = match stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))) {
        Ok(rows) => rows.filter_map(|r| r.ok()).collect(),
        Err(_) => return AdCheckResult { is_blocked: false, matched_patterns: vec![] },
    };

    for (id, ptype, pvalue, conf) in rows {
        let matches = match ptype.as_str() {
            "keyword" => text.contains(&pvalue.to_lowercase()),
            "domain" => source_url.contains(&pvalue),
            "regex" => regex::Regex::new(&pvalue)
                .map(|r| r.is_match(&text))
                .unwrap_or(false),
            _ => false,
        };

        if matches {
            matched.push(MatchedPattern {
                id,
                pattern_type: ptype,
                value: pvalue,
                confidence: conf,
            });
        }
    }

    AdCheckResult {
        is_blocked: !matched.is_empty(),
        matched_patterns: matched,
    }
}

pub fn increment_pattern_hit(pattern_id: i64) {
    let db_guard = match SYSTEM_DB.lock() {
        Ok(guard) => guard,
        Err(_) => return,
    };
    let db = match db_guard.as_ref() {
        Some(db) => db,
        None => return,
    };
    let conn = db.get_connection();
    let _ = conn.execute(
        "UPDATE ad_patterns SET hit_count = hit_count + 1, consecutive_hits = consecutive_hits + 1, last_hit_at = datetime('now', 'localtime') WHERE id = ?1",
        params![pattern_id],
    );
}

#[command]
pub fn check_ad_pattern(
    title: String,
    summary: String,
    source_url: String,
) -> Result<AdCheckResult, String> {
    Ok(check_ad_pattern_internal(&title, &summary, &source_url))
}

#[command]
pub fn add_ad_whitelist(url: String, note: Option<String>) -> Result<(), String> {
    let db_guard = SYSTEM_DB.lock().map_err(|e| e.to_string())?;
    let db = db_guard.as_ref().ok_or("System DB not initialized")?;
    let conn = db.get_connection();

    let hash = compute_url_hash(&url);
    conn.execute(
        "INSERT OR IGNORE INTO ad_whitelist (url_hash, source_url, note) VALUES (?1, ?2, ?3)",
        params![hash, url, note],
    ).map_err(|e| e.to_string())?;

    Ok(())
}

#[command]
pub fn remove_ad_whitelist(url: String) -> Result<(), String> {
    let db_guard = SYSTEM_DB.lock().map_err(|e| e.to_string())?;
    let db = db_guard.as_ref().ok_or("System DB not initialized")?;
    let conn = db.get_connection();

    let hash = compute_url_hash(&url);
    conn.execute("DELETE FROM ad_whitelist WHERE url_hash = ?1", params![hash])
        .map_err(|e| e.to_string())?;

    Ok(())
}

fn row_to_whitelist(row: &rusqlite::Row) -> rusqlite::Result<AdWhitelist> {
    Ok(AdWhitelist {
        id: row.get(0)?,
        url_hash: row.get(1)?,
        source_url: row.get(2)?,
        note: row.get(3)?,
        created_at: row.get(4)?,
    })
}

#[command]
pub fn list_ad_whitelist() -> Result<Vec<AdWhitelist>, String> {
    let db_guard = SYSTEM_DB.lock().map_err(|e| e.to_string())?;
    let db = db_guard.as_ref().ok_or("System DB not initialized")?;
    let conn = db.get_connection();

    let mut stmt = conn
        .prepare("SELECT id, url_hash, source_url, note, created_at FROM ad_whitelist ORDER BY created_at DESC")
        .map_err(|e| e.to_string())?;

    let rows: Vec<AdWhitelist> = stmt
        .query_map([], row_to_whitelist)
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    Ok(rows)
}

pub fn init_ad_patterns_with_conn(db: &SystemDb) -> Result<(), String> {
    let conn = db.get_connection();
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS ad_patterns (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            pattern_type TEXT NOT NULL,
            pattern_value TEXT NOT NULL,
            confidence INTEGER DEFAULT 50,
            source TEXT DEFAULT 'manual',
            hit_count INTEGER DEFAULT 0,
            consecutive_hits INTEGER DEFAULT 0,
            last_hit_at TEXT,
            enabled INTEGER DEFAULT 1,
            created_at TEXT DEFAULT (datetime('now', 'localtime')),
            updated_at TEXT DEFAULT (datetime('now', 'localtime')),
            UNIQUE(pattern_type, pattern_value)
        );

        CREATE TABLE IF NOT EXISTS ad_whitelist (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            url_hash TEXT NOT NULL UNIQUE,
            source_url TEXT,
            note TEXT,
            added_by TEXT DEFAULT 'user',
            created_at TEXT DEFAULT (datetime('now', 'localtime'))
        );

        CREATE INDEX IF NOT EXISTS idx_ad_patterns_type ON ad_patterns(pattern_type);
        CREATE INDEX IF NOT EXISTS idx_ad_patterns_enabled ON ad_patterns(enabled);
        CREATE INDEX IF NOT EXISTS idx_ad_whitelist_hash ON ad_whitelist(url_hash);
        "#,
    ).map_err(|e| e.to_string())?;

    Ok(())
}
