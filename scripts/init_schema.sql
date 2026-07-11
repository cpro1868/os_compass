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

-- 2. System variables (user + extension variables)
CREATE TABLE IF NOT EXISTS system_variables (
    key TEXT PRIMARY KEY,
    value TEXT,
    is_secret INTEGER DEFAULT 0,
    created_at TEXT DEFAULT (datetime('now', 'localtime')),
    updated_at TEXT DEFAULT (datetime('now', 'localtime'))
);

-- 3. App settings (system configuration)
CREATE TABLE IF NOT EXISTS app_settings (
    key TEXT PRIMARY KEY,
    value TEXT,
    is_secret INTEGER DEFAULT 0,
    updated_at TEXT DEFAULT (datetime('now', 'localtime'))
);

-- 4. Source plugins
CREATE TABLE IF NOT EXISTS source_plugins (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    plugin_class TEXT NOT NULL,
    description TEXT,
    enabled INTEGER DEFAULT 1,
    version TEXT,
    required_variables TEXT,
    created_at TEXT DEFAULT (datetime('now', 'localtime')),
    updated_at TEXT DEFAULT (datetime('now', 'localtime'))
);

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

-- 9. Indexes
CREATE INDEX IF NOT EXISTS idx_projects_category ON projects(category_id);
CREATE INDEX IF NOT EXISTS idx_projects_status ON projects(lifecycle_status, data_status);
CREATE INDEX IF NOT EXISTS idx_project_notes_project ON project_notes(project_id);
CREATE INDEX IF NOT EXISTS idx_variables_secret ON system_variables(is_secret);
CREATE INDEX IF NOT EXISTS idx_plugins_enabled ON source_plugins(enabled);
CREATE INDEX IF NOT EXISTS idx_releases_project ON project_releases(project_id);
CREATE INDEX IF NOT EXISTS idx_releases_published ON project_releases(published_at DESC);

-- 10. Seed data
INSERT OR IGNORE INTO categories (id, name, sort_order) VALUES (1, '未分类', 0);

INSERT OR IGNORE INTO source_plugins (id, name, plugin_class, description, enabled, version, required_variables) VALUES
('github', 'GitHub', 'plugins::GitHubPlugin', 'Fetch GitHub project info and health score', 1, '1.0.0',
 '[{"key": "github_token", "name": "GitHub Token", "description": "GitHub Personal Access Token for GitHub API access","secret": true}, {"key": "github_proxy", "name": "GitHub Proxy", "description": "Proxy address for GitHub API access","secret": false}]'),
('gitee', 'Gitee', 'plugins::GiteePlugin', 'Fetch Gitee project info and health score', 1, '1.0.0',
 '[{"key": "gitee_token", "name": "Gitee Token", "description": "Gitee private token for Gitee API access","secret": true}]'),
('crawler', 'Crawler', 'plugins::CrawlerPlugin', 'Fallback when no Token available', 1, '1.0.0', '[]');

INSERT OR IGNORE INTO system_variables (key, value, is_secret) VALUES
('github_token', '', 1),
('github_proxy', '', 0),
('gitee_token', '', 1);

DELETE FROM system_variables WHERE key LIKE 'settings.%';