-- Migration: V20260629_001__create_initial_schema.sql
-- Description: 创建初始数据库表结构
-- Author: System
-- Date: 2026-06-29

-- 1. 创建分类表（先创建，因为 projects 表有外键依赖）
CREATE TABLE IF NOT EXISTS categories (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name VARCHAR(100) NOT NULL,
    path VARCHAR(255) NOT NULL UNIQUE,
    explain TEXT,
    sort_order INTEGER DEFAULT 0,
    parent_id INTEGER REFERENCES categories(id),
    is_system BOOLEAN DEFAULT FALSE,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_categories_parent ON categories(parent_id);
CREATE INDEX idx_categories_path ON categories(path);
CREATE INDEX idx_categories_sort ON categories(sort_order);

-- 2. 创建项目主表
CREATE TABLE IF NOT EXISTS projects (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name VARCHAR(255) NOT NULL,
    url TEXT NOT NULL UNIQUE,
    source VARCHAR(20) NOT NULL CHECK(source IN ('github', 'gitee', 'npm', 'pypi', 'crawler', 'unknown')),
    lifecycle_status VARCHAR(20) NOT NULL DEFAULT 'TO_EXPLORE'
        CHECK(lifecycle_status IN ('TO_EXPLORE', 'DIVING', 'IN_USE', 'ABANDONED')),
    data_status VARCHAR(20) NOT NULL DEFAULT 'ACTIVE'
        CHECK(data_status IN ('ACTIVE', 'ARCHIVED', 'DELETED')),
    stars INTEGER DEFAULT 0,
    languages TEXT NOT NULL DEFAULT '[]',
    description TEXT,
    readme_versions TEXT NOT NULL DEFAULT '{}',
    readme_selected_lang VARCHAR(10) DEFAULT 'en',
    readme_translations TEXT NOT NULL DEFAULT '{}',
    readme_hash VARCHAR(64),
    readme_last_fetched DATETIME,
    summary VARCHAR(500),
    applicable_scenarios TEXT,
    category_id INTEGER REFERENCES categories(id),
    is_downloaded BOOLEAN DEFAULT FALSE,
    local_path TEXT,
    health_score INTEGER,
    license VARCHAR(50),
    runbook TEXT,
    archived_at DATETIME,
    deleted_at DATETIME,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_projects_status ON projects(lifecycle_status);
CREATE INDEX idx_projects_data_status ON projects(data_status);
CREATE INDEX idx_projects_category ON projects(category_id);
CREATE INDEX idx_projects_source ON projects(source);
CREATE INDEX idx_projects_updated ON projects(updated_at DESC);
CREATE INDEX idx_projects_stars ON projects(stars DESC);
CREATE INDEX idx_projects_languages ON projects(languages);

-- 3. 创建标签表
CREATE TABLE IF NOT EXISTS tags (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name VARCHAR(50) NOT NULL UNIQUE,
    color VARCHAR(7),
    source VARCHAR(20) NOT NULL DEFAULT 'user'
        CHECK(source IN ('ai', 'user', 'preset')),
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE UNIQUE INDEX idx_tags_name ON tags(name);

-- 4. 创建项目-标签关联表
CREATE TABLE IF NOT EXISTS project_tags (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    project_id INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    tag_id INTEGER NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(project_id, tag_id)
);

CREATE INDEX idx_project_tags_project ON project_tags(project_id);
CREATE INDEX idx_project_tags_tag ON project_tags(tag_id);

-- 5. 创建项目笔记表
CREATE TABLE IF NOT EXISTS project_notes (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    project_id INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    content TEXT NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_notes_project ON project_notes(project_id);

-- 6. 创建用户信息表（数据保险箱）
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

CREATE INDEX idx_user_info_project ON project_user_info(project_id);

-- 7. 创建 AI 报告表
CREATE TABLE IF NOT EXISTS ai_reports (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    project_id INTEGER NOT NULL UNIQUE REFERENCES projects(id) ON DELETE CASCADE,
    health_score INTEGER,
    commit_activity INTEGER,
    issue_rate INTEGER,
    release_frequency INTEGER,
    star_score INTEGER,
    contributor_score INTEGER,
    license_score INTEGER,
    report_content TEXT,
    generated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_reports_project ON ai_reports(project_id);

-- 8. 创建默认分类
INSERT OR IGNORE INTO categories (id, name, path, is_system) VALUES (1, '未分类', 'uncategorized', TRUE);
