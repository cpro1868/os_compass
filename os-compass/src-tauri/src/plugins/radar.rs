use crate::db::open_db_at_path;
use crate::content_filter::{filter_radar_item, FilterLevel, load_lexicon_from_file, scan_content};
use crate::feature_plugin::{
    DbMode, FeaturePlugin, FeaturePluginType, FeatureResult, PluginContext, PluginError,
};
use crate::plugin_config_db::PLUGIN_CONFIG_DB;
use crate::settings::get_settings;
use crate::source_engine::{get_adapter, SourceType};
use crate::commands::ad_patterns_cmd::{check_ad_pattern_internal, increment_pattern_hit};
use async_trait::async_trait;
use log;
use rusqlite::params;
use sha2::{Digest, Sha256};
use std::sync::Mutex;
use std::time::Duration;
use std::path::PathBuf;

pub struct RadarPlugin {
    _scan_flag: Mutex<bool>,
}

fn get_log_path() -> PathBuf {
    if let Some(base_dirs) = directories::BaseDirs::new() {
        base_dirs.data_dir().join(".os-compass").join("radar_filter.log")
    } else {
        PathBuf::from("radar_filter.log")
    }
}

fn write_filter_log(message: &str) {
    let log_path = get_log_path();
    use std::time::{SystemTime, UNIX_EPOCH};
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let secs = duration.as_secs();
    let hours = (secs / 3600) % 24 + 8;
    let mins = (secs / 60) % 60;
    let secs = secs % 60;
    let timestamp = format!("{:02}:{:02}:{:02}", hours, mins, secs);
    let log_line = format!("[{}] {}", timestamp, message);
    
    let _ = std::fs::create_dir_all(log_path.parent().unwrap());
    let _ = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .and_then(|mut file| {
            use std::io::Write;
            writeln!(file, "{}", log_line)
        });
    
    log::debug!("{}", message);
}

impl RadarPlugin {
    pub fn new() -> Self {
        RadarPlugin {
            _scan_flag: Mutex::new(false),
        }
    }
}

fn get_lexicon_path() -> PathBuf {
    if let Some(base_dirs) = directories::BaseDirs::new() {
        base_dirs.data_dir().join(".os-compass").join("spam_lexicon.json")
    } else {
        PathBuf::from("spam_lexicon.json")
    }
}

pub fn init_spam_lexicon() {
    let lexicon_path = get_lexicon_path();
    write_filter_log(&format!("[INIT] Loading lexicon from: {:?}", lexicon_path));
    if lexicon_path.exists() {
        load_lexicon_from_file(&lexicon_path);
        write_filter_log(&format!("[INIT] Lexicon loaded successfully"));
    } else {
        write_filter_log(&format!("[INIT] Lexicon file not found at {:?}, using default", lexicon_path));
    }
}

pub fn init_radar_db() -> Result<rusqlite::Connection, String> {
    // 雷达数据放在系统目录（使用 BaseDirs 确保和前端显示一致）
    let os_compass_dir = if let Some(base_dirs) = directories::BaseDirs::new() {
        base_dirs.data_dir().join(".os-compass")
    } else {
        std::path::PathBuf::from(".")
    };
    std::fs::create_dir_all(&os_compass_dir).ok();
    let db_path = os_compass_dir.join("plugin_radar.db");
    log::debug!("[radar] init_radar_db: db_path={:?}", db_path);

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

fn get_filter_threshold() -> f64 {
    let config_str = PLUGIN_CONFIG_DB.get_config("radar");
    let config: serde_json::Value = config_str
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or(serde_json::json!({}));
    
    let level = config.get("adFilterLevel")
        .and_then(|v| v.as_str())
        .unwrap_or("medium");
    
    match level {
        "off" => f64::MAX,
        "low" => 3.0,
        "high" => 1.0,
        _ => 2.0,
    }
}

pub async fn radar_scan_source(
    conn: &rusqlite::Connection,
    source_id: i64,
    source_type: &str,
    url: &str,
    time_range: Option<&str>,
) -> Result<(i64, i64, i64), String> {
    let mut scanned = 0i64;
    let mut new_items = 0i64;
    let mut errors = 0i64;

    log::info!("[radar] Scanning source_id={}, type={}, url={}, time_range={:?}", source_id, source_type, url, time_range);

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
            log::info!("[radar] Using source proxy: {}", full_proxy);
        } else {
            log::info!("[radar] Source proxy not enabled or empty, checking global proxy");
            let settings = get_settings();
            if !settings.proxy_host.is_empty() {
                proxy_url = Some(settings.proxy_host.clone());
                log::info!("[radar] Using global proxy: {}", proxy_url.as_ref().unwrap());
            }
        }
    } else {
        log::info!("[radar] Source config not found, trying global proxy");
        let settings = get_settings();
        if !settings.proxy_host.is_empty() {
            proxy_url = Some(settings.proxy_host.clone());
            log::info!("[radar] Using global proxy: {}", proxy_url.as_ref().unwrap());
        }
    }

    log::debug!("[radar] Fetching from url: {}", url);
    let contents = match adapter.fetch(url, proxy_url.as_deref(), time_range).await {
        Ok(c) => {
            log::debug!("[radar] Fetched {} items", c.len());
            c
        }
        Err(e) => {
            log::error!("[radar] Fetch error: {}", e);
            PLUGIN_CONFIG_DB.update_radar_source_status(source_id, "error", Some(&e.to_string())).ok();
            errors += 1;
            return Ok((scanned, new_items, errors));
        }
    };

    for content in &contents {
        scanned += 1;
        log::debug!("[radar] Processing item {}: {}", scanned, content.title);

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
            log::debug!("[radar] Item already exists, skipping");
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
            write_filter_log(&format!("[SKIP] Item blacklisted: {}", content.title));
            continue;
        }

        let filter_threshold = get_filter_threshold();
        write_filter_log(&format!("[CHECK] Item: {}, threshold: {}", content.title, filter_threshold));
        if filter_threshold < f64::MAX {
            let filter_result = crate::content_filter::filter_radar_item(
                &content.title,
                content.content.as_deref().unwrap_or(""),
                filter_threshold,
            );
            write_filter_log(&format!("[FILTER] title={}, threshold={}, filtered={}", content.title, filter_threshold, filter_result));
            if filter_result {
                write_filter_log(&format!("[SKIP] Spam filtered: {}", content.title));
                continue;
            }
        } else {
            write_filter_log(&format!("[INFO] Filter is OFF"));
        }

        let item_url = if content.url.is_empty() { content.title.clone() } else { content.url.clone() };
        let ad_check = check_ad_pattern_internal(&content.title, content.content.as_deref().unwrap_or(""), &item_url);
        if ad_check.is_blocked {
            write_filter_log(&format!("[AD-FILTER] title={}, matched={} patterns", content.title, ad_check.matched_patterns.len()));
            for pattern in &ad_check.matched_patterns {
                write_filter_log(&format!("[AD-FILTER]   - {}: {} (confidence: {})", pattern.pattern_type, pattern.value, pattern.confidence));
                increment_pattern_hit(pattern.id);
            }
            continue;
        }

        let project_name = content.title.clone();
        let project_url = item_url;
        let description = content.content.clone();
        let language = None::<String>;

        write_filter_log(&format!("[INSERT] name={}", project_name));
        let result = conn.execute(
            "INSERT OR IGNORE INTO radar_items (source_id, url_hash, project_name, project_url, description, language, raw_content, source_urls, published_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
            params![source_id, content_hash, project_name, project_url, description, language, content.content, content.url.clone(), content.published_at],
        );

        if result.is_ok() && conn.changes() > 0 {
            write_filter_log(&format!("[OK] Inserted: {}", project_name));
            new_items += 1;
        } else {
            write_filter_log(&format!("[SKIP] Duplicate or failed: {}", project_name));
        }
    }

    PLUGIN_CONFIG_DB.update_radar_source_status(source_id, "success", None).ok();
    log::info!("[radar] Scan completed: scanned={}, new={}, errors={}", scanned, new_items, errors);

    Ok((scanned, new_items, errors))
}

pub async fn radar_scan_all(time_range: Option<&str>) -> Result<(i64, i64, i64), String> {
    let conn = init_radar_db()?;

    // 从系统库读取信息源
    let system_sources = PLUGIN_CONFIG_DB.list_radar_sources();
    log::debug!("[radar] Found {} sources in system DB", system_sources.len());

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

        log::info!("[radar] Scanning source {}/{}: {}", idx + 1, system_sources.len(), source.name);

        match radar_scan_source(&conn, source_id, &source_type, &url, time_range).await {
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

    log::info!("[radar] Scan completed: total scanned={}, new={}, errors={}", total_scanned, total_new, total_errors);
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

    async fn init(&self, _context: &PluginContext) -> Result<(), PluginError> {
        init_radar_db()
            .map_err(|e| PluginError::InitFailed(e))?;
        init_spam_lexicon();
        Ok(())
    }

    async fn on_enable(&self, _context: &PluginContext) -> Result<(), PluginError> {
        Ok(())
    }

    async fn on_disable(&self, _context: &PluginContext) -> Result<(), PluginError> {
        Ok(())
    }

    async fn execute(&self, _context: &PluginContext) -> Result<FeatureResult, PluginError> {
        let result = tauri::async_runtime::block_on(radar_scan_all(None));
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
