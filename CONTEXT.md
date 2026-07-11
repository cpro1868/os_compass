> **👋 新会话必读**：如果你是 AI Agent 或开发者刚接入本项目，请先阅读 `AGENTS.md` 和 `CONTEXT.md`，然后查看 `CURRENT-TASK.md` 了解当前任务。

# OS-Compass 当前上下文

## 项目一句话

OS-Compass（开源罗盘）是一款基于"本地优先 (Local-First)"理念设计的跨平台桌面端开源项目全生命周期管理与决策系统。

## 当前阶段

**Phase 1 MVP 全部完成，工程化改进完成（Git + 测试框架 + CI）。**

## 当前重点任务

无活跃任务。工程化改进已完成（Git 仓库初始化 + vitest 测试框架 + GitHub Actions CI），等待用户指定下一步（Phase 2 M13 意图搜索或继续打磨）。

## 最近完成

- ✅ 工程化改进：Git 仓库初始化（3 次 commit）+ vitest 测试框架（18 个测试）+ GitHub Actions CI + pre-commit 脚本
- ✅ i18n 测试发现并修复 en.json 缺失 detail.aiAnalysis 键
- ✅ github_token 防御性修复：解密失败不再静默清空密文，改为保留+警告日志
- ✅ i18n 覆盖率提升：9/20 → 17/20 组件接入 useTranslation（实际 100%）
- ✅ 修复 en.json 重复 detail 块问题
- ✅ M11 国际化与体验优化全部完成
- ✅ 导入已有仓库功能实现（`import_vault` 命令 + 9 步校验链）
- ✅ 安全修复：凭据迁移泄露（`apply_legacy_settings` 只迁移用户偏好）
- ✅ 安全修复：硬编码代理地址/端口清除
- ✅ 加密密钥跟随仓库（每个仓库独立 `.cryptokey`）
- ✅ 配置存储迁移（settings.db → app_settings 表）
- ✅ Phase 1 MVP 核心功能（M2-M10 全部完成）

## 关键决策

| 决策项 | 结论 |
|--------|------|
| 桌面框架 | Tauri 2.0+ |
| 前端技术栈 | React 18 + TypeScript 5 + Tailwind CSS 3 + Zustand |
| 后端技术栈 | Rust 1.75+ + sqlx |
| 数据库 | SQLite 3.40+，启用 WAL 模式和外键约束 |
| 敏感信息加密 | 每个仓库独立 AES-256-GCM 密钥（`.cryptokey`），敏感字段加密存储 |
| 凭据迁移 | 只迁移用户偏好，禁止迁移 LLM/Proxy/Google 凭据（安全工程方法论第 8 章） |
| LLM 集成 | 兼容 OpenAI API 的多供应商方案 |
| 插件架构 | MVP 内置"情报雷达"，第三方扩展为二期预留 |
| 项目详情标签页 | 6 个：概览 / AI 报告 / Runbook / README / 本地笔记 / 用户信息 |

## 必读文档

1. `docs/产品需求文档 (PRD).md`
2. `docs/概要设计说明书.md`
3. `docs/详细设计说明书.md`
4. `docs/数据库设计说明书.md`
5. `docs/API接口设计说明书.md`
6. `docs/技术架构设计说明书.md`
7. `docs/里程碑和开发计划.md`
8. `docs/开发连续性保障机制.md`

## 下一步

1. **Phase 1 MVP 已全部完成**（M1-M12 全部 ✅）
2. 下一步进入 **Phase 2 二期功能**（意图搜索、情报雷达、插件系统等）
3. 或根据用户需求做整体打磨和用户测试
4. 定期检查 `docs/安全工程方法论.md` 合规性

## 恢复上下文指南

如果你是新接入的 Agent，请按以下顺序读取文件：

1. `AGENTS.md`
2. 本文件（`CONTEXT.md`）
3. `CURRENT-TASK.md`
4. `docs/工作进展记录.md`（最近 3-5 条）
5. `docs/里程碑和开发计划.md`
