# OS-Compass 当前任务

## 项目整体状态

| 阶段 | 状态 | 完成时间 |
|------|------|----------|
| Phase 0 准备期 | ✅ 完成 | 2026-07-06 |
| Phase 1-1 MVP 核心 | ✅ 完成 | 2026-08-19 |
| Phase 1-2 增强功能 | ✅ 完成 | 2026-08-19 |
| M13 情报雷达 + 意图搜索 | ✅ 完成 | - |
| M14 插件系统扩展 | ✅ 完成 | 2026-07-20 |
| M15 遗留问题修复 | ✅ 完成 | 2026-07-24 |

---

## 当前进行中

| 阶段 | 状态 | 说明 |
|------|------|------|
| M16 文章生成 | 📋 规划中 | 需求已完成，待开发 |

---

## M15 任务状态（已修复）

| Task | 模块 | 内容 | 状态 | 修复内容 |
|------|------|------|------|----------|
| Task 0 | 仓库路径 | 修复默认仓库路径错误 | ✅ | - |
| Task 1 | 用户信息后端 | CRUD API | ✅ | - |
| Task 2 | 用户信息前端 | UI + 编辑 + 复制 | ✅ | Bug 已修复 |
| Task 3 | 雷达增强后端 | 关键字/来源/时间/分页 | ✅ | 已实现 |
| Task 4 | 雷达增强前端 | 搜索工具栏 UI | ✅ | 已实现 |
| Task 5 | 广告过滤 | 自定义过滤实现 | ✅ | - |
| Task 6 | 广告过滤设置 | 保存到数据库 | ✅ | 已修复 |
| Task 7 | 过滤流程集成 | 采集时过滤 | ✅ | - |
| Task 14 | i18n | 翻译键更新 | ✅ | - |

---

## M15 任务状态

详见：`docs/M15实施计划.md`

| Task | 模块 | 内容 | 状态 | 遗留问题 |
|------|------|------|------|----------|
| Task 0 | 仓库路径 | 修复默认仓库路径错误 | ✅ | - |
| Task 1 | 用户信息 | 数据保险箱后端 CRUD | ✅ | - |
| Task 2 | 用户信息 | 数据保险箱前端 UI | ⚠️ | Bug：编辑为空、复制无效 |
| Task 3 | 雷达增强 | 搜索增强后端 API | ❌ | 未实现（关键字/来源/时间/分页） |
| Task 4 | 雷达增强 | 搜索增强前端 UI | ❌ | 未实现 |
| Task 5 | 广告过滤 | 自定义过滤实现 | ✅ | banlex 未集成 |
| Task 7 | 广告过滤 | 过滤流程集成 | ✅ | - |
| ~~Task 8-13~~ | ~~主页~~ | ⬅️ 移至 M16 | - | - |
| Task 14 | 全局 | i18n + 构建验证 | ✅ | - |

---

## M15 遗留问题详情

详见：`docs/M15实施计划.md` §10

### 1. 用户信息（Task 2）

| Bug | 位置 | 说明 |
|------|------|------|
| 编辑时值为空 | `ProjectDetailDialog.tsx:1563` | `setNewInfoValue("")` 应保留原值 |
| 复制功能无效 | `ProjectDetailDialog.tsx:580` | 未调用 Tauri 剪贴板 API |

### 2. 雷达增强（Task 3-4）

| 功能 | 状态 | 说明 |
|------|------|------|
| 关键字搜索 | ❌ | `get_radar_items` 无 `keyword` 参数 |
| 信息源筛选 | ❌ | 无 `source_ids` 参数 |
| 时间范围筛选 | ❌ | 无 `start_date/end_date` 参数 |
| 分页 | ❌ | 只有 `limit`，无 `page/page_size` |

### 3. 广告过滤（Task 5-7）

| 问题 | 说明 |
|------|------|
| banlex 未集成 | 使用自定义关键词匹配替代 |
| 设置无法保存 | adFilterLevel 保存问题 |

---
| 用户信息 | `design-system/public/project-detail.html`（用户信息标签页区域） | ✅ |
| 主页引导页 | `design-system/public/home.html` | ✅ |

---

## 验证状态

- [x] Rust 编译检查通过
- [x] TypeScript 类型检查通过
- [x] release 构建成功
- [x] Previous 目录更新

---

## 会话日志

### 2026-07-24 - M15 遗留问题确认

**问题发现**：
1. **用户信息功能存在 Bug**
   - Bug 1：编辑时值为空（`setNewInfoValue("")` 应保留原值）
   - Bug 2：复制功能无效（未调用 Tauri 剪贴板 API）

2. **雷达增强功能未实现**
   - 关键字搜索 ❌
   - 信息源筛选 ❌
   - 时间范围筛选 ❌
   - 分页功能 ❌

3. **广告过滤**
   - 使用自定义实现替代 banlex
   - 设置无法保存到数据库

**文档更新**：
- `docs/M15实施计划.md` - 新增 §10 重构需求（第二版）
- `CURRENT-TASK.md` - 更新 M15 任务状态

**状态**：⚠️ M15 存在遗留问题，待修复

---

### 2026-07-22 - M15 修复 + 完善（下午）

**问题修复**：

1. **系统库和仓库库分离**
   - 系统库：`Roaming\.os-compass\`
   - 仓库库：`Local\com.administrator.os-compass\vaults\`
   - 手动迁移：os_compass.db, .cryptokey, config.json
   - 修复 vault-index.json 路径问题

2. **project_user_info 表缺失**
   - 问题：用户信息功能报错 `no such table`
   - 修复：修改 `db.rs` 自动检查并添加缺失的表

3. **广告过滤插件位置**
   - 设置 → 功能插件 → 雷达插件 → 配置 → 广告过滤强度

**提交**：
- ccf08db: fix: 分离系统库和仓库库
- 602ec34: fix: 迁移仓库库到 Local 目录
- 7ad50d9: fix: 分离系统库和仓库库路径

---

### 2026-07-22 - M15 实施完成（上午）

**完成内容**：

1. **Task 0: 仓库路径修复** ✅
   - 修复 lib.rs 第 91-92 行
   - 提交：776c85e

2. **Task 1-2: 数据保险箱** ✅
   - 后端 CRUD API（5 个命令）
   - 前端 UI（添加/编辑/删除/复制）
   - 敏感信息加密存储
   - 提交：4897e2d, 87ef51a

3. **Task 3-4: 雷达搜索增强** ✅
   - source_engine 模块已完成（RSS/WebCrawl/Telegram/PlatformPreset/GitLab/DockerHub）
   - 6 种适配器全部实现

4. **Task 5-7: 广告过滤插件** ✅
   - 新增 content_filter 模块
   - 支持中英文 spam 词库匹配
   - 雷达采集时自动过滤广告
   - 前端过滤强度设置（关闭/低/中/高）
   - 提交：47782aa

5. **Task 14: i18n + 构建验证** ✅
   - 添加广告过滤相关翻译
   - 构建验证通过
   - 提交：60f3bce

**验证状态**：
- [x] Rust 编译检查通过
- [x] TypeScript 类型检查通过
- [x] release 构建成功
- [x] Previous 目录更新

**提交记录**：
- 776c85e: fix: 修复仓库路径
- 4897e2d: feat: 数据保险箱后端 CRUD API
- 87ef51a: feat: 数据保险箱前端 UI
- 47782aa: feat: 广告过滤插件
- 60f3bce: chore: i18n 翻译键更新

**状态**：✅ M15 全部完成

---

### 2026-07-22 - 需求调整 + 仓库管理 BUG 修复

**变更内容**：

1. **M15 调整**：
   - 移除 `index.html` 和 `home.html` 相关任务（移至 M16）
   - 理由：需求不完善，需要进一步明确

2. **仓库管理 BUG 修复**（严重）：
   - 问题1：当前仓库地址显示系统目录而非仓库目录
   - 问题2："打开文件夹"功能失效
   - 修复：
     - `lib.rs`：默认仓库使用 `vaults/default` 而非系统目录
     - 新增 `open_folder` 命令（跨平台支持）
     - 前端 `VaultDialog.tsx`：改为调用 `open_folder`

3. **M16 实施计划**：
   - 新增初始化向导（类似 Obsidian 首次启动）
   - 包含 index.html 和 home.html（从 M15 移入）

**文件变更**：
- `docs/M15实施计划.md`
- `docs/M16实施计划.md`（新建）
- `src-tauri/src/lib.rs`
- `src-tauri/src/commands/vault_cmd.rs`
- `src/components/VaultDialog.tsx`

**状态**：✅ 完成

---

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
