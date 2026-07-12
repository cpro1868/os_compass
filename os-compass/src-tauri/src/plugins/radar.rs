use crate::db::open_db_at_path;
use crate::feature_plugin::{
    DbMode, FeaturePlugin, FeaturePluginType, FeatureResult, PluginContext, PluginError,
};
use crate::settings::get_settings;
use crate::source_engine::{get_adapter, llm_parser::parse_content_with_llm, SourceType};
use async_trait::async_trait;
use rusqlite::params;
use sha2::{Digest, Sha256};
use std::sync::Mutex;
use tauri::Emitter;

pub struct RadarPlugin {
    _scan_flag: Mutex<bool>,
}

impl RadarPlugin {
    pub fn new() -> Self {
        RadarPlugin {
            _scan_flag: Mutex::new(false),
        }
    }
}

pub fn init_radar_db(vault_dir: &std::path::Path) -> Result<rusqlite::Connection, String> {
    let db_path = vault_dir.join("plugin_radar.db");
    let conn = open_db_at_path(&db_path)?;

    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS radar_sources (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            source_type TEXT NOT NULL,
            url TEXT NOT NULL,
            platform TEXT,
            enabled INTEGER DEFAULT 1,
            check_interval INTEGER DEFAULT 86400,
            last_checked_at TEXT,
            last_status TEXT,
            last_error TEXT,
            created_at TEXT DEFAULT (datetime('now', 'localtime')),
            updated_at TEXT DEFAULT (datetime('now', 'localtime'))
        );

        CREATE TABLE IF NOT EXISTS radar_items (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            source_id INTEGER NOT NULL REFERENCES radar_sources(id) ON DELETE CASCADE,
            url_hash TEXT NOT NULL UNIQUE,
            project_name TEXT,
            project_url TEXT,
            description TEXT,
            language TEXT,
            raw_content TEXT,
            source_urls TEXT,
            status TEXT DEFAULT 'unread',
            imported_project_id INTEGER,
            published_at TEXT,
            fetched_at TEXT DEFAULT (datetime('now', 'localtime')),
            expires_at TEXT
        );

        CREATE TABLE IF NOT EXISTS radar_blacklist (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            url_hash TEXT NOT NULL UNIQUE,
            project_url TEXT,
            reason TEXT,
            created_at TEXT DEFAULT (datetime('now', 'localtime'))
        );

        CREATE INDEX IF NOT EXISTS idx_radar_items_status ON radar_items(status);
        CREATE INDEX IF NOT EXISTS idx_radar_items_source ON radar_items(source_id);
        CREATE INDEX IF NOT EXISTS idx_radar_sources_enabled ON radar_sources(enabled);
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

pub async fn radar_scan_source(
    conn: &rusqlite::Connection,
    source_id: i64,
    source_type: &str,
    url: &str,
) -> Result<(i64, i64, i64), String> {
    let mut scanned = 0i64;
    let mut new_items = 0i64;
    let mut errors = 0i64;

    let st = SourceType::from_str(source_type).unwrap_or(SourceType::Rss);
    let adapter = get_adapter(st);

    let settings = get_settings();
    let proxy = if settings.proxy_host.is_empty() {
        None
    } else {
        Some(settings.proxy_host.as_str())
    };

    let contents = match adapter.fetch(url, proxy).await {
        Ok(c) => c,
        Err(e) => {
            conn.execute(
                "UPDATE radar_sources SET last_checked_at = datetime('now', 'localtime'), last_status = 'error', last_error = ? WHERE id = ?",
                params![e.to_string(), source_id],
            ).ok();
            errors += 1;
            return Ok((scanned, new_items, errors));
        }
    };

    for content in &contents {
        scanned += 1;
        let url_hash = compute_url_hash(&content.url);

        let exists: bool = conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM radar_items WHERE url_hash = ?)",
                params![url_hash],
                |row| row.get(0),
            )
            .unwrap_or(false);

        if exists {
            continue;
        }

        let blacklisted: bool = conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM radar_blacklist WHERE url_hash = ?)",
                params![url_hash],
                |row| row.get(0),
            )
            .unwrap_or(false);

        if blacklisted {
            continue;
        }

        let parsed = parse_content_with_llm(content, &settings).unwrap_or_default();

        let project_name = parsed.first()
            .and_then(|p| p.project_name.clone())
            .unwrap_or_else(|| content.title.clone());

        let project_url = parsed.first()
            .and_then(|p| p.project_url.clone())
            .unwrap_or_else(|| content.url.clone());

        let description = parsed.first().and_then(|p| p.description.clone());
        let language = parsed.first().and_then(|p| p.language.clone());

        let source_urls = contents.iter()
            .filter(|c| c.url.contains("github.com") || c.url.contains("gitee.com"))
            .map(|c| c.url.clone())
            .collect::<Vec<_>>()
            .join(",");

        let result = conn.execute(
            "INSERT OR IGNORE INTO radar_items (source_id, url_hash, project_name, project_url, description, language, raw_content, source_urls, published_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
            params![source_id, url_hash, project_name, project_url, description, language, content.content, source_urls, content.published_at],
        );

        if result.is_ok() && conn.changes() > 0 {
            new_items += 1;
        }
    }

    conn.execute(
        "UPDATE radar_sources SET last_checked_at = datetime('now', 'localtime'), last_status = 'success', last_error = NULL WHERE id = ?",
        params![source_id],
    ).ok();

    Ok((scanned, new_items, errors))
}

pub async fn radar_scan_all(vault_dir: &std::path::Path) -> Result<(i64, i64, i64), String> {
    let conn = init_radar_db(vault_dir)?;

    let mut stmt = conn.prepare("SELECT id, source_type, url FROM radar_sources WHERE enabled = 1")
        .map_err(|e| e.to_string())?;

    let sources: Vec<(i64, String, String)> = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    let mut total_scanned = 0i64;
    let mut total_new = 0i64;
    let mut total_errors = 0i64;

    for (source_id, source_type, url) in sources {
        match radar_scan_source(&conn, source_id, &source_type, &url).await {
            Ok((s, n, e)) => {
                total_scanned += s;
                total_new += n;
                total_errors += e;
            }
            Err(_) => {
                total_errors += 1;
            }
        }
    }

    Ok((total_scanned, total_new, total_errors))
}

#[async_trait]
impl FeaturePlugin for RadarPlugin {
    fn id(&self) -> &str {
        "radar"
    }

    fn name(&self) -> &str {
        "情报雷达"
    }

    fn plugin_type(&self) -> FeaturePluginType {
        FeaturePluginType::Radar
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
        init_radar_db(&context.vault_dir)
            .map_err(|e| PluginError::InitFailed(e))?;
        Ok(())
    }

    async fn on_enable(&self, _context: &PluginContext) -> Result<(), PluginError> {
        Ok(())
    }

    async fn on_disable(&self, _context: &PluginContext) -> Result<(), PluginError> {
        Ok(())
    }

    async fn execute(&self, context: &PluginContext) -> Result<FeatureResult, PluginError> {
        let vault_dir = context.vault_dir.clone();
        let result = tauri::async_runtime::block_on(radar_scan_all(&vault_dir));
        match result {
            Ok((scanned, new_items, errors)) => {
                Ok(FeatureResult::success_with_data(serde_json::json!({
                    "scanned": scanned,
                    "newItems": new_items,
                    "errors": errors
                })))
            }
            Err(e) => Err(PluginError::ExecutionFailed(e)),
        }
    }
}
