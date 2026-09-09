-- 迁移 2026-07-22: 添加数据保险箱表
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
