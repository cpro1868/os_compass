use crate::db::open_db_at_path;
use crate::feature_plugin::{
    DbMode, FeaturePlugin, FeaturePluginType, FeatureResult, PluginContext, PluginError,
};
use crate::plugin_config_db::PLUGIN_CONFIG_DB;
use crate::settings::get_settings;
use crate::source_engine::{get_adapter, SourceType};
use async_trait::async_trait;
use rusqlite::params;
use sha2::{Digest, Sha256};
use std::sync::Mutex;
use std::time::Duration;

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
    println!("[radar] init_radar_db: vault_dir={:?}, db_path={:?}", vault_dir, db_path);

    let conn = open_db_at_path(&db_path)?;

    // vault 数据库只存储采集的数据（radar_items），信息源在系统库
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS radar_items (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            source_id INTEGER NOT NULL,
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

    println!("[radar] Scanning source_id={}, type={}, url={}", source_id, source_type, url);

    let st = SourceType::from_str(source_type).unwrap_or(SourceType::Rss);
    let mut adapter = get_adapter(st);

    let sources = PLUGIN_CONFIG_DB.list_radar_sources();
    let source_config = sources.iter().find(|s| s.id == source_id);

    let mut proxy_url: Option<String> = None;

    if let Some(src) = source_config {
        if src.proxy_enabled && !src.proxy_host.is_empty() {
            let protocol = if src.proxy_protocol.is_empty() { "socks5" } else { &src.proxy_protocol };
            let full_proxy = if !src.proxy_username.is_empty() {
                format!("{}://{}:{}@{}:{}", protocol, src.proxy_username, src.proxy_password, src.proxy_host, src.proxy_port)
            } else {
                format!("{}://{}:{}", protocol, src.proxy_host, src.proxy_port)
            };
            proxy_url = Some(full_proxy.clone());
            println!("[radar] Using source proxy: {}", full_proxy);
        } else {
            println!("[radar] Source proxy not enabled or empty, checking global proxy");
            let settings = get_settings();
            if !settings.proxy_host.is_empty() {
                proxy_url = Some(settings.proxy_host.clone());
                println!("[radar] Using global proxy: {}", proxy_url.as_ref().unwrap());
            }
        }
    } else {
        println!("[radar] Source config not found, trying global proxy");
        let settings = get_settings();
        if !settings.proxy_host.is_empty() {
            proxy_url = Some(settings.proxy_host.clone());
            println!("[radar] Using global proxy: {}", proxy_url.as_ref().unwrap());
        }
    }

    println!("[radar] Fetching from url: {}", url);
    let contents = match adapter.fetch(url, proxy_url.as_deref()).await {
        Ok(c) => {
            println!("[radar] Fetched {} items", c.len());
            c
        }
        Err(e) => {
            println!("[radar] Fetch error: {}", e);
            PLUGIN_CONFIG_DB.update_radar_source_status(source_id, "error", Some(&e.to_string())).ok();
            errors += 1;
            return Ok((scanned, new_items, errors));
        }
    };

    for content in &contents {
        scanned += 1;
        println!("[radar] Processing item {}: {}", scanned, content.title);

        // 使用 content 的 hash 作为唯一标识，而不是 url（因为 url 可能为空或重复）
        let content_hash = compute_url_hash(&format!("{}|{}", content.title, content.content.as_ref().unwrap_or(&String::new())));

        let exists: bool = conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM radar_items WHERE url_hash = ?)",
                params![content_hash],
                |row| row.get(0),
            )
            .unwrap_or(false);

        if exists {
            println!("[radar] Item already exists, skipping");
            continue;
        }

        let blacklisted: bool = conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM radar_blacklist WHERE url_hash = ?)",
                params![content_hash],
                |row| row.get(0),
            )
            .unwrap_or(false);

        if blacklisted {
            println!("[radar] Item is blacklisted, skipping");
            continue;
        }

        let project_name = content.title.clone();
        let project_url = if content.url.is_empty() { content.title.clone() } else { content.url.clone() };
        let description = content.content.clone();
        let language = None::<String>;

        println!("[radar] Inserting item: name={}, url={}", project_name, project_url);
        let result = conn.execute(
            "INSERT OR IGNORE INTO radar_items (source_id, url_hash, project_name, project_url, description, language, raw_content, source_urls, published_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
            params![source_id, content_hash, project_name, project_url, description, language, content.content, content.url.clone(), content.published_at],
        );

        if result.is_ok() && conn.changes() > 0 {
            println!("[radar] Successfully inserted new item");
            new_items += 1;
        } else {
            println!("[radar] Insert failed or no changes: {:?}", result.err());
        }
    }

    PLUGIN_CONFIG_DB.update_radar_source_status(source_id, "success", None).ok();
    println!("[radar] Scan completed: scanned={}, new={}, errors={}", scanned, new_items, errors);

    Ok((scanned, new_items, errors))
}

pub async fn radar_scan_all(vault_dir: &std::path::Path) -> Result<(i64, i64, i64), String> {
    let conn = init_radar_db(vault_dir)?;

    // 从系统库读取信息源
    let system_sources = PLUGIN_CONFIG_DB.list_radar_sources();
    println!("[radar] Found {} sources in system DB", system_sources.len());

    if system_sources.is_empty() {
        return Ok((0, 0, 0));
    }

    let mut total_scanned = 0i64;
    let mut total_new = 0i64;
    let mut total_errors = 0i64;

    // 顺序扫描，每个信息源之间延迟 1 秒
    for (idx, source) in system_sources.iter().enumerate() {
        if idx > 0 {
            tokio::time::sleep(Duration::from_secs(1)).await;
        }

        let source_id = source.id;
        let source_type = source.source_type.clone();
        let url = source.url.clone();

        println!("[radar] Scanning source {}/{}: {}", idx + 1, system_sources.len(), source.name);

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

    println!("[radar] Scan completed: total scanned={}, new={}, errors={}", total_scanned, total_new, total_errors);
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
