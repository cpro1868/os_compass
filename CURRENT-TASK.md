# OS-Compass 当前任务

## 项目整体状态

| 阶段 | 状态 | 完成时间 |
|------|------|----------|
| Phase 0 准备期 | ✅ 完成 | 2026-07-06 |
| Phase 1-1 MVP 核心 | ✅ 完成 | 2026-08-19 |
| Phase 1-2 增强功能 | ✅ 完成 | 2026-08-19 |
| M13 情报雷达 + 意图搜索 | ✅ 完成 | - |
| M14 插件系统扩展 | ✅ 完成 | 2026-07-20 |

---

## 当前进行中

| 阶段 | 状态 | 说明 |
|------|------|------|
| M15 主页增强 + 用户信息 + 雷达增强 + 广告过滤 | 📋 待开发 | 详见下方任务清单 |

---

## M15 待开发任务清单

详见：`docs/M15实施计划.md`

| Task | 模块 | 内容 | 工时 | 状态 |
|------|------|------|------|------|
| Task 1 | 用户信息 | 数据保险箱后端 CRUD | 0.4d | 📋 |
| Task 2 | 用户信息 | 数据保险箱前端 UI | 0.4d | 📋 |
| Task 3 | 雷达增强 | 搜索增强后端 API | 0.4d | 📋 |
| Task 4 | 雷达增强 | 搜索增强前端 UI | 0.5d | 📋 |
| Task 5 | 广告过滤 | banlex 集成 + 中文词库 | 0.5d | 📋 |
| Task 6 | 广告过滤 | 过滤设置 UI | 0.3d | 📋 |
| Task 7 | 广告过滤 | 过滤流程集成 | 0.2d | 📋 |
| Task 8 | 主页 | 首页原型设计 | 1d | 📋 |
| Task 9 | 主页 | 极简输入框组件 | 0.5d | 📋 |
| Task 10 | 主页 | URL 平台识别逻辑 | 0.5d | 📋 |
| Task 11 | 主页 | 首页 UI 实现 | 1d | 📋 |
| Task 12 | 主页 | 导航优化（Logo/侧边栏） | 0.5d | 📋 |
| Task 13 | 主页 | 新手引导组件 | 1d | 📋 |
| Task 14 | 全局 | i18n + 构建验证 | 0.3d | 📋 |

**M15 总工时**：6.5d

---

## 已完成原型

| 原型 | 路径 | 状态 |
|------|------|------|
| 情报雷达搜索增强 | `design-system/public/radar-inbox.html` | ✅ |
| 广告过滤设置 | `design-system/public/radar-filter-settings.html` | ✅ |
| 用户信息 | `design-system/public/project-detail.html`（用户信息标签页区域） | ✅ |
| 主页 | `design-system/public/home.html` | 📋 待设计 |

---

## 验证状态

- [x] Rust 编译检查通过
- [x] TypeScript 类型检查通过
- [x] release 构建成功
- [x] Previous 目录更新

---

## 会话日志

### 2026-07-21 - 需求探讨：三个新需求确认

**新增三个需求**：

1. **用户信息（数据保险箱）**
   - 项目级敏感信息存储（Token、API Key 等）
   - 字段：key、value（加密存储）、remark（明文）
   - 状态：需求明确，可开发

2. **情报雷达搜索增强**
   - 关键字搜索、来源筛选、时间范围筛选
   - 分页浏览
   - 状态：原型已完成

3. **广告和无效信息过滤插件**
   - banlex 词库检测 + 中文扩展
   - 强度控制（关闭/低/中/高）
   - 状态：方案确定，原型已完成

**原型更新**：
- `design-system/public/radar-inbox.html` - 搜索增强功能
- `design-system/public/radar-filter-settings.html` - 广告过滤设置

**文档更新**：
- `docs/产品需求文档 (PRD).md`
- `docs/概要设计说明书.md`
- `docs/requirements.md`
- `docs/需求确认文档.md`
- `docs/里程碑和开发计划.md`
- `docs/M13实施计划.md`（Task 18）
- `docs/M15实施计划.md`（全面重构）

**提交**：
- b9659ad: docs: update all docs with radar search enhancement feature
- 0f4bdce: docs: update M15 plan with user info, radar enhancement, and ad filter
- 869c1b4: docs: add ad filter plugin spec with banlex integration

**状态**：📋 需求确认完成，待开发实施

---

### 2026-07-21 - 路径显示修复

**问题**：设置页面显示的雷达数据路径与实际存储路径不一致

**原因**：
1. 前端用 `app.path().app_data_dir()`（`%APPDATA%/com.os-compass.os-compass/`）
2. 后端用 `BaseDirs::data_dir().join(".os-compass")`（`%APPDATA%/.os-compass/`）

**修复**：
1. 后端新增 `get_radar_data_dir` 命令
2. 前端雷达数据路径使用独立命令获取

**提交**：
- b7f8372: fix: unify path separator to forward slash in radar data dir
- 5a2d7d4: fix: plugin data path display - use separate command for radar data dir

**状态**：✅ 完成

---

### 2026-07-19 - 情报雷达数据加载失败

**问题**：`no such table: radar_sources`

**根因**：跨库 JOIN 查询

**修复**：分两步查询，应用层合并数据

**提交**：d0463b8

**状态**：✅ 完成

---

### 2026-07-17 - 情报雷达系统性修复

**修复内容**：
1. 数据架构：信息源→系统库，采集数据→仓库库
2. 代理配置：优先读取信息源自己的设置
3. Telegram 适配器：4种解析方式
4. 前端：编辑/删除信息源功能

**提交**：
- ebfc24b, a924ac5, 170d185, ac8315b, bdd5a7a, 200c3a8, d0436ba

**状态**：✅ 完成

---

### 2026-07-16 - 情报雷达数据架构重构

**问题**：采集后无数据

**需求**：信息源放系统库，采集数据放仓库

**提交**：ebfc24b, a924ac5, d0436ba

**状态**：✅ 完成

---

### 2026-07-14 - Bug 修复

**问题**：
1. LLM 配置丢失
2. 扩展配置被清空

**提交**：6cb34ca

**状态**：✅ 完成
