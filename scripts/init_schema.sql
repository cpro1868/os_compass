-- OS-Compass Database Init Schema
-- Tables, indexes, and seed data

-- 1. Business tables
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

-- NOTE: system_variables, app_settings, source_plugins moved to system_settings.db (V2.0)

-- 5. README variants
CREATE TABLE IF NOT EXISTS readme_variants (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    project_id INTEGER NOT NULL,
    file_name TEXT NOT NULL,
    language TEXT NOT NULL,
    content TEXT NOT NULL,
    created_at TEXT DEFAULT (datetime('now', 'localtime')),
    UNIQUE(project_id, file_name)
);

-- 6. Translations
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

-- 7. Project clone
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

-- 8. Project releases
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

-- 8.5 Project user info (数据保险箱)
CREATE TABLE IF NOT EXISTS project_user_info (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    project_id INTEGER NOT NULL,
    info_key VARCHAR(100) NOT NULL,
    info_value TEXT,
    is_secret BOOLEAN DEFAULT FALSE,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(project_id, info_key),
    FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE
);

-- 9. Indexes
CREATE INDEX IF NOT EXISTS idx_projects_category ON projects(category_id);
CREATE INDEX IF NOT EXISTS idx_projects_status ON projects(lifecycle_status, data_status);
CREATE INDEX IF NOT EXISTS idx_project_notes_project ON project_notes(project_id);
CREATE INDEX IF NOT EXISTS idx_variables_secret ON system_variables(is_secret);
CREATE INDEX IF NOT EXISTS idx_plugins_enabled ON source_plugins(enabled);
CREATE INDEX IF NOT EXISTS idx_releases_project ON project_releases(project_id);
CREATE INDEX IF NOT EXISTS idx_releases_published ON project_releases(published_at DESC);
CREATE INDEX IF NOT EXISTS idx_user_info_project ON project_user_info(project_id);

-- 10. Seed data
INSERT OR IGNORE INTO categories (id, name, sort_order) VALUES (1, '未分类', 0);

-- NOTE: source_plugins and system_variables moved to system_settings.db (V2.0)

-- 11. Feature plugins (Radar & Search)
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