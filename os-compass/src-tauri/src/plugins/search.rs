use crate::db::open_db_at_path;
use crate::feature_plugin::{
    DbMode, FeaturePlugin, FeaturePluginType, FeatureResult, PluginContext, PluginError,
};
use crate::settings::get_settings;
use crate::source_engine::{get_adapter, llm_parser::parse_content_with_llm, SourceType};
use async_trait::async_trait;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub struct SearchPlugin;

impl SearchPlugin {
    pub fn new() -> Self {
        SearchPlugin
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub query: String,
    pub results: Vec<ProjectMatch>,
    pub total: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectMatch {
    pub project_name: Option<String>,
    pub project_url: Option<String>,
    pub description: Option<String>,
    pub language: Option<String>,
    pub source: String,
    pub match_score: f64,
}

pub fn init_search_db(vault_dir: &std::path::Path) -> Result<rusqlite::Connection, String> {
    let db_path = vault_dir.join("plugin_search.db");
    let conn = open_db_at_path(&db_path)?;

    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS search_sources (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            source_type TEXT NOT NULL,
            url TEXT NOT NULL,
            platform TEXT,
            enabled INTEGER DEFAULT 1,
            created_at TEXT DEFAULT (datetime('now', 'localtime')),
            updated_at TEXT DEFAULT (datetime('now', 'localtime'))
        );

        CREATE TABLE IF NOT EXISTS search_cache (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            source_id INTEGER NOT NULL REFERENCES search_sources(id) ON DELETE CASCADE,
            url_hash TEXT NOT NULL UNIQUE,
            project_name TEXT,
            project_url TEXT,
            description TEXT,
            language TEXT,
            raw_content TEXT,
            keywords TEXT,
            fetched_at TEXT DEFAULT (datetime('now', 'localtime')),
            expires_at TEXT
        );

        CREATE TABLE IF NOT EXISTS search_history (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            query TEXT NOT NULL,
            result_count INTEGER DEFAULT 0,
            result_summary TEXT,
            created_at TEXT DEFAULT (datetime('now', 'localtime'))
        );

        CREATE INDEX IF NOT EXISTS idx_search_sources_enabled ON search_sources(enabled);
        CREATE INDEX IF NOT EXISTS idx_search_cache_keywords ON search_cache(keywords);
        CREATE INDEX IF NOT EXISTS idx_search_history_created ON search_history(created_at DESC);
        "#,
    )
    .map_err(|e| e.to_string())?;

    Ok(conn)
}

fn compute_url_hash(url: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(url.as_bytes());
    hex::encode(hasher.finalize())
}

pub async fn three_layer_search(
    vault_dir: &std::path::Path,
    query: &str,
) -> Result<SearchResult, String> {
    let search_conn = init_search_db(vault_dir)?;
    let settings = get_settings();

    let mut results: Vec<ProjectMatch> = Vec::new();

    let local_results = {
        let db_guard = crate::db::DATABASE.lock().unwrap();
        let db = db_guard.as_ref().ok_or("Database not initialized")?;
        let main_conn = db.get_connection();

        let search_pattern = format!("%{}%", query.to_lowercase());
        let mut stmt = main_conn
            .prepare("SELECT id, name, url, description, languages FROM projects WHERE lifecycle_status != 'DELETED' AND (name LIKE ? OR description LIKE ? OR languages LIKE ?)")
            .map_err(|e| e.to_string())?;

        let rows = stmt
            .query_map(params![&search_pattern, &search_pattern, &search_pattern], |row| {
                Ok(ProjectMatch {
                    project_name: row.get(1)?,
                    project_url: row.get(2)?,
                    description: row.get(3)?,
                    language: row.get(4)?,
                    source: "local".to_string(),
                    match_score: 1.0,
                })
            })
            .map_err(|e| e.to_string())?;

        let mut local = Vec::new();
        for row in rows {
            local.push(row.map_err(|e| e.to_string())?);
        }
        local
    };

    results.extend(local_results);

    if results.len() < 5 {
        let cache_pattern = format!("%{}%", query.to_lowercase());
        let mut stmt = search_conn
            .prepare("SELECT project_name, project_url, description, language FROM search_cache WHERE keywords LIKE ? LIMIT ?")
            .map_err(|e| e.to_string())?;

        let cache_rows = stmt
            .query_map(params![&cache_pattern, &(10 - results.len())], |row| {
                Ok(ProjectMatch {
                    project_name: row.get(0)?,
                    project_url: row.get(1)?,
                    description: row.get(2)?,
                    language: row.get(3)?,
                    source: "cache".to_string(),
                    match_score: 0.7,
                })
            })
            .map_err(|e| e.to_string())?;

        for row in cache_rows {
            results.push(row.map_err(|e| e.to_string())?);
        }
    }

    if results.len() < 5 {
        let proxy = if settings.proxy_host.is_empty() {
            None
        } else {
            Some(settings.proxy_host.as_str())
        };

        let adapter = get_adapter(SourceType::WebCrawl);
        let search_url = format!("https://api.github.com/search/repositories?q={}", urlencoding::encode(query));

        if let Ok(contents) = adapter.fetch(&search_url, proxy).await {
            for content in contents.iter().take(10 - results.len()) {
                let url_hash = compute_url_hash(&content.url);
                let parsed = parse_content_with_llm(content, &settings).unwrap_or_default();

                let project_name = parsed
                    .first()
                    .and_then(|p| p.project_name.clone())
                    .unwrap_or_else(|| content.title.clone());

                let project_url = parsed
                    .first()
                    .and_then(|p| p.project_url.clone())
                    .unwrap_or_else(|| content.url.clone());

                let description = parsed.first().and_then(|p| p.description.clone());

                results.push(ProjectMatch {
                    project_name: Some(project_name),
                    project_url: Some(project_url),
                    description,
                    language: None,
                    source: "web".to_string(),
                    match_score: 0.3,
                });
            }
        }
    }

    results.sort_by(|a, b| b.match_score.partial_cmp(&a.match_score).unwrap());

    let total = results.len() as i64;

    search_conn.execute(
        "INSERT INTO search_history (query, result_count) VALUES (?, ?)",
        params![query, total],
    ).ok();

    Ok(SearchResult {
        query: (*query).to_string(),
        results,
        total,
    })
}

#[async_trait]
impl FeaturePlugin for SearchPlugin {
    fn id(&self) -> &str {
        "search"
    }

    fn name(&self) -> &str {
        "意图搜索"
    }

    fn plugin_type(&self) -> FeaturePluginType {
        FeaturePluginType::Search
    }

    fn version(&self) -> &str {
        "1.0.0"
    }

    fn db_mode(&self) -> DbMode {
        DbMode::Vault
    }

    fn db_path_template(&self) -> Option<&str> {
        Some("${vault_dir}/plugin_${plugin_id}.db")
    }

    async fn init(&self, context: &PluginContext) -> Result<(), PluginError> {
        init_search_db(&context.vault_dir).map_err(|e| PluginError::InitFailed(e))?;
        Ok(())
    }

    async fn on_enable(&self, _context: &PluginContext) -> Result<(), PluginError> {
        Ok(())
    }

    async fn on_disable(&self, _context: &PluginContext) -> Result<(), PluginError> {
        Ok(())
    }

    async fn execute(&self, _context: &PluginContext) -> Result<FeatureResult, PluginError> {
        Ok(FeatureResult::success())
    }
}
