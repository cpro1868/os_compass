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
            ("project_embeddings", r#"
                CREATE TABLE IF NOT EXISTS project_embeddings (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    project_id INTEGER NOT NULL UNIQUE,
                    embedding BLOB NOT NULL,
                    dimension INTEGER NOT NULL DEFAULT 1536,
                    created_at TEXT DEFAULT (datetime('now', 'localtime'))
                );
                CREATE INDEX IF NOT EXISTS idx_embeddings_project ON project_embeddings(project_id);
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

CREATE TABLE IF NOT EXISTS project_embeddings (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    project_id INTEGER NOT NULL UNIQUE,
    embedding BLOB NOT NULL,
    dimension INTEGER NOT NULL DEFAULT 1536,
    created_at TEXT DEFAULT (datetime('now', 'localtime'))
);
CREATE INDEX IF NOT EXISTS idx_embeddings_project ON project_embeddings(project_id);

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

/// 加载预置分类清单（exe 同级 scripts/preset_categories.txt）
/// 找不到则返回 None，由调用方使用内嵌兜底清单
fn load_preset_categories_script() -> Option<String> {
    let exe = std::env::current_exe().ok()?;
    let exe_dir = exe.parent()?;
    let script_path = exe_dir.join("scripts").join("preset_categories.txt");
    if !script_path.exists() {
        return None;
    }
    std::fs::read_to_string(script_path).ok()
}

/// 内嵌兜底预置分类清单元数据（与 scripts/preset_categories.txt 内容一致）
/// 仅在外部脚本文件丢失时使用，保证功能可用
const PRESET_CATEGORIES_TXT: &str = r#"技术/前端开发/Web框架/React生态
技术/前端开发/Web框架/Vue生态
技术/前端开发/Web框架/Angular
技术/前端开发/Web框架/Svelte
技术/前端开发/UI组件库
技术/前端开发/小程序
技术/前端开发/桌面应用
技术/后端开发/Web框架
技术/后端开发/数据库
技术/后端开发/微服务
技术/后端开发/缓存与消息队列
技术/移动开发/Android
技术/移动开发/iOS
技术/移动开发/跨平台
技术/DevOps/容器与编排
技术/DevOps/CI与CD
技术/DevOps/监控与可观测性
技术/DevOps/基础设施即代码
技术/AI与机器学习/大模型与LLM
技术/AI与机器学习/机器学习框架
技术/AI与机器学习/计算机视觉
技术/AI与机器学习/自然语言处理
技术/AI与机器学习/数据工程
技术/数据工程/大数据框架
技术/数据工程/数据仓库
技术/数据工程/ETL与数据治理
学术/基础科学/数学
学术/基础科学/物理学
学术/基础科学/化学
学术/基础科学/生物学
学术/工程学科/土木工程
学术/工程学科/机械工程
学术/工程学科/电子电气
学术/医学与生命科学/药物研发
学术/医学与生命科学/基因技术
学术/医学与生命科学/医学影像
学术/社会科学/经济学
学术/社会科学/心理学
学术/社会科学/社会学
商业/企业管理/ERP与企业管理
商业/企业管理/CRM与客户管理
商业/企业管理/项目管理
商业/金融科技/支付系统
商业/金融科技/风险管理
商业/金融科技/区块链与加密货币
商业/市场营销/广告投放
商业/市场营销/增长与裂变
商业/创业/创业工具
商业/创业/商业智能
创意/内容创作/写作工具
创意/内容创作/博客与CMS
创意/内容创作/图文编辑
创意/音视频/视频剪辑
创意/音视频/特效动画
创意/音视频/流媒体
创意/设计/UI与UX设计
创意/设计/3D建模
创意/设计/平面设计
创意/游戏/游戏引擎
创意/游戏/游戏开发
创意/游戏/独立游戏"#;

/// 导入预置分类（幂等）
///
/// 逻辑：
/// 1. 读取预置分类清单（外部脚本优先，缺失则用内嵌兜底）
/// 2. 按 `/` 拆分为路径，逐级处理：父分类不存在则先创建（自增 id），再建子分类
/// 3. 按 name 判断已存在（同路径同层级下唯一），存在则跳过
/// 4. 完全不使用写死 id，不与用户已有分类冲突
///
/// 返回本次实际新增的分类数量。仅当用户在安装/初始化时选择"导入预置分类"才调用。
pub fn import_preset_categories() -> Result<i64, String> {
    let txt = load_preset_categories_script().unwrap_or_else(|| PRESET_CATEGORIES_TXT.to_string());

    let db_guard = DATABASE.lock().unwrap();
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();

    import_preset_categories_into(&conn, &txt)
}

/// 将预置分类清单导入到指定连接（幂等、逐级创建、动态 id）
pub fn import_preset_categories_into(conn: &rusqlite::Connection, txt: &str) -> Result<i64, String> {
    let mut created: i64 = 0;

    for line in txt.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with("--") || line.starts_with('#') {
            continue; // 跳过空行与注释
        }
        let segments: Vec<&str> = line.split('/').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
        if segments.is_empty() {
            continue;
        }

        let mut parent_id: Option<i64> = None;
        for seg in segments {
            let id: Option<i64> = if let Some(pid) = parent_id {
                conn.query_row(
                    "SELECT id FROM categories WHERE name = ?1 AND parent_id IS ?2",
                    rusqlite::params![seg, pid],
                    |row| row.get(0),
                ).ok()
            } else {
                conn.query_row(
                    "SELECT id FROM categories WHERE name = ?1 AND parent_id IS NULL",
                    rusqlite::params![seg],
                    |row| row.get(0),
                ).ok()
            };

            match id {
                Some(cid) => {
                    parent_id = Some(cid);
                }
                None => {
                    conn.execute(
                        "INSERT INTO categories (name, parent_id, sort_order) VALUES (?1, ?2, 0)",
                        rusqlite::params![seg, parent_id],
                    ).map_err(|e| format!("Failed to insert category '{}': {}", seg, e))?;
                    let cid = conn.last_insert_rowid();
                    parent_id = Some(cid);
                    created += 1;
                }
            }
        }
    }

    Ok(created)
}
