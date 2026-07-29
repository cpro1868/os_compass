use rusqlite::{Connection, Result};
use std::path::PathBuf;
use std::sync::Mutex;
use log;

pub type MutexGuard<'a, T> = std::sync::MutexGuard<'a, T>;

pub struct Database {
    conn: Mutex<Connection>,
}

impl Database {
    pub fn new(path: PathBuf) -> Result<Self> {
        let conn = Connection::open(path)?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    pub fn init_schema(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        // 优先从外部脚本文件加载（exe 同级 scripts/init_schema.sql）
        if let Some(sql) = load_external_init_script() {
            log::debug!("[db] Initializing schema from external script");
            conn.execute_batch(&sql).map_err(|e| {
                log::warn!("[db] External script failed: {}, falling back to embedded", e);
                e
            })?;
        } else {
            // 兜底：使用内嵌的脚本（防止脚本文件丢失导致无法启动）
            log::debug!("[db] Initializing schema from embedded SQL");
            conn.execute_batch(INIT_SCHEMA_SQL)?;
        }

        // 检查并添加缺失的表（兼容已有数据库）
        log::debug!("[db] Checking for missing tables...");
        let missing_tables = [
            ("project_user_info", r#"
                CREATE TABLE IF NOT EXISTS project_user_info (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    project_id INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
                    info_key VARCHAR(100) NOT NULL,
                    info_value TEXT,
                    is_secret BOOLEAN DEFAULT FALSE,
                    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                    UNIQUE(project_id, info_key)
                );
                CREATE INDEX IF NOT EXISTS idx_user_info_project ON project_user_info(project_id);
            "#),
        ];

        for (table_name, create_sql) in &missing_tables {
            let exists: i32 = conn.query_row(
                &format!("SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='{}'", table_name),
                [],
                |row| row.get(0),
            ).unwrap_or(1);
            
            if exists == 0 {
                log::debug!("[db] Creating missing table: {}", table_name);
                conn.execute_batch(create_sql).ok();
            }
        }

        Ok(())
    }

    pub fn get_connection(&self) -> std::sync::MutexGuard<'_, Connection> {
        self.conn.lock().unwrap()
    }
}

/// 从 exe 同级 scripts/init_schema.sql 加载初始化脚本
/// 找不到则返回 None，由调用方使用内嵌兜底脚本
fn load_external_init_script() -> Option<String> {
    let exe = std::env::current_exe().ok()?;
    let exe_dir = exe.parent()?;
    let script_path = exe_dir.join("scripts").join("init_schema.sql");
    if !script_path.exists() {
        return None;
    }
    std::fs::read_to_string(script_path).ok()
}

/// 内嵌兜底脚本（与 scripts/init_schema.sql 内容一致）
/// 仅在外部脚本文件丢失时使用，保证程序可启动
const INIT_SCHEMA_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS categories (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    parent_id INTEGER REFERENCES categories(id),
    sort_order INTEGER DEFAULT 0,
    created_at TEXT DEFAULT (datetime('now', 'localtime')),
    updated_at TEXT DEFAULT (datetime('now', 'localtime'))
);

CREATE TABLE IF NOT EXISTS tags (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,
    color TEXT DEFAULT '#6366f1',
    source TEXT DEFAULT 'user',
    created_at TEXT DEFAULT (datetime('now', 'localtime'))
);

CREATE TABLE IF NOT EXISTS projects (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    url TEXT,
    source TEXT NOT NULL,
    source_id TEXT,
    description TEXT,
    languages TEXT,
    stars INTEGER DEFAULT 0,
    forks INTEGER DEFAULT 0,
    open_issues INTEGER DEFAULT 0,
    license TEXT,
    homepage TEXT,
    latest_commit TEXT,
    latest_release TEXT,
    readme_content TEXT,
    readme_lang TEXT DEFAULT 'en',
    health_score REAL,
    ai_summary TEXT,
    ai_use_cases TEXT,
    ai_risks TEXT,
    ai_dependencies TEXT,
    translated_summary TEXT,
    translated_use_cases TEXT,
    translated_risks TEXT,
    translated_dependencies TEXT,
    translated_description TEXT,
    readme_translation TEXT,
    category_id INTEGER REFERENCES categories(id),
    lifecycle_status TEXT DEFAULT 'TO_EXPLORE',
    data_status TEXT DEFAULT 'ACTIVE',
    is_downloaded INTEGER DEFAULT 0,
    local_path TEXT,
    runbook TEXT,
    archived_at TEXT,
    deleted_at TEXT,
    created_at TEXT DEFAULT (datetime('now', 'localtime')),
    updated_at TEXT DEFAULT (datetime('now', 'localtime'))
);

CREATE TABLE IF NOT EXISTS project_tags (
    project_id INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    tag_id INTEGER NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    PRIMARY KEY (project_id, tag_id)
);

CREATE TABLE IF NOT EXISTS project_notes (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    project_id INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    content TEXT NOT NULL,
    created_at TEXT DEFAULT (datetime('now', 'localtime')),
    updated_at TEXT DEFAULT (datetime('now', 'localtime'))
);

-- NOTE: system_variables and app_settings moved to system_settings.db (V2.0)

CREATE TABLE IF NOT EXISTS readme_variants (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    project_id INTEGER NOT NULL,
    file_name TEXT NOT NULL,
    language TEXT NOT NULL,
    content TEXT NOT NULL,
    created_at TEXT DEFAULT (datetime('now', 'localtime')),
    UNIQUE(project_id, file_name)
);

CREATE TABLE IF NOT EXISTS translations (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    project_id INTEGER NOT NULL,
    field_name TEXT NOT NULL,
    language TEXT NOT NULL,
    content TEXT NOT NULL,
    created_at TEXT DEFAULT (datetime('now', 'localtime')),
    updated_at TEXT DEFAULT (datetime('now', 'localtime')),
    UNIQUE(project_id, field_name, language)
);

CREATE TABLE IF NOT EXISTS project_clone (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    project_id INTEGER NOT NULL UNIQUE,
    cloned_path TEXT,
    cloned_at TEXT,
    proxy_enabled INTEGER DEFAULT 0,
    proxy_protocol TEXT DEFAULT 'http',
    proxy_host TEXT,
    proxy_port INTEGER DEFAULT 0,
    proxy_username TEXT,
    proxy_password TEXT,
    created_at TEXT DEFAULT (datetime('now', 'localtime')),
    updated_at TEXT DEFAULT (datetime('now', 'localtime'))
);

CREATE TABLE IF NOT EXISTS project_releases (
    id TEXT PRIMARY KEY,
    project_id INTEGER NOT NULL,
    tag_name TEXT NOT NULL,
    published_at TEXT,
    body TEXT,
    body_zh TEXT,
    download_urls TEXT,
    source TEXT,
    fetched_at TEXT,
    created_at TEXT DEFAULT (datetime('now', 'localtime')),
    FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_projects_category ON projects(category_id);
CREATE INDEX IF NOT EXISTS idx_projects_status ON projects(lifecycle_status, data_status);
CREATE INDEX IF NOT EXISTS idx_project_notes_project ON project_notes(project_id);
CREATE INDEX IF NOT EXISTS idx_variables_secret ON system_variables(is_secret);
CREATE INDEX IF NOT EXISTS idx_plugins_enabled ON source_plugins(enabled);
CREATE INDEX IF NOT EXISTS idx_releases_project ON project_releases(project_id);
CREATE INDEX IF NOT EXISTS idx_releases_published ON project_releases(published_at DESC);

INSERT OR IGNORE INTO categories (id, name, sort_order) VALUES (1, '未分类', 0);

-- NOTE: source_plugins moved to system_settings.db (V2.0)
-- NOTE: system_variables moved to system_settings.db (V2.0)

CREATE TABLE IF NOT EXISTS feature_plugins (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    plugin_type TEXT NOT NULL,
    enabled INTEGER DEFAULT 0,
    config TEXT,
    version TEXT,
    db_mode TEXT DEFAULT 'none',
    db_path_template TEXT,
    created_at TEXT DEFAULT (datetime('now', 'localtime')),
    updated_at TEXT DEFAULT (datetime('now', 'localtime'))
);

CREATE INDEX IF NOT EXISTS idx_feature_plugins_enabled ON feature_plugins(enabled);

INSERT OR IGNORE INTO feature_plugins (id, name, plugin_type, enabled, version, db_mode, db_path_template) VALUES
('radar', '情报雷达', 'radar', 0, '1.0.0', 'vault', '${vault_dir}/plugin_${plugin_id}.db'),
('search', '意图搜索', 'search', 0, '1.0.0', 'vault', '${vault_dir}/plugin_${plugin_id}.db');

-- 向量搜索表（sqlite-vss）
CREATE TABLE IF NOT EXISTS project_embeddings (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    project_id INTEGER NOT NULL UNIQUE,
    embedding BLOB NOT NULL,
    dimension INTEGER NOT NULL DEFAULT 1536,
    created_at TEXT DEFAULT (datetime('now', 'localtime'))
);
CREATE INDEX IF NOT EXISTS idx_embeddings_project ON project_embeddings(project_id);
"#;

pub fn switch_database(path: PathBuf) -> Result<(), String> {
    let db_path_for_log = path.clone();
    let new_db = Database::new(path).map_err(|e| e.to_string())?;
    new_db.init_schema().map_err(|e| e.to_string())?;
    let mut global = DATABASE.lock().unwrap();
    *global = Some(new_db);
    drop(global);

    #[cfg(feature = "embedding")]
    {
        crate::embedding::reset_vss_state();
    }

    log::info!("[db] Database switched to: {:?}", db_path_for_log);
    Ok(())
}

pub fn get_database() -> MutexGuard<'static, Option<Database>> {
    DATABASE.lock().unwrap()
}

lazy_static::lazy_static! {
    pub static ref DATABASE: Mutex<Option<Database>> = Mutex::new(None);
}

pub fn init_database(app_dir: PathBuf) -> Result<()> {
    let db_path = app_dir.join("os_compass.db");
    let db = Database::new(db_path)?;
    db.init_schema()?;
    let mut global = DATABASE.lock().unwrap();
    *global = Some(db);
    Ok(())
}

pub fn open_db_at_path(path: &std::path::Path) -> Result<Connection, String> {
    if let Some(parent) = path.parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
    }
    let conn = Connection::open(path).map_err(|e| e.to_string())?;
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;").map_err(|e| e.to_string())?;
    Ok(conn)
}
