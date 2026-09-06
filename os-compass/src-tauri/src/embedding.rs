use crate::db::DATABASE;
use crate::plugins::search::ProjectMatch;
use log;
use reqwest::Client;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use tauri::command;

static EMBEDDING_CONFIG: Mutex<Option<EmbeddingConfig>> = Mutex::new(None);
static VEC_LOADED: Mutex<bool> = Mutex::new(false);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingConfig {
    pub embedding_enabled: bool,
    pub embedding_api_type: String,
    pub embedding_api_url: String,
    pub embedding_api_key: String,
    pub embedding_model: String,
    pub embedding_dimension: i32,
    pub vec_extension_path: String,
}

impl Default for EmbeddingConfig {
    fn default() -> Self {
        Self {
            embedding_enabled: true,
            embedding_api_type: "openai".to_string(),
            embedding_api_url: "https://api.openai.com/v1".to_string(),
            embedding_api_key: String::new(),
            embedding_model: "text-embedding-3-small".to_string(),
            embedding_dimension: 1536,
            vec_extension_path: String::new(),
        }
    }
}

pub fn init_module() -> Result<(), String> {
    let config = load_embedding_settings()?;
    let mut guard = EMBEDDING_CONFIG.lock().unwrap();
    *guard = Some(config);
    log::info!("[embedding] Module initialized");
    Ok(())
}

pub fn update_config(config: EmbeddingConfig) {
    let mut guard = EMBEDDING_CONFIG.lock().unwrap();
    *guard = Some(config);
    log::info!("[embedding] Config updated");
}

pub fn is_enabled() -> bool {
    if let Ok(guard) = EMBEDDING_CONFIG.lock() {
        if let Some(config) = guard.as_ref() {
            return config.embedding_enabled && !config.embedding_api_key.is_empty();
        }
    }
    false
}

pub async fn generate_embedding(text: &str) -> Result<Vec<f32>, String> {
    let (url, api_key, model) = {
        let guard = EMBEDDING_CONFIG.lock().unwrap();
        let config = guard.as_ref().ok_or_else(|| {
            log::error!("[embedding] Config not loaded");
            "Embedding config not loaded".to_string()
        })?;
        if !config.embedding_enabled {
            log::error!("[embedding] Embedding is disabled");
            return Err("Embedding is disabled".to_string());
        }
        if config.embedding_api_key.is_empty() {
            log::error!("[embedding] API key is empty");
            return Err("Embedding API key is not configured".to_string());
        }
        log::info!("[embedding] Using API: {} with model: {}", config.embedding_api_url, config.embedding_model);
        let base_url = config.embedding_api_url.trim_end_matches('/');
        let url = if base_url.ends_with("/v1") {
            format!("{}/embeddings", base_url)
        } else {
            format!("{}/v1/embeddings", base_url)
        };
        (
            url,
            config.embedding_api_key.clone(),
            config.embedding_model.clone(),
        )
    };
    
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| format!("Client build failed: {}", e))?;
        
    log::info!("[embedding] HTTP client built with 30s timeout");
    
    let request_body = serde_json::json!({
        "input": text,
        "model": model,
    });

    log::info!("[embedding] Sending embedding request...");
    let response = client.post(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&request_body)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!("API error {}: {}", status, body));
    }

    #[derive(Deserialize)]
    struct ApiResponse {
        data: Vec<EmbeddingData>,
    }
    
    #[derive(Deserialize)]
    struct EmbeddingData {
        embedding: Vec<f32>,
    }

    let api_response: ApiResponse = response.json().await
        .map_err(|e| format!("Parse response failed: {}", e))?;

    api_response.data.first()
        .map(|d| d.embedding.clone())
        .ok_or_else(|| "No embedding returned".to_string())
}

pub fn store_embeddings(project_id: i64, embeddings: &[f32]) -> Result<(), String> {
    let db_guard = DATABASE.lock().unwrap();
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();

    ensure_vec_table_exists_internal(&conn)?;

    let embedding_json = serde_json::to_string(embeddings)
        .map_err(|e| format!("Serialize embedding failed: {}", e))?;

    conn.execute(
        "INSERT OR REPLACE INTO project_embeddings (project_id, embedding) VALUES (?, ?)",
        params![project_id, embedding_json],
    ).map_err(|e| e.to_string())?;

    log::debug!("[embedding] Stored embedding for project {}", project_id);
    Ok(())
}

fn ensure_vec_table_exists_internal(conn: &rusqlite::Connection) -> Result<(), String> {
    let table_exists: i32 = conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='project_embeddings'",
        [],
        |row| row.get(0),
    ).unwrap_or(0);

    if table_exists == 0 {
        log::info!("[embedding] Creating vec0 virtual table for embeddings...");
        conn.execute_batch(
            "CREATE VIRTUAL TABLE IF NOT EXISTS project_embeddings USING vec0(
                project_id INTEGER PRIMARY KEY,
                embedding FLOAT[1536]
            );"
        ).map_err(|e| {
            log::error!("[embedding] Failed to create vec0 table: {}", e);
            format!("Failed to create vec0 table: {}", e)
        })?;
        log::info!("[embedding] vec0 virtual table created successfully");
    }

    Ok(())
}

pub async fn semantic_search(query: &str, limit: usize) -> Result<Vec<(i64, f32)>, String> {
    log::info!("[semantic_search] 开始语义搜索，query: {}, limit: {}", query, limit);

    if !is_enabled() {
        log::error!("[semantic_search] Embedding 未启用");
        return Err("Embedding not configured or disabled".to_string());
    }
    log::info!("[semantic_search] Embedding 已启用");

    log::info!("[semantic_search] 开始生成查询向量...");
    let query_embedding = generate_embedding(query).await?;
    log::info!("[semantic_search] 查询向量生成成功，长度: {}", query_embedding.len());

    log::info!("[semantic_search] 开始数据库查询...");
    let db_guard = DATABASE.lock().unwrap();
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();

    let mut stmt = conn.prepare(
        "SELECT project_id, embedding FROM project_embeddings"
    ).map_err(|e| format!("准备查询 embedding 失败: {}", e))?;

    let rows = stmt.query_map([], |row| {
        Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
    }).map_err(|e| format!("查询 embedding 失败: {}", e))?;

    let mut scored: Vec<(i64, f32)> = Vec::new();
    let mut parse_failures = 0usize;
    for row in rows {
        let (project_id, emb_json) = match row {
            Ok(v) => v,
            Err(_) => continue,
        };
        match serde_json::from_str::<Vec<f32>>(&emb_json) {
            Ok(vec) => {
                if vec.is_empty() {
                    parse_failures += 1;
                    continue;
                }
                let score = cosine_similarity(&query_embedding, &vec);
                scored.push((project_id, score));
            }
            Err(_) => parse_failures += 1,
        }
    }

    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    scored.truncate(limit);

    log::info!(
        "[semantic_search] 完成：候选 {} 条，解析失败 {} 条，返回 top {}",
        scored.len(),
        parse_failures,
        limit.min(scored.len())
    );

    Ok(scored)
}

pub(crate) fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let n = a.len().min(b.len());
    if n == 0 {
        return 0.0;
    }
    let mut dot = 0.0f64;
    let mut na = 0.0f64;
    let mut nb = 0.0f64;
    for i in 0..n {
        let x = a[i] as f64;
        let y = b[i] as f64;
        dot += x * y;
        na += x * x;
        nb += y * y;
    }
    let denom = (na * nb).sqrt();
    if denom == 0.0 {
        return 0.0;
    }
    (dot / denom) as f32
}

/// 项目名称、描述、语言拼装成单一文本，用于生成 embedding 向量。
/// 必须包含 language 字段，否则 “地图类工具” 这类跨语言搜索会因为关键词缺失而无法定位到目标项目。
pub(crate) fn build_project_embedding_text(
    name: &str,
    description: Option<&str>,
    languages: Option<&str>,
) -> String {
    let mut parts: Vec<String> = Vec::new();
    let name = name.trim();
    if !name.is_empty() {
        parts.push(name.to_string());
    }
    if let Some(d) = description {
        let t = d.trim();
        if !t.is_empty() {
            parts.push(t.to_string());
        }
    }
    if let Some(langs) = languages {
        if let Some(primary) = crate::plugins::search::primary_language(Some(langs.to_string())) {
            parts.push(format!("language: {}", primary));
        }
    }
    parts.join("\n")
}

fn load_embedding_settings() -> Result<EmbeddingConfig, String> {
    let base_dirs = directories::BaseDirs::new().ok_or("Cannot find base directories")?;
    let app_data = base_dirs.data_dir().join(".os-compass");
    let db_path = app_data.join("plugins.db");

    let conn = rusqlite::Connection::open(&db_path).map_err(|e| e.to_string())?;
    load_embedding_settings_from_conn(&conn)
}

fn load_embedding_settings_from_conn(conn: &rusqlite::Connection) -> Result<EmbeddingConfig, String> {
    let result: Result<EmbeddingConfig, _> = conn.query_row(
        "SELECT embedding_enabled, embedding_api_type, embedding_api_url, embedding_api_key,
         embedding_model, embedding_dimension, vss_extension_path
         FROM embedding_settings WHERE id = 1",
        [],
        |row| {
            Ok(EmbeddingConfig {
                embedding_enabled: row.get::<_, i32>(0)? == 1,
                embedding_api_type: row.get(1)?,
                embedding_api_url: row.get(2)?,
                embedding_api_key: row.get(3)?,
                embedding_model: row.get(4)?,
                embedding_dimension: row.get(5)?,
                vec_extension_path: row.get(6)?,
            })
        },
    );

    if let Ok(cfg) = result {
        return Ok(cfg);
    }

    log::warn!("[embedding] embedding_settings 列名或行不匹配，尝试重建兼容表");
    let _ = conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS embedding_settings (
            id INTEGER PRIMARY KEY,
            embedding_enabled INTEGER DEFAULT 1,
                embedding_api_type TEXT DEFAULT 'openai',
                embedding_api_url TEXT DEFAULT '',
                embedding_api_key TEXT DEFAULT '',
                embedding_model TEXT DEFAULT 'text-embedding-3-small',
                embedding_dimension INTEGER DEFAULT 1536,
                vss_extension_path TEXT DEFAULT ''
            );
            INSERT OR IGNORE INTO embedding_settings (id) VALUES (1);"
    );
    Ok(EmbeddingConfig::default())
}

pub fn init_vec_extension() -> Result<(), String> {
    use sqlite_vec::sqlite3_vec_init;
    
    unsafe {
        rusqlite::ffi::sqlite3_auto_extension(
            Some(std::mem::transmute(sqlite3_vec_init as *const ()))
        );
    }
    
    log::info!("[embedding] sqlite-vec extension registered");
    Ok(())
}

pub fn preload_vec_once() -> Result<(), String> {
    let mut loaded = VEC_LOADED.lock().unwrap();
    if *loaded {
        log::info!("[embedding] sqlite-vec already loaded, skipping");
        return Ok(());
    }
    drop(loaded);

    match init_vec_extension() {
        Ok(()) => {
            let mut loaded = VEC_LOADED.lock().unwrap();
            *loaded = true;
            log::info!("[embedding] sqlite-vec extension ready");
            Ok(())
        }
        Err(e) => {
            log::warn!("[embedding] sqlite-vec extension failed: {}", e);
            Err(e)
        }
    }
}

pub fn ensure_vec_loaded() -> Result<(), String> {
    let loaded = VEC_LOADED.lock().unwrap();
    if *loaded {
        return Ok(());
    }
    drop(loaded);

    preload_vec_once()
}

pub fn reset_vec_state() {
    if let Ok(mut loaded) = VEC_LOADED.lock() {
        *loaded = false;
        log::info!("[embedding] sqlite-vec state reset for database switch");
    }
}

#[command]
pub async fn generate_embedding_for_text(text: String) -> Result<Vec<f32>, String> {
    generate_embedding(&text).await
}

#[command]
pub async fn generate_project_embeddings(project_id: i64) -> Result<usize, String> {
    let (name, description, languages) = {
        let db_guard = DATABASE.lock().unwrap();
        let db = db_guard.as_ref().ok_or("Database not initialized")?;
        let conn = db.get_connection();

        conn.query_row(
            "SELECT name, description, languages FROM projects WHERE id = ?",
            params![project_id],
            |row| Ok((
                row.get::<_, String>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, Option<String>>(2)?,
            )),
        ).map_err(|e| e.to_string())?
    };

    let text = build_project_embedding_text(&name, description.as_deref(), languages.as_deref());
    let embedding = generate_embedding(&text).await?;
    store_embeddings(project_id, &embedding)?;

    Ok(1)
}

#[command]
pub async fn rebuild_embeddings(project_id: Option<i64>) -> Result<i64, String> {
    let project_ids = {
        let db_guard = DATABASE.lock().unwrap();
        let db = db_guard.as_ref().ok_or("Database not initialized")?;
        let conn = db.get_connection();

        if let Some(pid) = project_id {
            vec![pid]
        } else {
            log::info!("[embedding] Starting rebuild for all projects");
            let mut stmt = conn.prepare(
                "SELECT id FROM projects WHERE lifecycle_status != 'DELETED'"
            ).map_err(|e| e.to_string())?;

            let rows = stmt.query_map([], |row| row.get(0))
                .map_err(|e| e.to_string())?;

            let ids: Vec<i64> = rows.filter_map(|r| r.ok()).collect();
            log::info!("[embedding] Found {} projects to vectorize", ids.len());
            ids
        }
    };

    let mut count = 0i64;
    for pid in project_ids {
        let (name, description, languages) = {
            let db_guard = DATABASE.lock().unwrap();
            let db = db_guard.as_ref().ok_or("Database not initialized")?;
            let conn = db.get_connection();

            conn.query_row(
                "SELECT name, description, languages FROM projects WHERE id = ?",
                params![pid],
                |row| Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, Option<String>>(2)?,
                )),
            ).unwrap_or_else(|_| ("".to_string(), None, None))
        };

        if name.is_empty() {
            log::warn!("[embedding] Project {} has empty name, skipping", pid);
            continue;
        }

        let text = build_project_embedding_text(
            &name,
            description.as_deref(),
            languages.as_deref(),
        );
        log::info!("[embedding] Generating embedding for project {}: {}", pid, name);

        match generate_embedding(&text).await {
            Ok(embedding) => {
                match store_embeddings(pid, &embedding) {
                    Ok(()) => {
                        count += 1;
                        log::info!("[embedding] Stored embedding for project {}", pid);
                    }
                    Err(e) => {
                        log::error!("[embedding] Failed to store embedding for project {}: {}", pid, e);
                    }
                }
            }
            Err(e) => {
                log::error!("[embedding] Failed to generate embedding for project {}: {}", pid, e);
            }
        }
    }

    log::info!("[embedding] Rebuilt {} embeddings", count);
    Ok(count)
}

#[command]
pub async fn semantic_search_projects(query: String, limit: Option<usize>) -> Result<Vec<ProjectMatch>, String> {
    let limit = limit.unwrap_or(10);
    
    let matches = semantic_search(&query, limit).await?;

    let db_guard = DATABASE.lock().unwrap();
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();

    let mut results = Vec::new();
    for (project_id, distance) in matches {
        if let Ok((name, url, description, language, stars, forks)) = conn.query_row(
            "SELECT name, url, description, languages, stars, forks FROM projects WHERE id = ?",
            params![project_id],
            |row| Ok((
                row.get::<_, String>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, Option<i64>>(4)?,
                row.get::<_, Option<i64>>(5)?,
            )),
        ) {
            results.push(ProjectMatch {
                name,
                url: url.unwrap_or_default(),
                description,
                stars,
                forks,
                language: crate::plugins::search::primary_language(language),
                health_score: None,
                source: "local".to_string(),
                match_score: ((1.0f32 - distance).max(0.0f32) as f64),
                project_id: Some(project_id),
            });
        }
    }

    Ok(results)
}
