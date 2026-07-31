use crate::db::DATABASE;
use crate::plugins::search::ProjectMatch;
use directories::BaseDirs;
use log;
use reqwest::Client;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use tauri::command;

static EMBEDDING_CONFIG: Mutex<Option<EmbeddingConfig>> = Mutex::new(None);
static VSS_LOADED: Mutex<bool> = Mutex::new(false);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingConfig {
    pub embedding_enabled: bool,
    pub embedding_api_type: String,
    pub embedding_api_url: String,
    pub embedding_api_key: String,
    pub embedding_model: String,
    pub embedding_dimension: i32,
    pub vss_extension_path: String,
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
            vss_extension_path: String::new(),
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
    
    let client = Client::new();
    let request_body = serde_json::json!({
        "input": text,
        "model": model,
    });

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

    let embedding_blob: Vec<u8> = embeddings.iter()
        .flat_map(|v| v.to_le_bytes())
        .collect();

    conn.execute(
        "INSERT OR REPLACE INTO project_embeddings (project_id, embedding) VALUES (?, ?)",
        params![project_id, embedding_blob],
    ).map_err(|e| e.to_string())?;

    log::debug!("[embedding] Stored embedding for project {}", project_id);
    Ok(())
}

pub async fn semantic_search(query: &str, limit: usize) -> Result<Vec<(i64, f32)>, String> {
    if !is_enabled() {
        return Err("Embedding not configured or disabled".to_string());
    }

    if let Err(e) = ensure_vss_loaded() {
        log::warn!("[embedding] VSS not available, falling back: {}", e);
    }

    let query_embedding = generate_embedding(query).await?;
    
    let db_guard = DATABASE.lock().unwrap();
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();

    let embedding_blob: Vec<u8> = query_embedding.iter()
        .flat_map(|v| v.to_le_bytes())
        .collect();

    let mut stmt = conn.prepare(
        "SELECT project_id, vss_search_params(?, 5) as distance
         FROM project_embeddings
         ORDER BY distance
         LIMIT ?"
    ).map_err(|e| e.to_string())?;

    let results = stmt.query_map(params![embedding_blob, limit as i64], |row| {
        Ok((row.get(0)?, row.get(1)?))
    }).map_err(|e| e.to_string())?;

    let mut matches = Vec::new();
    for row in results {
        if let Ok((project_id, distance)) = row {
            matches.push((project_id, distance));
        }
    }

    Ok(matches)
}

fn load_embedding_settings() -> Result<EmbeddingConfig, String> {
    let base_dirs = directories::BaseDirs::new().ok_or("Cannot find base directories")?;
    let app_data = base_dirs.data_dir().join(".os-compass");
    let db_path = app_data.join("plugins.db");
    
    let conn = rusqlite::Connection::open(&db_path).map_err(|e| e.to_string())?;

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
                vss_extension_path: row.get(6)?,
            })
        },
    );

    result.or_else(|_| {
        conn.execute_batch(
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
        ).ok();
        Ok(EmbeddingConfig::default())
    })
}

pub fn load_vss_extension(conn: &rusqlite::Connection) -> Result<(), String> {
    let base_dirs = BaseDirs::new().ok_or("Cannot find base directories")?;
    let app_data = base_dirs.data_dir().join(".os-compass");
    
    let default_path = app_data.join("vss0.dll");
    let extension_path = std::env::var("VSS_EXTENSION_PATH")
        .unwrap_or_else(|_| default_path.to_string_lossy().to_string());

    if !std::path::Path::new(&extension_path).exists() {
        log::warn!("[embedding] VSS extension not found at: {}", extension_path);
        return Err(format!("VSS extension not found: {}", extension_path));
    }

    unsafe {
        conn.load_extension(&extension_path, None)
            .map_err(|e| format!("Failed to load VSS extension: {}", e))?;
    }

    log::info!("[embedding] VSS extension loaded from: {}", extension_path);
    Ok(())
}

pub fn ensure_vss_loaded() -> Result<(), String> {
    let mut loaded = VSS_LOADED.lock().unwrap();
    if *loaded {
        return Ok(());
    }
    drop(loaded);

    let db_guard = DATABASE.lock().unwrap();
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();
    let conn_ref: &rusqlite::Connection = &*conn;

    match load_vss_extension_internal(conn_ref) {
        Ok(()) => {
            let mut loaded = VSS_LOADED.lock().unwrap();
            *loaded = true;
            log::info!("[embedding] VSS extension ready");
            Ok(())
        }
        Err(e) => {
            log::warn!("[embedding] VSS extension not available: {}", e);
            Err(e)
        }
    }
}

fn load_vss_extension_internal(conn: &rusqlite::Connection) -> Result<(), String> {
    let base_dirs = BaseDirs::new().ok_or("Cannot find base directories")?;
    let app_data = base_dirs.data_dir().join(".os-compass");

    let default_path = app_data.join("vss0.dll");
    let extension_path = std::env::var("VSS_EXTENSION_PATH")
        .unwrap_or_else(|_| default_path.to_string_lossy().to_string());

    if !std::path::Path::new(&extension_path).exists() {
        return Err(format!("VSS extension not found: {}", extension_path));
    }

    unsafe {
        conn.load_extension(&extension_path, None)
            .map_err(|e| format!("Failed to load VSS extension: {}", e))?;
    }

    log::info!("[embedding] VSS extension loaded from: {}", extension_path);
    Ok(())
}

pub fn reset_vss_state() {
    if let Ok(mut loaded) = VSS_LOADED.lock() {
        *loaded = false;
        log::info!("[embedding] VSS state reset for database switch");
    }
}

#[command]
pub async fn generate_embedding_for_text(text: String) -> Result<Vec<f32>, String> {
    generate_embedding(&text).await
}

#[command]
pub async fn generate_project_embeddings(project_id: i64) -> Result<usize, String> {
    let (name, description) = {
        let db_guard = DATABASE.lock().unwrap();
        let db = db_guard.as_ref().ok_or("Database not initialized")?;
        let conn = db.get_connection();

        conn.query_row(
            "SELECT name, description FROM projects WHERE id = ?",
            params![project_id],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?)),
        ).map_err(|e| e.to_string())?
    };

    let text = format!("{}: {}", name, description.unwrap_or_default());
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
        let (name, description) = {
            let db_guard = DATABASE.lock().unwrap();
            let db = db_guard.as_ref().ok_or("Database not initialized")?;
            let conn = db.get_connection();

            conn.query_row(
                "SELECT name, description FROM projects WHERE id = ?",
                params![pid],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?)),
            ).unwrap_or_else(|_| ("".to_string(), None))
        };

        if name.is_empty() {
            log::warn!("[embedding] Project {} has empty name, skipping", pid);
            continue;
        }

        let text = format!("{}: {}", name, description.unwrap_or_default());
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
                language,
                health_score: None,
                source: "local".to_string(),
                match_score: ((1.0f32 - distance).max(0.0f32) as f64),
            });
        }
    }

    Ok(results)
}
