-- Migration: V20260701_001__create_extension_tables.sql
-- Description: 创建扩展配置相关表（系统变量、扩展配置）
-- Author: System
-- Date: 2026-07-01

-- 1. 创建系统变量表
CREATE TABLE IF NOT EXISTS system_variables (
    key VARCHAR(100) PRIMARY KEY,
    value TEXT,
    is_secret BOOLEAN DEFAULT FALSE,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_variables_secret ON system_variables(is_secret);

-- 2. 创建扩展配置表
CREATE TABLE IF NOT EXISTS source_plugins (
    id VARCHAR(50) PRIMARY KEY,
    name VARCHAR(100) NOT NULL,
    plugin_class VARCHAR(255) NOT NULL,
    description TEXT,
    enabled BOOLEAN DEFAULT TRUE,
    version VARCHAR(20),
    required_variables TEXT,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_plugins_enabled ON source_plugins(enabled);

-- 3. 插入预置扩展
INSERT OR IGNORE INTO source_plugins (id, name, plugin_class, description, enabled, version, required_variables) VALUES
('github', 'GitHub', 'plugins::GitHubPlugin', '获取 GitHub 项目信息、健康度评分', TRUE, '1.0.0',
 '[
   {"key": "github_token", "name": "GitHub Token", "secret": true},
   {"key": "github_proxy", "name": "代理地址", "secret": false}
 ]'),
('gitee', 'Gitee', 'plugins::GiteePlugin', '获取 Gitee 项目信息、健康度评分', TRUE, '1.0.0',
 '[
   {"key": "gitee_token", "name": "Gitee 私有令牌", "secret": true}
 ]'),
('crawler', '通用爬虫', 'plugins::CrawlerPlugin', '无 Token 时的降级方案', TRUE, '1.0.0', '[]');
