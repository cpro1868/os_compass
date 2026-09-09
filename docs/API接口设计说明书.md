# OS-Compass API 接口设计说明书

| 部门 | 研发部 |
|------|--------|
| 编写 | 系统架构师 |
| 审核 |  |
| 批准 |  |
| 日期 | 2026-07-27 |
| 版本 | V1.8 |

---

## 1 概述

### 1.1 编写目的

本文档定义 OS-Compass 系统的 API 接口规范，作为前端与后端、Tauri 与 Rust 服务之间通信的契约文档。

### 1.2 接口设计原则

| 原则 | 说明 |
|------|------|
| 统一通信方式 | 前后端通过 Tauri IPC 通信，使用 `invoke` 调用命令 |
| 统一返回格式 | 所有命令返回 `Result<T, String>`，成功返回 JSON，失败返回错误字符串 |
| 错误信息脱敏 | 错误信息不含敏感数据，仅返回用户可见的错误提示 |
| 类型安全 | TypeScript 类型与 Rust 类型严格对应 |

---

## 2 数据类型定义

### 2.1 枚举类型

```typescript
// 项目生命周期状态（看板列）
type LifecycleStatus = 'TO_EXPLORE' | 'DIVING' | 'IN_USE' | 'ABANDONED';

// 数据可用性状态
type DataStatus = 'ACTIVE' | 'ARCHIVED' | 'DELETED';

// 项目来源平台
type ProjectSource = 'github' | 'gitee' | 'npm' | 'pypi' | 'crawler' | 'unknown';

// 主题类型
type Theme = 'dark' | 'light' | 'system';

// 代理协议
type ProxyProtocol = 'http' | 'https' | 'socks5';
```

### 2.2 核心数据结构

#### 2.2.0 Vault（仓库）

```typescript
interface Vault {
  id: string;           // UUID
  name: string;         // 仓库名称
  path: string;         // 仓库路径
  project_count: number; // 项目数量
  created_at: string;   // 创建时间
  last_opened: string;  // 最后打开时间
}

interface VaultCreateParams {
  name: string;         // 仓库名称
  path: string;         // 存放目录（不含仓库文件夹名）
}

interface VaultValidationResult {
  valid: boolean;
  error?: string;       // 错误信息
  table_count?: number; // 表数量
  project_count?: number; // 项目数量
}
```

#### 2.2.1 Project（项目）

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| id | number | 是 | 主键 |
| name | string | 是 | 项目名称 |
| url | string | 是 | 项目地址 |
| source | ProjectSource | 是 | 来源平台 |
| lifecycle_status | LifecycleStatus | 是 | **看板生命周期状态** |
| data_status | DataStatus | 是 | **数据可用性状态** |
| stars | number | 否 | Star 数 |
| languages | string[] | 否 | **编程语言列表**（JSON 数组） |
| description | string | 否 | 项目描述 |
| readme_raw | string | 否 | README 原文 |
| readme_translated | string | 否 | README 翻译 |
| summary | string | 否 | AI 一句话总结 |
| applicable_scenarios | string[] | 否 | AI 提炼的适用场景 |
| tags | string[] | 否 | 技术标签 |
| category_id | number | 否 | 所属分类 ID |
| is_downloaded | boolean | 否 | 是否已下载 |
| local_path | string | 否 | 本地路径 |
| health_score | number | 否 | 健康度评分（0-100） |
| license | string | 否 | 许可证类型 |
| runbook | Runbook | 否 | Runbook 内容 |
| archived_at | string | 否 | 归档时间 |
| deleted_at | string | 否 | 删除时间 |
| created_at | string | 是 | 创建时间（ISO 8601） |
| updated_at | string | 是 | 更新时间（ISO 8601） |

**状态说明**：

| 状态维度 | 字段 | 状态值 | 说明 |
|----------|------|--------|------|
| 看板生命周期 | lifecycle_status | TO_EXPLORE | 待探索 |
| | | DIVING | 深度研究中 |
| | | IN_USE | 已落地/在用 |
| | | ABANDONED | 弃用/避坑 |
| 数据可用性 | data_status | ACTIVE | 正常状态，可显示 |
| | | ARCHIVED | 归档状态，隐藏显示，可恢复 |
| | | DELETED | 删除状态，隐藏显示，可恢复或彻底删除 |

#### 2.2.2 Runbook（操作手册）

| 字段 | 类型 | 说明 |
|------|------|------|
| requirements | string[] | 前置环境要求 |
| install_steps | string[] | 安装步骤 |
| run_steps | string[] | 运行步骤 |
| dev_entry | string[] | 二开入口路径 |

#### 2.2.3 Category（分类）

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| id | number | 是 | 主键 |
| name | string | 是 | 分类名称 |
| path | string | 是 | 路径标识，如 "frontend/react" |
| explain | string | 否 | 分类描述，供 AI 理解 |
| sort_order | number | 否 | 排序权重 |
| parent_id | number | 否 | 上级分类 ID |
| is_system | boolean | 否 | 是否系统分类 |
| children | Category[] | 否 | 子分类（仅用于树形结构展示） |

#### 2.2.4 AIReport（AI 报告）

| 字段 | 类型 | 说明 |
|------|------|------|
| id | number | 主键 |
| project_id | number | 项目 ID |
| health_score | number | 综合评分（0-100） |
| commit_activity | number | 提交活跃度评分 |
| issue_rate | number | Issue 闭环率评分 |
| release_frequency | number | 版本发布频率评分 |
| star_score | number | Star 评分 |
| contributor_score | number | 贡献者评分 |
| license_score | number | License 评分 |
| report_content | string | 完整报告内容 |
| generated_at | string | 生成时间 |

#### 2.2.5 ProjectNote（项目笔记）

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| id | number | 是 | 主键 |
| project_id | number | 是 | 项目 ID |
| content | string | 是 | 笔记内容 |
| created_at | string | 是 | 创建时间 |
| updated_at | string | 是 | 更新时间 |

#### 2.2.6 ProjectUserInfo（用户信息/数据保险箱）

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| id | number | 是 | 主键 |
| project_id | number | 是 | 项目 ID |
| info_key | string | 是 | 键名 |
| info_value | string | 否 | 值（敏感字段加密存储） |
| is_secret | boolean | 否 | 是否敏感 |
| created_at | string | 是 | 创建时间 |
| updated_at | string | 是 | 更新时间 |

#### 2.2.7 Tag（标签）

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| id | number | 是 | 主键 |
| name | string | 是 | 标签名称 |
| color | string | 否 | 标签颜色（十六进制） |
| source | 'ai' \| 'user' \| 'preset' | 是 | 来源：ai（LLM提取）/ user（用户添加）/ preset（预设） |
| created_at | string | 是 | 创建时间 |

#### 2.2.8 Settings（设置）

| 字段 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| theme | Theme | "dark" | 主题 |
| default_language | string | "zh-CN" | 默认阅读语言 |
| auto_translate_readme | boolean | true | 自动翻译 README |
| auto_translate_report | boolean | true | 自动翻译 AI 报告 |
| download_path | string | "" | 下载目录 |
| default_editor | string | "code" | 默认编辑器 |
| llm_provider | string | "openai" | LLM 供应商 |
| llm_api_base | string | "" | API Base URL |
| llm_api_key | string | null | API Key（加密存储） |
| llm_model | string | "gpt-4o" | 模型名称 |
| **LLM 代理** | 分组 | 用于 LLM API 调用 |
| llm_proxy_enabled | boolean | false | 是否启用 LLM 代理 |
| llm_proxy_protocol | string | "http" | 代理协议 |
| llm_proxy_host | string | "" | 代理地址 |
| llm_proxy_port | number | 0 | 代理端口 |
| llm_proxy_username | string | "" | 用户名 |
| llm_proxy_password | string | null | 密码（加密存储） |
| **下载代理** | 分组 | 用于 Git Clone 和爬虫 |
| proxy_enabled | boolean | false | 是否启用下载代理 |
| proxy_protocol | string | "http" | 代理协议 |
| proxy_host | string | "" | 代理地址 |
| proxy_port | number | 0 | 代理端口 |
| proxy_username | string | "" | 用户名 |
#### 2.2.9 ProjectClone（克隆配置）

| 字段 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| id | number | 自动 | 主键 |
| project_id | number | 必填 | 项目 ID |
| cloned_path | string | null | 克隆到的本地目录 |
| cloned_at | string | null | 克隆时间 |
| proxy_enabled | boolean | false | 是否启用代理 |
| proxy_protocol | string | "http" | 代理协议 |
| proxy_host | string | "" | 代理地址 |
| proxy_port | number | 0 | 代理端口 |
| proxy_username | string | "" | 代理用户名 |
| proxy_password | string | null | 代理密码 |
| **翻译引擎** | 分组 | 用于翻译功能 |
| translate_engine | string | "auto" | 翻译引擎：auto/google/llm |
| google_api_key | string | "" | Google 翻译 API Key |
| google_proxy_enabled | boolean | false | Google 翻译代理 |
| google_proxy_protocol | string | "http" | 代理协议 |
| google_proxy_host | string | "" | 代理地址 |
| **克隆设置** | 分组 | 用于项目克隆功能 |
| clone_path | string | "" | 全局克隆目录 |
| clone_proxy_enabled | boolean | false | 是否启用克隆代理 |
| clone_proxy_protocol | string | "http" | 代理协议 |
| clone_proxy_host | string | "" | 代理地址 |
| clone_proxy_port | number | 0 | 代理端口 |
| clone_proxy_username | string | "" | 用户名 |
| clone_proxy_password | string | null | 密码（加密存储） |

---

## 3 Tauri Commands 接口规范

### 3.1 命令调用约定

**前端调用方式**：

```typescript
import { invoke } from '@tauri-apps/api/core';

// 调用示例
const project = await invoke<Project>('import_project', {
  url: 'https://github.com/facebook/react',
  autoAnalyze: true,
  autoTranslate: true,
});
```

**后端 Rust 实现**：

```rust
#[tauri::command]
pub async fn import_project(
    url: String,
    auto_analyze: bool,
    auto_translate: bool,
    state: State<'_, AppState>,
) -> Result<Project, String> {
    // 实现逻辑
}
```

**命名规范**：
- 前端：camelCase（如 `autoAnalyze`）
- 后端：snake_case（如 `auto_analyze`）

### 3.2 项目管理接口

#### 3.2.1 import_project

导入项目（URL 自动解析或手动录入）。导入前检查 URL 唯一性，入库成功后触发后台异步任务。

| 参数 | 类型 | 必填 | 默认值 | 说明 |
|------|------|------|--------|------|
| url | string | 是 | - | 项目 URL |
| auto_analyze | boolean | 否 | true | 是否自动 AI 分析 |
| auto_translate | boolean | 否 | true | 是否自动翻译 README |
| category_id | number | 否 | null | 指定分类 ID |

**返回值**：ImportResponse

| 字段 | 类型 | 说明 |
|------|------|------|
| success | boolean | 是否成功 |
| project_id | number \| null | 项目 ID |
| error | string \| null | 错误信息 |
| data | ProjectData \| null | 项目数据 |
| duplicate | DuplicateInfo \| null | 重复项目信息（URL 已存在时返回） |

**错误码**：
- `INVALID_URL`：URL 格式无效
- `PLUGIN_NOT_FOUND`：不支持的平台
- `NETWORK_FAILED`：网络请求失败
- `DUPLICATE_PROJECT`：项目已存在

#### 3.2.1a post_import_tasks

异步执行导入后的后台任务（AI 分析、翻译、标签生成）。使用 `tauri::async_runtime::spawn` 脱离前端生命周期独立运行。

| 参数 | 类型 | 必填 | 默认值 | 说明 |
|------|------|------|--------|------|
| project_id | number | 是 | - | 项目 ID |
| generate_ai_report | boolean | 否 | true | 是否生成 AI 报告 |
| auto_translate | boolean | 否 | true | 是否自动翻译概览 |
| auto_translate_readme | boolean | 否 | true | 是否自动翻译 README |
| default_language | string | 否 | "zh-CN" | 目标语言 |

**返回值**：立即返回 `Ok(())`，实际任务在后台执行

**事件通知**：通过 `import:task_progress` 事件通知前端任务进度

| 事件载荷字段 | 类型 | 说明 |
|-------------|------|------|
| project_id | number | 项目 ID |
| task_type | string | 任务类型（ai_analysis/generate_tags/summary/use_cases/risks/dependencies/description/readme_translation/all） |
| status | string | 状态（running/done/failed） |
| error | string \| null | 错误信息 |

---

#### 3.2.2 get_projects

获取所有项目列表。

**返回值**：Project[]

---

#### 3.2.3 get_projects_by_status

按看板状态筛选项目。

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| lifecycle_status | LifecycleStatus | 是 | 看板生命周期状态 |
| include_archived | boolean | 否 | 是否包含归档项目（默认 false） |
| include_deleted | boolean | 否 | 是否包含已删除项目（默认 false） |

**返回值**：Project[]

---

#### 3.2.4 get_project

获取单个项目详情。

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| id | number | 是 | 项目 ID |

**返回值**：Project

---

#### 3.2.5 update_project_status

更新项目看板状态（看板拖拽）。

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| id | number | 是 | 项目 ID |
| lifecycle_status | LifecycleStatus | 是 | 新状态 |

**返回值**：void

---

#### 3.2.6 update_project

更新项目信息。

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| id | number | 是 | 项目 ID |
| name | string | 否 | 项目名称 |
| description | string | 否 | 项目描述 |
| summary | string | 否 | AI 一句话总结 |
| tags | string[] | 否 | 技术标签 |
| category_id | number | 否 | 所属分类 |
| runbook | Runbook | 否 | Runbook 内容 |

**返回值**：Project

---

#### 3.2.7 soft_delete_project

软删除项目（data_status → DELETED）。

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| id | number | 是 | 项目 ID |

**返回值**：void

**说明**：软删除后项目仍可恢复，不会删除本地文件。

---

#### 3.2.8 archive_project

归档项目（data_status → ARCHIVED）。

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| id | number | 是 | 项目 ID |

**返回值**：void

**说明**：归档不改变看板状态（lifecycle_status），仅改变数据可用性。

---

#### 3.2.9 restore_project

恢复项目（data_status → ACTIVE）。

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| id | number | 是 | 项目 ID |

**返回值**：void

**说明**：从 ARCHIVED 或 DELETED 状态恢复。

---

#### 3.2.10 permanent_delete

永久删除项目（仅 data_status = DELETED 时可执行）。

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| id | number | 是 | 项目 ID |
| confirm | boolean | 是 | 必须为 true 才执行删除 |
| delete_files | boolean | 否 | 是否删除本地文件（默认 false） |

**返回值**：void

**错误码**：
- `NOT_SOFT_DELETED`：项目未软删除，不可永久删除
- `CONFIRM_REQUIRED`：未确认删除

---

#### 3.2.11 batch_archive

批量归档项目。

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| ids | number[] | 是 | 项目 ID 列表 |

**返回值**：{ success: number, failed: number }

---

#### 3.2.12 batch_restore

批量恢复项目（从 ARCHIVED 或 DELETED 恢复为 ACTIVE）。

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| ids | number[] | 是 | 项目 ID 列表 |

**返回值**：{ success: number, failed: number }

---

#### 3.2.13 batch_soft_delete

批量软删除项目。

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| ids | number[] | 是 | 项目 ID 列表 |

**返回值**：{ success: number, failed: number }

---

#### 3.2.14 batch_move

批量移动项目到指定分类。

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| ids | number[] | 是 | 项目 ID 列表 |
| category_id | number | 是 | 目标分类 ID |

**返回值**：{ success: number, failed: number }

---

#### 3.2.15 batch_export

批量导出项目为 JSON/CSV。

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| ids | number[] | 是 | 项目 ID 列表 |
| format | 'json' \| 'csv' | 是 | 导出格式 |

**返回值**：string（文件内容）

---

#### 3.2.16 get_archived_projects

获取归档项目列表。

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| page | number | 否 | 页码，默认 1 |
| page_size | number | 否 | 每页数量，默认 20 |

**返回值**：{ items: Project[], total: number }

---

#### 3.2.17 get_deleted_projects

获取已删除项目列表（可恢复）。

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| page | number | 否 | 页码，默认 1 |
| page_size | number | 否 | 每页数量，默认 20 |

**返回值**：{ items: Project[], total: number }

---

### 3.4 标签管理接口

#### 3.4.1 get_tags

获取所有标签。

**返回值**：Tag[]

---

#### 3.4.2 create_tag

创建或获取标签（如果已存在则返回现有标签）。

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| name | string | 是 | 标签名称 |
| color | string | 否 | 标签颜色 |
| source | 'ai' \| 'user' \| 'preset' | 是 | 来源 |

**返回值**：Tag

---

#### 3.4.3 add_project_tag

为项目添加标签。

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| project_id | number | 是 | 项目 ID |
| tag_id | number | 是 | 标签 ID |

**返回值**：void

**错误码**：
- `PROJECT_NOT_FOUND`：项目不存在
- `TAG_NOT_FOUND`：标签不存在

---

#### 3.4.4 remove_project_tag

移除项目标签。

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| project_id | number | 是 | 项目 ID |
| tag_id | number | 是 | 标签 ID |

**返回值**：void

---

#### 3.4.5 get_project_tags

获取项目的所有标签。

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| project_id | number | 是 | 项目 ID |

**返回值**：Tag[]

---

#### 3.4.6 delete_tag

删除标签（同时移除所有项目关联）。

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| id | number | 是 | 标签 ID |

**返回值**：void

---

## 3.3 仓库管理接口

#### 3.3.1 create_vault

创建新仓库。

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| name | string | 是 | 仓库名称 |
| path | string | 是 | 存放目录（不含仓库文件夹名） |

**返回值**：Vault

**说明**：
1. 在 `path` 下创建以 `name` 为名的子目录
2. 在子目录中创建 `os_compass.db` 数据库文件
3. 执行初始化脚本创建所有表结构
4. 创建 `config.json` 仓库元数据文件
5. 返回创建成功的仓库信息

**错误码**：
- `PATH_NOT_FOUND`：目录不存在
- `VAULT_EXISTS`：同名仓库已存在
- `DB_INIT_FAILED`：数据库初始化失败

---

#### 3.3.2 list_vaults

获取仓库列表。

**参数**：无

**返回值**：Vault[]

**说明**：从 `vault-index.json` 读取所有仓库信息，包含每个仓库的项目数量。

---

#### 3.3.3 open_vault

切换当前仓库。

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| path | string | 是 | 仓库路径 |

**返回值**：Vault

**说明**：
1. 验证仓库有效性（文件存在、可打开、表结构完整）
2. 更新 `last-vault.txt` 记录当前仓库
3. 更新仓库的 `last_opened` 时间
4. 返回仓库信息

**错误码**：
- `VAULT_NOT_FOUND`：仓库不存在
- `VAULT_INVALID`：仓库无效（文件损坏或表结构不完整）

---

#### 3.3.4 delete_vault

删除仓库。

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| path | string | 是 | 仓库路径 |
| permanent | boolean | 是 | 是否彻底删除（true=删除数据库文件，false=仅从索引移除） |

**返回值**：void

**说明**：
- `permanent = false`：仅从 `vault-index.json` 移除记录，数据库文件保留
- `permanent = true`：删除数据库文件和 config.json，从索引移除记录

**错误码**：
- `VAULT_NOT_FOUND`：仓库不存在
- `CURRENT_VAULT`：不能删除当前打开的仓库

---

#### 3.3.5 validate_vault

验证仓库有效性。

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| path | string | 是 | 仓库路径 |

**返回值**：VaultValidationResult

```typescript
interface VaultValidationResult {
  valid: boolean;
  error?: string;
  table_count?: number;
  project_count?: number;
}
```

**说明**：检查数据库文件存在、可打开、表结构完整。

---

#### 3.3.6 import_vault

导入已有仓库（用户手上有完整仓库文件但不在清单里）。

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| name | string | 是 | 仓库名称（默认取目录名，可手动修改） |
| path | string | 是 | 仓库目录路径（必须含 os_compass.db 和 .cryptokey） |

**返回值**：Vault

**说明**：
1. 调用 `check_vault_integrity` 做完整齐全+有效性校验（目录存在、db 存在、cryptokey 存在、db 能打开、12 张必需表全在、关键列存在、密钥匹配验证）
2. 加载 `vault-index.json`
3. 路径重复检查（路径已在清单中 → 拒绝）
4. 名称重复检查（名称已在清单中 → 拒绝）
5. 追加 `{name, path}` 到 `vaults` 数组
6. 持久化 `vault-index.json`
7. 返回仓库信息（含 project_count）
8. **不切换**当前仓库，**不修改**目标目录里的任何文件

**校验失败错误信息**（按检查顺序，任一失败立即返回）：
- "目录不存在"
- "未找到 os_compass.db，不是有效的仓库目录"
- "未找到 .cryptokey 加密密钥文件，无法解密敏感数据"
- "数据库文件损坏：{错误详情}"
- "数据库结构不完整，缺少表：{表名}。可能不是 OS-Compass 仓库或为旧版本"
- "数据库版本过旧，不支持导入"
- ".cryptokey 文件格式无效，无法解密敏感数据"
- ".cryptokey 与数据库加密数据不匹配，请确认密钥文件来自同一仓库"
- "该仓库已在清单中，无需重复导入"
- "仓库名 '{名称}' 已存在，请修改名称"

**密钥匹配验证逻辑**（仅当 db 中有 `is_secret=1 AND value != ''` 的记录时执行）：
1. 查询 `system_variables WHERE is_secret=1 AND value != '' LIMIT 1`，无记录则查 `app_settings WHERE is_secret=1 AND value != '' LIMIT 1`
2. 两表都无记录 → 跳过密钥匹配验证
3. 有记录 → 读 `.cryptokey`，调 `key_from_base64`，构造 `CryptoManager`，对记录的 `value` 调 `decrypt`
   - 解密成功 → 通过
   - 解密失败 → 拒绝导入（密钥不匹配）

**错误码**：
- `PATH_NOT_FOUND`：目录不存在
- `VAULT_PATH_DUPLICATE`：路径已在清单中
- `VAULT_NAME_DUPLICATE`：名称已在清单中
- `VAULT_CRYPTOKEY_MISSING`：缺少 .cryptokey 文件
- `VAULT_CRYPTOKEY_MISMATCH`：密钥与加密数据不匹配
- `VAULT_SCHEMA_OUTDATED`：数据库版本过旧
- `VAULT_TABLES_MISSING`：缺少必需表

---

#### 3.3.7 get_current_vault

获取当前仓库信息。

**参数**：无

**返回值**：Vault | null

**说明**：从 `last-vault.txt` 读取当前仓库路径，返回仓库信息。

---

### 3.5 分类管理接口

#### 3.5.1 get_categories

获取所有分类（树形结构）。

**返回值**：Category[]（包含嵌套的 children）

---

#### 3.5.2 create_category

创建分类。

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| name | string | 是 | 分类名称 |
| explain | string | 否 | 分类描述 |
| parent_id | number | 否 | 上级分类 ID |

**返回值**：Category

**错误码**：
- `INVALID_NAME`：分类名称无效
- `DUPLICATE_NAME`：同级分类名称重复

---

#### 3.5.3 update_category

更新分类。

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| id | number | 是 | 分类 ID |
| name | string | 否 | 分类名称 |
| explain | string | 否 | 分类描述 |
| parent_id | number | 否 | 上级分类 ID |

**返回值**：Category

---

#### 3.5.4 delete_category

删除分类。

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| id | number | 是 | 分类 ID |

**返回值**：void

**错误码**：
- `SYSTEM_CATEGORY`：系统分类不可删除
- `HAS_CHILDREN`：存在子分类

---

### 3.4 AI 服务接口

#### 3.4.1 analyze_project

分析项目（重新生成 AI 报告）。AI 分析输入的 README 会经过预处理（过滤图片、按场景截取）。

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| id | number | 是 | 项目 ID |

**返回值**：AIReport（字段 useCases 使用驼峰命名，通过 serde rename 映射 Rust 的 use_cases）

**错误码**：
- `LLM_NOT_CONFIGURED`：未配置 LLM
- `LLM_API_ERROR`：LLM 调用失败

---

#### 3.4.2 translate

翻译文本（自动选择引擎：Google 优先，失败降级 LLM）。用于 README 翻译。

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| text | string | 是 | 待翻译文本 |
| target_lang | string | 是 | 目标语言 |

**返回值**：string（翻译后的文本）

---

#### 3.4.2a translate_with_llm

翻译文本（强制使用 LLM 引擎）。用于概览翻译（summary/use_cases/risks/dependencies/description）。

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| text | string | 是 | 待翻译文本 |
| target_lang | string | 是 | 目标语言 |

**返回值**：string（翻译后的文本）

---

#### 3.4.3 translate_readme

翻译 README。

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| id | number | 是 | 项目 ID |

**返回值**：string（翻译后的 README）

---

#### 3.4.3 regenerate_runbook

重新生成 Runbook。

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| id | number | 是 | 项目 ID |

**返回值**：Runbook

---

### 3.5 统计接口

#### 3.5.1 get_statistics

获取统计数据。

**返回值**：Statistics

```typescript
interface Statistics {
  // 健康度分布
  health_distribution: {
    excellent: number;  // 80-100
    good: number;        // 60-79
    fair: number;        // 40-59
    poor: number;        // 0-39
  };
  // 分类分布
  category_distribution: Array<{
    category_id: number;
    category_name: string;
    count: number;
  }>;
  // 语言分布
  language_distribution: Array<{
    language: string;
    count: number;
  }>;
  // 状态分布
  status_distribution: {
    to_explore: number;
    diving: number;
    in_use: number;
    abandoned: number;
  };
  // 总计
  total_projects: number;
  total_archived: number;
}
```

---

### 3.6 下载服务接口

#### 3.5.1 download_project

下载项目到本地。

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| id | number | 是 | 项目 ID |

**返回值**：string（本地路径）

**错误码**：
- `DOWNLOAD_DIR_EXISTS`：目录已存在
- `GIT_NOT_FOUND`：未安装 git
- `DOWNLOAD_FAILED`：下载失败

---

#### 3.5.2 open_in_explorer

用文件管理器打开目录。

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| path | string | 是 | 目录路径 |

**返回值**：void

---

#### 3.5.3 open_in_editor

用编辑器打开目录。

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| path | string | 是 | 目录路径 |
| editor | string | 否 | 编辑器命令，默认 "code" |

**返回值**：void

---

### 3.6 笔记接口

#### 3.6.1 get_notes

获取项目笔记。

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| project_id | number | 是 | 项目 ID |

**返回值**：ProjectNote[]

---

#### 3.6.2 create_note

创建笔记。

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| project_id | number | 是 | 项目 ID |
| content | string | 是 | 笔记内容 |

**返回值**：ProjectNote

---

#### 3.6.3 update_note

更新笔记。

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| id | number | 是 | 笔记 ID |
| content | string | 是 | 笔记内容 |

**返回值**：ProjectNote

---

#### 3.6.4 delete_note

删除笔记。

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| id | number | 是 | 笔记 ID |

**返回值**：void

---

### 3.7 用户信息接口（数据保险箱）

#### 3.7.1 get_user_info

获取用户信息列表。

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| project_id | number | 是 | 项目 ID |

**返回值**：ProjectUserInfo[]（敏感字段返回解密后的明文）

---

#### 3.7.2 create_user_info

创建用户信息。

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| project_id | number | 是 | 项目 ID |
| info_key | string | 是 | 键名 |
| info_value | string | 是 | 值 |
| is_secret | boolean | 否 | 是否敏感 |

**返回值**：ProjectUserInfo

---

#### 3.7.3 update_user_info

更新用户信息。

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| id | number | 是 | 信息 ID |
| info_key | string | 否 | 键名 |
| info_value | string | 否 | 值 |
| is_secret | boolean | 否 | 是否敏感 |

**返回值**：ProjectUserInfo

---

#### 3.7.4 delete_user_info

删除用户信息。

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| id | number | 是 | 信息 ID |

**返回值**：void

---

### 3.8 设置接口

#### 3.8.1 get_settings

获取系统设置。

**返回值**：Settings

---

#### 3.8.2 update_settings

更新系统设置。

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| settings | Settings | 是 | 完整设置对象 |

**返回值**：void

**说明**：敏感字段（llm_api_key、proxy_password）后端会自动加密存储。

---

#### 3.8.3 reset_settings

重置设置为默认值。

**返回值**：Settings

---

#### 3.8.4 test_llm_connection

测试 LLM 连接。

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| api_base | string | 是 | API Base URL |
| api_key | string | 是 | API Key |
| model | string | 是 | 模型名称 |

**返回值**：boolean

---

#### 3.8.5 build_clone_command

构建克隆命令（不执行）。

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| git_url | string | 是 | Git 仓库地址 |
| project_name | string | 是 | 项目名称（用于构建目录名） |

**返回值**：string（完整的 git clone 命令）

**说明**：命令中包含代理参数（如果已配置）

---

#### 3.8.6 execute_clone

执行克隆操作。

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| project_id | number | 是 | 项目 ID |
| git_url | string | 是 | Git 仓库地址 |

**返回值**：string（克隆到的目录路径）

**说明**：
- 使用 project_clone 表中的 cloned_path 和代理配置
- 如果 cloned_path 未设置，返回错误
- 如果目录已存在，返回错误
- 克隆成功后更新 cloned_at 字段

---

#### 3.8.7 get_clone_settings

获取项目克隆配置（不存在时自动创建）。

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| project_id | number | 是 | 项目 ID |

**返回值**：ProjectClone

**说明**：
- 如果记录不存在，自动创建一条
- 新记录使用 Settings 中的 download_proxy_* 作为代理默认值

---

#### 3.8.8 save_clone_settings

保存项目克隆配置。

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| clone | ProjectClone | 是 | 克隆配置对象 |

**返回值**：void

---

### 3.9 扩展配置接口

#### 3.9.1 get_plugin_config

获取扩展配置。

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| plugin_id | string | 是 | 扩展 ID |

**返回值**：Record<string, string>

---

#### 3.9.2 update_plugin_config

更新扩展配置。

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| plugin_id | string | 是 | 扩展 ID |
| config | Record<string, string> | 是 | 配置对象 |
| is_secret | Record<string, boolean> | 否 | 敏感字段标记 |

**返回值**：void

---

#### 3.9.3 get_system_variables

获取系统变量列表。

**返回值**：SystemVariable[]

```typescript
interface SystemVariable {
  key: string;
  value: string;
  isSecret: boolean;
}
```

---

#### 3.9.4 set_system_variable

设置系统变量。

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| key | string | 是 | 变量键名 |
| value | string | 是 | 变量值 |
| isSecret | boolean | 是 | 是否敏感 |

**返回值**：void

---

#### 3.9.5 delete_system_variable

删除系统变量。

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| key | string | 是 | 变量键名 |

**返回值**：void

---

#### 3.9.6 list_extensions

获取扩展列表。

**返回值**：Extension[]

```typescript
interface Extension {
  id: string;
  name: string;
  pluginClass: string;
  description: string;
  enabled: boolean;
  version: string;
  requiredVariables: VariableDef[];
}
```

---

### 3.10 国际化接口

#### 3.10.1 get_locale

获取当前语言设置。

**返回值**：string（如 "zh-CN"、"en"）

---

#### 3.10.2 set_locale

设置语言。

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| locale | string | 是 | 语言代码 |

**返回值**：void

---

#### 3.10.3 get_translations

获取翻译资源。

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| locale | string | 是 | 语言代码 |

**返回值**：Record<string, string>

---

### 3.11 爬虫接口

#### 3.11.1 test_crawler

测试爬虫连接。

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| url | string | 是 | 测试 URL |

**返回值**：{ success: boolean, markdown?: string, error?: string }

---

#### 3.11.2 fetch_as_markdown

将 URL 转换为 Markdown。

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| url | string | 是 | 目标 URL |

**返回值**：string（Markdown 内容）

---

### 3.12 Releases 接口

#### 3.12.1 数据结构

```typescript
interface Release {
  id: string;
  tag_name: string;
  published_at: string;
  body: string;
  body_zh: string | null;
  download_urls: DownloadUrl[];
  source: string;  // github/gitlab/gitee
}

interface DownloadUrl {
  name: string;
  url: string;
}
```

#### 3.12.2 fetch_releases

抓取并保存 Releases。

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| project_id | string | 是 | 项目 ID |

**返回值**：Release[]

**错误码**：
- `NETWORK_FAILED`：网络请求失败
- `API_RATE_LIMITED`：API 限流
- `PLATFORM_NOT_SUPPORTED`：平台不支持

---

#### 3.12.3 get_releases

获取本地缓存的 Releases。

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| project_id | string | 是 | 项目 ID |

**返回值**：Release[]

---

#### 3.12.4 translate_release_body

翻译 Release 说明。

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| project_id | string | 是 | 项目 ID |
| release_id | string | 是 | Release ID |
| text | string | 是 | 待翻译文本 |

**返回值**：string（翻译后的文本）

---

### 3.13 AI 智能分类接口

#### 3.13.1 数据结构

```typescript
interface ClassifyResult {
  category_id: string;
  category_path: string;
  confidence: number;  // 0-100
}
```

#### 3.13.2 ai_classify_project

AI 智能分类项目。

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| project_id | string | 是 | 项目 ID |

**返回值**：ClassifyResult

**错误码**：
- `PROJECT_INFO_INSUFFICIENT`：项目信息不足，无法分类
- `NO_CATEGORIES`：系统中暂无分类
- `LLM_API_ERROR`：LLM 调用失败

---

## 3.14 插件管理接口（二期 M13）

> **二期功能标记**：以下所有接口均为二期（Phase 2）设计。

### 3.14.0 存储架构

插件系统采用三层操作域设计：

| 域 | 存储位置 | 说明 |
|----|---------|------|
| 插件域 | `${AppData}/.os-compass/plugins.db` | 插件元数据、启用状态、配置（系统级） |
| 仓库域 | 当前仓库 `os_compass.db` | LLM、代理、Token 等（运行时注入） |
| 业务域 | `{vault_dir}/plugin_*.db` | 插件业务数据（跟随仓库） |

### 3.14.1 list_feature_plugins

获取所有功能插件列表。

| 项目 | 说明 |
|------|------|
| 命令 | `list_feature_plugins` |
| 参数 | 无 |
| 返回 | `Vec<FeaturePluginInfo>` |

```typescript
interface FeaturePluginInfo {
  id: string;             // 插件 ID
  name: string;           // 插件名称
  plugin_type: string;    // 'radar' | 'search' | 'webhook' | 'importer'
  version: string;        // 插件版本
  enabled: boolean;       // 启用状态（来自 plugins.db）
  config: string | null;  // 配置 JSON（来自 plugins.db）
  db_mode: string;        // 'main' | 'global' | 'vault' | 'none'
  db_path_template: string | null; // 数据库路径模板
}
```

### 3.14.2 set_plugin_enabled

启用/禁用插件（动态启停，立即生效）。

| 项目 | 说明 |
|------|------|
| 命令 | `set_plugin_enabled` |
| 参数 | `{ pluginId: string, enabled: boolean }` |
| 返回 | `void` |
| 错误 | 插件不存在、插件初始化失败 |
| 存储 | 写入 `${AppData}/.os-compass/plugins.db` |

### 3.14.3 get_plugin_config / save_plugin_config

获取/保存插件配置。

| 项目 | 说明 |
|------|------|
| 命令 | `get_plugin_config` / `save_plugin_config` |
| 参数 | `{ pluginId: string }` / `{ pluginId: string, config: string }` |
| 返回 | `string | null`（配置 JSON）/ `void` |
| 存储 | 读写 `${AppData}/.os-compass/plugins.db` |

### 3.14.4 plugin_get_db_path

获取插件独立数据库文件路径。

| 项目 | 说明 |
|------|------|
| 命令 | `plugin_get_db_path` |
| 参数 | `{ pluginId: string }` |
| 返回 | `string`（.db 文件路径，如 `{vault_dir}/plugin_radar.db`） |
| 说明 | 插件自行决定是否使用独立 .db。调用此命令获取路径后，插件自行打开连接。 |

### 3.14.5 PluginContext（运行时注入）

插件执行时，系统注入的上下文信息：

```rust
pub struct PluginContext {
    pub vault_dir: PathBuf,        // 当前仓库目录
    pub app_data_dir: PathBuf,     // 应用数据目录
    pub config: Value,             // 插件配置（来自 plugins.db）
    pub llm_config: Value,        // LLM 配置（来自当前仓库）
    pub plugin_db_path: Option<PathBuf>, // 插件数据库路径
}
```

> **注意**：LLM 配置、代理配置等运行时依赖来自当前仓库，插件域的配置仅包含插件自身的设置参数。

---

## 3.15 意图搜索接口（二期 M13/M22）

> **二期功能标记**：以下所有接口均为二期（Phase 2）设计。
> **更新日期**：2026-08-14
> **更新说明**：
> - M13：基础意图搜索
> - M22：**搜索历史记录功能（搜索历史保存与加载）**

### 3.15.1 intent_search

执行意图搜索（双轨搜索）。

| 项目 | 说明 |
|------|------|
| 命令 | `intent_search` |
| 参数 | `{ query: string, conversationId?: string }` |
| 返回 | `SearchResult` |

```typescript
interface SearchResult {
  query: string;              // 原始查询
  local_results: ProjectMatch[];   // 第一轨：本地向量搜索结果
  web_results: ProjectMatch[];    // 第二轨：LLM 联网搜索结果
  total: number;              // 总结果数
  conversation_id: string;   // 会话 ID（多轮对话）
}

interface ProjectMatch {
  project_name: string;
  project_url: string;
  description: string;
  language: string | null;
  stars: number | null;
  health_score: number | null;
  match_score: number;        // 匹配度 0-100
  source: 'local' | 'llm';     // 结果来源
  source_detail?: string;       // 来源详情
}
```

### 3.15.2 get_search_history（M22）

> **M22 功能**：获取搜索历史记录。

| 项目 | 说明 |
|------|------|
| 命令 | `get_search_history` |
| 参数 | `{ limit?: number }`（默认 20） |
| 返回 | `Vec<SearchHistoryItem>` |

```typescript
interface SearchHistoryItem {
  id: number;
  query: string;
  result_count: number;
  created_at: string;
}
```

### 3.15.3 clear_search_history（M22）

> **M22 功能**：清空搜索历史记录。

| 项目 | 说明 |
|------|------|
| 命令 | `clear_search_history` |
| 参数 | 无 |
| 返回 | `void` |

### 3.15.4 搜索信息源管理

| 命令 | 参数 | 返回 | 说明 |
|------|------|------|------|
| `list_search_sources` | 无 | `Vec<SearchSource>` | 列出所有搜索信息源 |
| `add_search_source` | `{ name, sourceType, url, platform? }` | `SearchSource` | 添加信息源 |
| `update_search_source` | `{ id, name?, url?, enabled? }` | `void` | 更新信息源 |
| `delete_search_source` | `{ id }` | `void` | 删除信息源 |
| `rebuild_embeddings` | `{ projectId?: number }` | `number`（重建向量数） | 重建项目向量索引 |

```typescript
interface SearchSource {
  id: number;
  name: string;
  source_type: 'rss' | 'web_crawl' | 'telegram' | 'platform_preset';
  url: string;
  platform: string | null;
  enabled: boolean;
  created_at: string;
}
```

### 3.15.5 import_search_result

将搜索结果一键导入到本地项目库。

| 项目 | 说明 |
|------|------|
| 命令 | `import_search_result` |
| 参数 | `{ projectUrl: string, projectName: string, categoryId?: number }` |
| 返回 | `{ projectId: number }` |
| 说明 | 内部调用现有的 `import_project` 流程 |

---

## 3.16 情报雷达接口（二期 M13）

> **二期功能标记**：以下所有接口均为二期（Phase 2）设计。

### 3.16.1 雷达信息源管理

| 命令 | 参数 | 返回 | 说明 |
|------|------|------|------|
| `list_radar_sources` | 无 | `Vec<RadarSource>` | 列出所有雷达信息源 |
| `add_radar_source` | `{ name, sourceType, url, platform?, checkInterval? }` | `RadarSource` | 添加信息源 |
| `update_radar_source` | `{ id, name?, url?, enabled?, checkInterval? }` | `void` | 更新信息源 |
| `delete_radar_source` | `{ id }` | `void` | 删除信息源 |

```typescript
interface RadarSource {
  id: number;
  name: string;
  source_type: 'rss' | 'web_crawl' | 'telegram' | 'platform_preset';
  url: string;
  platform: string | null;
  enabled: boolean;
  check_interval: number;  // 秒
  last_checked_at: string | null;
  last_status: 'success' | 'error' | null;
  last_error: string | null;
}
```

### 3.16.2 get_radar_items

获取收件箱条目（支持搜索增强）。

| 项目 | 说明 |
|------|------|
| 命令 | `get_radar_items` |
| 参数 | `{ status?: string, limit?: number, keyword?: string, sourceIds?: number[], startDate?: string, endDate?: string, page?: number, pageSize?: number }` |
| 返回 | `PaginatedRadarItems` |

**参数说明**：

| 参数 | 类型 | 说明 |
|------|------|------|
| status | string | 状态筛选（默认 'unread'） |
| limit | number | 限制数量（兼容旧版） |
| keyword | string | 关键字搜索（模糊匹配 project_name、description） |
| sourceIds | number[] | 信息源 ID 列表（多选） |
| startDate | string | 开始日期（YYYY-MM-DD） |
| endDate | string | 结束日期（YYYY-MM-DD） |
| page | number | 页码（从 1 开始） |
| pageSize | number | 每页数量（默认 20） |

**返回结构**：

```typescript
interface PaginatedRadarItems {
  items: RadarItem[];  // 数据列表
  total: number;        // 总数
  page: number;         // 当前页
  pageSize: number;     // 每页数量
  totalPages: number;   // 总页数
}

interface RadarItem {
  id: number;
  source_id: number;
  project_name: string | null;
  project_url: string | null;
  description: string | null;
  language: string | null;
  source_urls: string | null;  // JSON 数组
  status: 'unread' | 'imported' | 'blacklisted' | 'ignored';
  published_at: string | null;
  fetched_at: string;
}
```

### 3.16.3 radar_item_action

对收件箱条目执行操作（导入/不感兴趣/忽略）。

| 项目 | 说明 |
|------|------|
| 命令 | `radar_item_action` |
| 参数 | `{ itemId: number, action: 'import' | 'blacklist' | 'ignore', categoryId?: number }` |
| 返回 | `{ projectId?: number }`（action='import' 时返回导入的项目 ID） |
| 说明 | import 内部调用 `import_project`；blacklist 加入 `radar_blacklist` 表 |

### 3.16.4 trigger_radar_scan

手动触发雷达扫描（不等定时任务）。

| 项目 | 说明 |
|------|------|
| 命令 | `trigger_radar_scan` |
| 参数 | `{ sourceId?: number }`（不传则扫描所有启用的源） |
| 返回 | `{ scanned: number, newItems: number, errors: number }` |

### 3.16.5 get_radar_unread_count

获取未读条目数量（用于 Sidebar 徽标）。

| 项目 | 说明 |
|------|------|
| 命令 | `get_radar_unread_count` |
| 参数 | 无 |
| 返回 | `number` |

### 3.16.6 clear_radar_cache

清理雷达缓存（手动清理或 TTL 过期清理）。

| 项目 | 说明 |
|------|------|
| 命令 | `clear_radar_cache` |
| 参数 | `{ beforeDays?: number }`（清理 N 天前的数据，不传则清理所有已处理条目） |
| 返回 | `number`（清理条目数） |

---

### 4.1 错误码定义

| 错误码 | 说明 | 用户提示 |
|--------|------|----------|
| INVALID_URL | URL 格式无效 | "URL 格式不正确" |
| PLUGIN_NOT_FOUND | 不支持的平台 | "暂不支持该平台" |
| DUPLICATE_PROJECT | 项目已存在 | "该项目已存在于列表中" |
| NETWORK_FAILED | 网络请求失败 | "无法连接，请检查网络或代理" |
| NETWORK_TIMEOUT | 网络超时 | "网络超时，请重试" |
| LLM_NOT_CONFIGURED | 未配置 LLM | "请先配置 LLM" |
| LLM_API_ERROR | LLM 调用失败 | "AI 分析失败，请重试" |
| LLM_PARSE_ERROR | LLM 响应格式错误 | "AI 响应解析失败" |
| DOWNLOAD_DIR_EXISTS | 目录已存在 | "该目录已存在" |
| GIT_NOT_FOUND | 未安装 git | "未检测到 git，请先安装" |
| EDITOR_NOT_FOUND | 编辑器未找到 | "未找到编辑器，请先安装" |
| SYSTEM_CATEGORY | 系统分类不可操作 | "系统分类不可删除/修改" |
| HAS_CHILDREN | 存在子分类 | "请先删除子分类" |
| INVALID_NAME | 名称无效 | "名称不能为空" |
| DUPLICATE_NAME | 名称重复 | "同级分类名称不能重复" |
| DB_ERROR | 数据库错误 | "数据保存失败" |
| NOT_SOFT_DELETED | 项目未软删除 | "请先软删除项目后再永久删除" |
| CONFIRM_REQUIRED | 确认删除必填 | "请确认删除操作" |
| API_RATE_LIMITED | API 限流 | "请求过于频繁，请稍后重试" |
| PLATFORM_NOT_SUPPORTED | 平台不支持 | "该平台暂不支持 Releases" |
| PROJECT_INFO_INSUFFICIENT | 项目信息不足 | "项目信息不足，无法分类" |
| NO_CATEGORIES | 无可用分类 | "系统中暂无分类，请先创建分类" |
| PLUGIN_NOT_REGISTERED | 插件未注册 | "插件未注册" |
| PLUGIN_NOT_ENABLED | 插件未启用 | "请先启用该插件" |
| PLUGIN_INIT_FAILED | 插件初始化失败 | "插件初始化失败，请检查配置" |
| SOURCE_NOT_FOUND | 信息源不存在 | "信息源不存在" |
| SOURCE_FETCH_FAILED | 信息源拉取失败 | "信息源拉取失败，请检查 URL 或网络" |
| SOURCE_PARSE_FAILED | 内容解析失败 | "内容解析失败" |
| SEARCH_STRATEGY_NOT_CONFIGURED | 联网搜索策略未配置 | "请先配置联网搜索策略" |
| SEARCH_NO_RESULTS | 无搜索结果 | "未找到匹配的项目" |

### 4.2 前端错误处理示例

```typescript
import { invoke } from '@tauri-apps/api/core';
import { useToast } from '@/hooks/useToast';

async function handleImport(url: string) {
  try {
    const project = await invoke<Project>('import_project', {
      url,
      autoAnalyze: true,
      autoTranslate: true,
    });
    toast.success('导入成功');
    return project;
  } catch (error) {
    const errorCode = parseErrorCode(error);
    switch (errorCode) {
      case 'INVALID_URL':
        toast.error('URL 格式不正确');
        break;
      case 'PLUGIN_NOT_FOUND':
        toast.error('暂不支持该平台');
        break;
      case 'DUPLICATE_PROJECT':
        toast.info('该项目已存在');
        break;
      case 'NETWORK_FAILED':
        toast.error('网络连接失败，请检查代理设置');
        break;
      default:
        toast.error('导入失败');
    }
    throw error;
  }
}

function parseErrorCode(error: unknown): string {
  if (typeof error === 'string') {
    try {
      const parsed = JSON.parse(error);
      return parsed.code || 'UNKNOWN';
    } catch {
      return 'UNKNOWN';
    }
  }
  return 'UNKNOWN';
}
```

---

## 5 接口变更记录

| 版本 | 日期 | 变更说明 |
|------|------|----------|
| V1.0 | 2026-06-27 | 初始版本 |
| V1.1 | 2026-06-29 | 新增批量操作、归档接口、统计接口、国际化接口、爬虫接口 |
| V1.2 | 2026-06-29 | 重构项目状态设计：lifecycle_status + data_status 双维度；新增 soft_delete_project、archive_project、restore_project；delete_project 改为 soft_delete；新增 batch_soft_delete、get_deleted_projects；Project 结构更新 |
| V1.3 | 2026-06-29 | batch_unarchive 改名为 batch_restore，统一恢复语义 |
| V1.4 | 2026-07-07 | 新增 Releases 接口（fetch_releases、get_releases、translate_release_body）；新增 AI 智能分类接口（ai_classify_project） |
| V1.5 | 2026-07-08 | 新增仓库管理接口（create_vault、list_vaults、open_vault、delete_vault、validate_vault） |
| V1.6 | 2026-07-10 | 新增导入已有仓库接口（import_vault），含完整校验链和密钥匹配验证 |
| V1.7 | 2026-07-11 | 新增二期 M13 接口：插件管理（list_feature_plugins、set_plugin_enabled、get/save_plugin_config、plugin_get_db_path）；意图搜索（intent_search、get/clear_search_history、搜索信息源管理、import_search_result）；情报雷达（雷达信息源管理、get_radar_items、radar_item_action、trigger_radar_scan、get_radar_unread_count、clear_radar_cache） |
| V1.8 | 2026-07-27 | 意图搜索改为双轨搜索：refresh_search_cache 改为 rebuild_embeddings |
| V1.9 | 2026-08-14 | 新增 M19 接口：广告屏蔽（ad_patterns CRUD）、定时采集（radar_schedule）、平台扩展（platform_packages）

---

## 3.17 M19 广告屏蔽接口

> **M19 广告学习功能**：用户标记广告 → LLM 判定 → 屏蔽列表

### 3.17.1 广告屏蔽列表管理

| 命令 | 参数 | 返回 | 说明 |
|------|------|------|------|
| `mark_as_ad` | `{ itemId, title, summary, sourceUrl }` | `AdMarkResult` | 标记广告（核心接口） |
| `list_ad_patterns` | `{ type?, enabled? }` | `Vec<AdPattern>` | 列出屏蔽规则 |
| `add_ad_pattern` | `{ type, value, confidence? }` | `AdPattern` | 添加屏蔽规则 |
| `update_ad_pattern` | `{ id, enabled?, confidence? }` | `void` | 更新规则 |
| `delete_ad_pattern` | `{ id }` | `void` | 删除规则 |
| `check_ad_pattern` | `{ title, summary, sourceUrl }` | `AdCheckResult` | 检查是否匹配 |

```typescript
interface AdMarkResult {
  success: boolean;
  patternId?: number;
  llmJudgment: {
    isAd: boolean;
    confidence: number;
    adKeywords: string[];
    reasoning: string;
  };
  message: string;
}

interface AdPattern {
  id: number;
  pattern_type: 'keyword' | 'domain' | 'regex';
  pattern_value: string;
  confidence: number;
  source: 'manual' | 'llm_learned';
  hit_count: number;
  enabled: boolean;
  created_at: string;
}

interface AdCheckResult {
  isBlocked: boolean;
  matchedPatterns: Array<{
    id: number;
    type: string;
    value: string;
    confidence: number;
  }>;
}
```

### 3.17.2 调用示例

```typescript
import { invoke } from '@tauri-apps/api/core';

// 标记广告（核心功能）
const result = await invoke<AdMarkResult>('mark_as_ad', {
  itemId: 'item_123',
  title: '限时优惠！立即购买',
  summary: '全网最低价，错过不再有...',
  sourceUrl: 'https://example.com/promo'
});
// 返回：{ success: true, patternId: 5, llmJudgment: {...}, message: '已添加到屏蔽列表' }

// 列出所有屏蔽规则
const patterns = await invoke<AdPattern[]>('list_ad_patterns', { type: 'keyword' });

// 添加手动规则
await invoke<AdPattern>('add_ad_pattern', {
  type: 'keyword',
  value: '低价促销',
  confidence: 70,
  source: 'manual'
});

// 检查内容是否匹配
const checkResult = await invoke<AdCheckResult>('check_ad_pattern', {
  title: '限时优惠来袭',
  summary: '全场五折...',
  sourceUrl: 'https://example.com'
});
// 返回：{ isBlocked: true, matchedPatterns: [{ id: 5, type: 'keyword', value: '低价促销', confidence: 70 }] }

// 更新规则置信度
await invoke('update_ad_pattern', { id: 5, confidence: 85 });

// 删除规则
await invoke('delete_ad_pattern', { id: 5 });

// 标记误判加入白名单
await invoke('add_ad_whitelist', { url: 'https://example.com/legit-article' });
```

### 3.17.2 广告白名单

| 命令 | 参数 | 返回 | 说明 |
|------|------|------|------|
| `add_ad_whitelist` | `{ url }` | `void` | 添加到白名单 |
| `remove_ad_whitelist` | `{ url }` | `void` | 从白名单移除 |

---

## 3.18 M19 定时采集接口

> **M19 定时采集功能**：情报雷达定时自动采集，有新内容时通知用户
>
> **原型文件**：`design-system/public/radar-inbox.html`

### 3.18.1 定时任务管理

| 命令 | 参数 | 返回 | 说明 |
|------|------|------|------|
| `get_radar_schedule` | 无 | `RadarSchedule` | 获取定时配置 |
| `update_radar_schedule` | `RadarScheduleUpdate` | `void` | 更新定时配置 |
| `trigger_radar_scan_now` | 无 | `number` | 手动触发立即采集 |

**采集模式**：
- `interval`（间隔模式）：固定时间间隔（5min/15min/30min/1h/2h/6h/12h/24h）
- `custom`（自定义模式）：秒/分/时自定义间隔

```typescript
interface RadarSchedule {
  enabled: boolean;
  mode: 'interval' | 'custom';
  interval_seconds: number;      // 间隔模式秒数
  custom_unit: 'second' | 'minute' | 'hour';  // 自定义单位
  custom_value: number;          // 自定义数值
  notification_enabled: boolean;
  system_notification: boolean;
  badge_notification: boolean;
  last_run_at: string | null;
  next_run_at: string | null;
  new_items_count: number;
}

interface RadarScheduleUpdate {
  enabled?: boolean;
  mode?: 'interval' | 'custom';
  interval_seconds?: number;
  custom_unit?: 'second' | 'minute' | 'hour';
  custom_value?: number;
  notification_enabled?: boolean;
  system_notification?: boolean;
  badge_notification?: boolean;
}
```

### 3.18.3 调用示例

```typescript
// 获取定时采集配置
const schedule = await invoke<RadarSchedule>('get_radar_schedule');

// 开启定时采集（间隔模式，每15分钟）
await invoke('update_radar_schedule', {
  enabled: true,
  mode: 'interval',
  intervalSeconds: 900
});

// 开启定时采集（自定义模式，每30秒）
await invoke('update_radar_schedule', {
  enabled: true,
  mode: 'custom',
  customUnit: 'second',
  customValue: 30,
  notificationEnabled: true,
  systemNotification: true,
  badgeNotification: true
});

// 手动触发立即采集
const newCount = await invoke<number>('trigger_radar_scan_now');

// 列出未读通知
const notifications = await invoke<Vec<Notification>>('list_notifications', { unreadOnly: true });

// 标记通知已读
await invoke('mark_notification_read', { id: notification.id });
```

### 3.18.2 通知管理

| 命令 | 参数 | 返回 | 说明 |
|------|------|------|------|
| `list_notifications` | `{ unreadOnly?: boolean }` | `Vec<Notification>` | 列出通知 |
| `mark_notification_read` | `{ id }` | `void` | 标记已读 |
| `clear_notifications` | 无 | `void` | 清除所有通知 |

```typescript
interface Notification {
  id: number;
  title: string;
  body: string | null;
  item_count: number;
  read: boolean;
  created_at: string;
}
```

---

## 3.19 M19 平台扩展接口

> **M19 平台扩展**：NPM/PyPI/Crates.io 项目导入

### 3.19.1 平台适配器

| 命令 | 参数 | 返回 | 说明 |
|------|------|------|------|
| `get_platforms` | 无 | `Vec<Platform>` | 列出支持平台 |
| `fetch_platform_package` | `{ url }` | `PlatformPackage` | 拉取平台包信息 |
| `import_platform_package` | `{ url, categoryId? }` | `Project` | 导入平台包到项目库 |

```typescript
interface Platform {
  id: string;
  name: string;
  icon: string;
}

interface PlatformPackage {
  name: string;
  version: string;
  description: string | null;
  homepage: string | null;
  repository_url: string | null;
  license: string | null;
  downloads: number | null;
  recent_downloads: number | null;
  health_score: number | null;
  health_grade: string | null;
  raw_data: object;
}
```

### 3.19.3 调用示例

```typescript
// 列出支持的平台
const platforms = await invoke<Vec<Platform>>('get_platforms');
// [{ id: 'npm', name: 'NPM', icon: 'npm' }, ...]

// 拉取 NPM 包信息（支持 URL 或包名）
const pkg = await invoke<PlatformPackage>('fetch_platform_package', {
  url: 'react'  // 或 'https://www.npmjs.com/package/react'
});
// {
//   name: 'react',
//   version: '18.2.0',
//   description: 'The library for web and native user interfaces.',
//   downloads: 15000000,
//   health_score: 85.5,
//   health_grade: 'Good',
//   ...
// }

// 获取健康度评分明细
const breakdown = await invoke<HealthBreakdown>('get_platform_health_breakdown', {
  platform: 'npm',
  packageName: 'react'
});
// {
//   total: 85.5,
//   grade: 'Good',
//   dimensions: [
//     { name: '下载量', score: 100, weight: 30, detail: '...' },
//     { name: '维护状态', score: 80, weight: 20, detail: '...' },
//     ...
//   ]
// }

// 导入到项目库
const project = await invoke<Project>('import_platform_package', {
  url: 'react',
  categoryId: 1  // 可选：指定分类
});
```

### 3.19.2 平台健康度

| 命令 | 参数 | 返回 | 说明 |
|------|------|------|------|
| `get_platform_health_breakdown` | `{ platform, packageName }` | `HealthBreakdown` | 获取评分明细 |

```typescript
interface HealthBreakdown {
  total: number;
  grade: 'Excellent' | 'Good' | 'Fair' | 'Poor' | 'Bad';
  dimensions: ScoreDimension[];
}

interface ScoreDimension {
  name: string;
  score: number;
  weight: number;
  detail: string;
}
```

---

**文档结束**
