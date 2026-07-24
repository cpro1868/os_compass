> **重要提示**：
> 1. 如果你是新接入的 Agent（无论使用 Kimi Code、OpenCode、Cursor、Copilot、Claude Code 还是其他工具），**请务必先阅读本文件**。
> 2. 本项目的所有上下文都通过文件系统维护，**不要依赖前一个会话的内存**。
> 3. 敏感信息（API Key、密码、Token、完整本地路径）**严禁写入任何文件**。
> 4. **修改前必须充分理解需求，不确定时请先读需求文档，禁止盲目修改后让用户调试**。

---

## ⚠️ 系统核心概念：系统库 vs 仓库库（必须理解）

**这是 OS-Compass 的基石概念，所有功能开发前必须明确数据存放位置！**

### 系统库（全局不变）
- **路径**：`%APPDATA%\.os-compass\plugins.db`
- **内容**：全局配置和插件配置
- **切换仓库时**：不变
- **示例表**：`radar_sources`（情报源）、`system_plugins`（插件状态）、`article_prompts`（Prompt 模板）

### 仓库库（跟随仓库切换）
- **路径**：`<用户选择的仓库目录>\os_compass.db`
- **内容**：用户数据和业务数据
- **切换仓库时**：切换到新仓库的数据库
- **示例表**：`projects`（项目）、`categories`（分类）、`radar_items`（雷达采集数据）

### 数据存放决策树

```
新功能需要存储数据？
├── 是否全局共享（不随仓库切换）？
│   ├── 是 → 存系统库 `plugins.db`
│   └── 否 → 存仓库库 `os_compass.db`
└──
```

### ⚠️ 错误代价
- 放错位置会导致：数据丢失、功能异常、仓库切换后数据消失
- **不确定时必须阅读** `docs/系统关键错误记录.md` 中的 #001-#007

### 常见数据对应表

| 数据 | 存放位置 | 原因 |
|------|----------|------|
| LLM 配置 | 系统库 | 全局共享 |
| 代理配置 | 系统库 | 全局共享 |
| 插件启用状态 | 系统库 | 全局共享 |
| 情报源 | 系统库 | 全局共享 |
| Prompt 模板 | 系统库 | 全局共享 |
| 项目数据 | 仓库库 | 按仓库隔离 |
| 分类数据 | 仓库库 | 按仓库隔离 |
| 雷达采集数据 | 仓库库 | 按仓库隔离 |

---

## ⚠️ 关键开发规则（必须遵守）

### 规则 1：修改前必须理解需求
- 修改代码前必须充分理解当前功能和需求
- 不确定时先阅读相关设计文档
- **禁止盲目修改后让用户调试**

### 规则 2：阶段区分规则（最重要）

**不同阶段的操作范围完全不同，违反将导致严重问题！**

| 阶段 | 可操作 | 禁止操作 |
|------|--------|----------|
| **需求讨论** | 口头讨论、文档记录 | 代码、编译、构建 |
| **原型设计** | `.html` 原型文件 | 代码、编译、构建 |
| **设计确认** | 确认设计文档 | 代码、编译、构建 |
| **开发实施** | 开始编写代码 | - |

**代码文件定义**：`src/` 目录下所有文件（`.tsx`, `.ts`, `.rs` 等）

**判断方法**：
- 用户说"讨论"、"设计"、"原型"、"需求" → 设计阶段，不写代码
- 用户说"开始开发"、"实施"、"写代码" → 开发阶段
- 用户未明确说明 → 主动询问确认阶段

### 规则 4：测试验证
- 每次修改后必须自行验证
- 运行 `pnpm tauri build` 确保编译通过
- 检查 `Previous/os-compass.exe` 确保程序可用
- **不要依赖用户帮忙调试**

### 规则 5：提交规范
- 提交前运行 `git diff` 检查变更
- 提交信息清晰说明做了什么
- 重要变更记录到 `docs/系统关键错误记录.md`

---

## 1. 项目简介

**OS-Compass（开源罗盘）** 是一款基于"本地优先 (Local-First)"理念设计的跨平台桌面端开源项目全生命周期管理与决策系统。

- **技术栈**：Tauri 2.0 + React 18 + TypeScript 5 + Tailwind CSS 3 + Zustand + SQLite
- **当前阶段**：设计阶段已收尾，开发环境出现阻塞，待解决
- **当前任务**：见 `CURRENT-TASK.md`

---

## 2. 新会话恢复流程

每次新会话开始时，请按以下顺序读取文件：

```
1. AGENTS.md（本文件）
2. CONTEXT.md
3. CURRENT-TASK.md
4. docs/工作进展记录.md（最近 3-5 条）
5. docs/里程碑和开发计划.md（当前阶段）
```

读取完成后，继续执行 `CURRENT-TASK.md` 中描述的当前任务。如果当前任务已完成，则从里程碑计划中选择下一个任务。

---

## 3. 工作规范

### 3.1 会话开始

- 读取上述恢复流程中的文件
- 更新 `SESSION-LOG.md` 记录会话开始
- 如果 `CURRENT-TASK.md` 状态为 `done`，选择下一个任务并更新

### 3.2 任务执行

- 使用 `TodoList` 工具跟踪原子任务
- 每个任务应独立完成，周期控制在 30-120 分钟
- 优先读取相关设计文档，再动手修改代码
- 所有修改必须落到文件，不能只停留在对话中

### 3.3 会话结束

每次会话结束前，必须完成以下动作：

1. **验证**：运行相关测试或检查，确保没有破坏现有功能
2. **记录**：追加 `docs/工作进展记录.md`
3. **更新状态**：更新 `CURRENT-TASK.md`
4. **记录日志**：追加 `SESSION-LOG.md`
5. **提交**：执行 `git commit`（如果有代码/文档变更）
6. **检查点**：如完成重要节点，创建 `checkpoints/YYYY-MM-DD_HH-MM_名称.md`

**时间记录规范**：
- 时间格式：`YYYY-MM-DD HH:MM`（24小时制）
- 必须使用 **bash `date` 命令获取当前准确时间**，不得自行估算或编造
- 时间应为北京时间（UTC+8），如果系统时间为 UTC，需要加 8 小时
- 每条工作记录的时间必须真实反映工作完成的实际时间

### 3.4 异常中断

如果会话意外中断，新会话应：

1. 从 `CURRENT-TASK.md` 恢复当前任务
2. 从 `SESSION-LOG.md` 查看最后操作
3. 用 `git status` 检查未提交变更
4. 继续或回退到最近的检查点

---

## 4. 关键文件索引

| 文件 | 用途 | 更新频率 |
|------|------|---------|
| `AGENTS.md` | Agent 协作指南（本文件） | 机制变更时 |
| `CONTEXT.md` | 项目快速上下文 | 阶段/里程碑变化时 |
| `CURRENT-TASK.md` | 当前任务状态 | 任务状态变化时 |
| `SESSION-LOG.md` | 当前会话日志 | 关键步骤/异常时 |
| `docs/工作进展记录.md` | 历史工作记录 | 每次会话结束 |
| `docs/里程碑和开发计划.md` | 开发路线图 | 计划调整时 |
| `docs/WBS分解.md` | 工作分解结构 | 范围变更时 |
| `docs/开发连续性保障机制.md` | 连续性机制详细说明 | 机制变更时 |
| `docs/开发前检查清单.md` | 编码前准备事项 | 初始化前确认 |
| `docs/插件接入规范.md` | FeaturePlugin 接入规范（二期） | 插件机制变更时 |
| `docs/M13实施计划.md` | M13 二期实施计划（14 Task） | 实施进度变化时 |
| `docs/M14实施计划.md` | M14 插件系统实施计划（8 Task） | 实施进度变化时 |
| `docs/M15实施计划.md` | M15 主页增强实施计划（5 Task） | 实施进度变化时 |
| `checkpoints/` | 检查点目录 | 重要节点 |
| `Previous/` | **可运行版本目录** | **每次构建成功后更新** |

### Previous 目录规范

**用途**：存放最新可运行的版本文件，供用户直接双击使用

**必须包含的文件**：
```
Previous/
├── os-compass.exe      # 主程序（必选）
├── WebView2Loader.dll # WebView2 依赖（必选）
├── os_compass.db       # 空数据库文件（如需要）
└── settings.db        # 默认设置文件（如需要）
```

**发布流程**：
1. 构建成功后
2. 将 `src-tauri/target/release/os-compass.exe` 复制到 `/Previous`
3. 将 `src-tauri/target/release/WebView2Loader.dll` 复制到 `/Previous`
4. 不要复制安装包（.msi、-setup.exe）
5. 可选：复制 web2md.exe 到 `/Previous`（爬虫服务）

---

## 5. 安全红线

- ❌ 禁止将 API Key、密码、Token 写入任何文件
- ❌ 禁止将完整本地敏感路径写入文档
- ❌ 禁止将 `.env`、`config.json`、数据库文件提交到 Git
- ✅ 敏感配置使用 `config.example.json` 模板
- ✅ API Key 使用 OS 密钥库存储

---

## 6. 技术约束

- 前端：React 18 + TypeScript 5 + Tailwind CSS 3
- 状态管理：Zustand
- 后端：Rust 1.75+
- 数据库：SQLite 3.40+，启用 WAL 模式和外键约束
- 敏感数据：AES-256-GCM + 应用主密钥存 OS 密钥库
- 桌面框架：Tauri 2.0+
- 爬虫服务：web2md（Go CLI + HTTP API）

---

## 7. 开发命令

### 构建命令

```cmd
$env:PATH="D:\Soft\msys64\ucrt64\bin;D:\Soft\msys64\usr\bin;$env:PATH"
cd "G:\Projects\kimicode\os_compass\os-compass"
pnpm tauri build
```

### 测试命令

```cmd
# 前端测试（vitest）
cd "G:\Projects\kimicode\os_compass\os-compass"
pnpm test              # 运行一次
pnpm test:watch        # 监听模式
pnpm typecheck         # TypeScript 类型检查

# Rust 测试（注意：cargo test 因 Tauri GUI 依赖无法在 CLI 运行）
cd "G:\Projects\kimicode\os_compass\os-compass\src-tauri"
cargo check --lib      # 编译检查
cargo clippy --lib     # lint 检查

# 提交前检查
cd "G:\Projects\kimicode\os_compass"
powershell -ExecutionPolicy Bypass -File scripts\pre-commit.ps1
```

### Git 规范

- 仓库根目录：`G:\Projects\kimicode\os_compass`
- 分支：`main`（主分支）
- 提交前必须运行 `scripts/pre-commit.ps1` 或手动执行 typecheck + test
- 敏感文件（.db、.cryptokey、.env）已在 .gitignore 中排除

---

**最后更新**: 2026-07-24（新增系统库 vs 仓库库核心概念）
