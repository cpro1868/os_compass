-- Migration: V20260702_001__add_runbook_column.sql
-- Description: 添加 runbook 列到 projects 表
-- Date: 2026-07-02

ALTER TABLE projects ADD COLUMN IF NOT EXISTS runbook TEXT;
