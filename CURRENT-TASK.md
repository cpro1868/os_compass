# OS-Compass 当前任务

## 2026-09-07 17:26 - 意图搜索多轮对话记忆与智能收敛 ✅ 已完成

### 需求（用户验收反馈）
解决意图搜索在多轮对话中“无上下文记忆、追问废话、前言不搭后语、死循环追问”的问题。

### 实施
- 后端：
  - `search.rs` / `search_cmd.rs`：新增 `SearchContextMessage` 与 `analyze_intent_with_history`；
  - 重塑 System Prompt：承接前文领域，智能收敛（领域+形态齐备立即进入搜索），约束追问纪律，支持 markdown 代码块清洗。
- 前端：
  - `api/search.ts`：类型扩展支持 history；
  - `SearchView.tsx`：上下文抽取（最近 6 条问答），复合缓存键（`contextDigest#query`），澄清选项点击同步触发 `handleSubmit(val)`，消除异步丢词。
- 验证：
  - 新增 `SearchView.context-intent.test.tsx` 与 Rust 单元测试；
  - 11 文件 76 tests 全绿，typecheck 通过，MSI+NSIS 构建成功；
  - 产物归档至 `Previous/os-compass.exe`（SHA-256：`75C87DB5C062468E4B5123D784F9A6E7381005BA34AC1208368C126104E1F245`）。

---

## 2026-09-07 14:16 - "再推荐 5 个"直接返回大模型原文（无选项卡） ✅ 已完成

### 需求
点击“让大模型再推荐 5 个”后，在新聊天记录中直接展示大模型返回的自然语言原文，不使用项目选项卡。

### 实施
- 后端：`ask_llm_direct` 直接对话，prompt 要求自然语言列出名称/URL/用途/语言；返回 `RecommendMoreResult.raw_text`。
- 前端：`SearchView.tsx` 新消息直接渲染 `rawText`（whitespace-pre-wrap），无选项卡/卡片。
- 验证：75 tests / typecheck / tauri build 均通过；Previous/os-compass.exe SHA-256 `D35BEBD9AD185437D292907C6123FA556EAF3F2570F209185D05E2961D6E2311`。

---

## 2026-09-07 13:30 - "再推荐 5 个"改为新开聊天消息直接返回 ✅ 已完成

### 需求（用户验收反馈）
再推荐结果不再追加到当前搜索消息的选项卡列表，而是新开一条聊天记录，将大模型返回的信息直接展示。

### 实施（仅前端）
- `MessageItem.recommendResults?: ProjectMatch[]`；`askMoreForQuery` 新开 assistant 消息：loading 文案 → 返回后渲染标题+项目卡片（无折叠/无按钮）；0 条/失败分别显示提示文案。
- `ResultsList` 移除内联 loading/滚动/added 逻辑；i18n 新增 `search.recommendTitle`（zh/en）。

### 验证
- pnpm test 76 全绿（重写 recommend-feedback 4 用例 + 修正 cardIssues 断言）/ typecheck / tauri build MSI+NSIS 成功
- `Previous/os-compass.exe` SHA-256 `6B6C73E4C2C5E019A789BFCAEBF286803BD595C80BF47EC2AB38B5258EF78A4F`

---

## 2026-09-07 11:36 - 意图搜索相关性阈值 + "再推荐 5 个"反馈优化 ✅ 已完成

### 现象
1. 搜索"音频工具/地图工具"返回大量完全不相关项目（项目集、useSend 等）。
2. 点击"让大模型再推荐 5 个"后无任何信息返回。

### 根因（详见 `docs/修复计划_意图搜索相关性_2026-09-06.md` 与 `docs/工作进展记录.md` 2026-09-07 条目）
1. `semantic_search` 无相似度阈值，top-10 垃圾全收；且 ≥3 条结果堵死 LLM 兜底。
2. 向量诊断：45 个项目仅 20 个有向量（全部 1024 维，无维度混用），voice-pro/奇幻地图生成器等最相关项目未向量化。
3. 前端再推荐去重只比对 `web_results`（漏了本地结果）；全命中时无提示；等待期无内联 loading。

### 修复
- `embedding.rs`：`SEMANTIC_SEARCH_MIN_SCORE = 0.45` 阈值过滤 + 维度不一致跳过
- `SearchView.tsx`：去重扩展到本地+联网结果、0 新增 Toast、内联 loading、追加后展开+滚动定位
- `search_cmd.rs`：debug log 输出 URL 清单；zh/en 新增 2 个 i18n 键

### 验证
- pnpm test 75 全绿（新增 3 用例）/ typecheck / cargo check --lib --tests / tauri build MSI+NSIS 成功
- `Previous/os-compass.exe` SHA-256 `AFCBF92BC7F650DAF705CA93E38816CAC74EE804696477D65B290796488F5771`

### 用户侧一次性操作
首次运行新版本后，在看板页点击"向量化全部"，45 个项目全部向量化后搜索相关性完整生效。

---

## 2026-09-06 11:14 - 意图搜索质量回归与"再推荐 5 个"无响应修复

### 现象
用户验收：搜索"地图工具"命中 AI-Cubby、shotcut、OpenHarness 等毫无关联项目；"让大模型再推荐 5 个"点击后无任何返回。

### 根因
1. 仓库库 42 个项目里仅 20 个存有 embedding 向量；"奇幻地图生成器"等地图相关项目根本没被向量化。
2. `generate_project_embeddings` / `rebuild_embeddings` 仅以 `"{name}: {description}"` 生成 embedding 文本，丢失 `languages` 关键语义信号，bge-m3 无法定位真实相关项目。
3. `recommend_more_projects` 仅依赖 LLM 直推。LLM API 180s 超时或返回 0 条时，前端只显示"暂无更多推荐"，无兜底。

### 修复
- `embedding.rs` 新增 `build_project_embedding_text(name, description, languages)`；`generate_project_embeddings` 与 `rebuild_embeddings` 同步接入。
- `search_cmd.rs` `recommend_more_projects`：先调 `recommend_projects_with_llm`；为空 / 出错时降级走 `embedding::semantic_search` 在本地库找最相关项目并补足条目。
- `embedding_tests.rs` 新增 9 个 Rust 单元测试覆盖 `primary_language`、cosine、embedding 文本拼装、ProjectMatch 构造。

### 验证
- `cargo check --lib --tests`：通过
- `pnpm test`：71 tests passed
- `pnpm typecheck`：通过
- `pnpm tauri build`：MSI + NSIS 成功
- 端到端："地图工具"真实排序为 奇幻地图生成器 → MarkWrite → 真实住宅地址生成器 → AI-Cubby
- 产物：`Previous/os-compass.exe` SHA-256 `3222152F0F702B9BE93A240D3AD47893C3086F97E89981B3BEBFB1D898AF9581`

### 用户侧一次性操作
首次运行新版本后调用"重建全部向量"（看板页向量按钮），22 个新向量化项才能参与搜索。

---

## 2026-09-05 22:06 - 意图搜索三项缺陷修复（卡片精简/详情跳转/追加推荐）

### 需求
1. 项目信息卡片去除杂乱内容：不显示 URL，只展示名称、描述（2 行截断）、语言（单语言标签，从 DB 存的 JSON 数组如 `["C++","QML"]` 提取首项）；去除已入库/来源徽标/Star/健康分等杂音；`llm_text` 原样保留。
2. 点击卡片提示“打开项目详情失败”：本地项目（带 `project_id`）改为派发全局 `openProjectDetail` CustomEvent，复用主应用详情弹窗；非本地项目（LLM 推荐）用系统默认浏览器（`openUrl`）打开外部链接并自动补齐 `https://`。
3. 点击“让大模型再推荐 5 个”无反应：修复追加到 `web_results` 后因折叠阈值（5 条）导致新结果全被隐藏的问题；点击推荐并成功返回后自动展开全部，且追加后的结果立即可见；返回 0 条给出 Toast 提示。

### 根因
- 杂乱问题：Rust 侧 `ProjectMatch.language` 直接将 `projects.languages`（GitHub API 存下的 JSON 数组字面量）原样字符串塞给前端；前端卡片原版带有 source 徽标、已入库徽标、star、fork、health_score、url 导致信息密集。
- 打开详情失败：前端调用 `invoke('open_project_detail', { url })`，但全 Rust 侧根本不存在该命令（零定义）；主应用真正监听的是 `openProjectDetail` 事件（接收 `{ projectId }`）。
- 再推荐无反应：`ResultsList` 内部 state `expanded` 与父级消息流脱节；且本地已有≥5 条时，追加在末尾的 5 条 LLM 推荐被 `all.slice(0, 5)` 截断在外，用户视觉上“无任何反应”。

### 实施
- 后端：
  - `src-tauri/src/plugins/search.rs`：`ProjectMatch` 新增 `pub project_id: Option<i64>`；新增 `primary_language(raw)` 提取多语言 JSON 数组首项并回落单语言字面量；`three_layer_search` 注入 `project_id: Some(project_id)`。
  - `src-tauri/src/embedding.rs`：`semantic_search_projects` 同样调用 `primary_language` 并注入 `project_id: Some(project_id)`。
  - `src-tauri/src/commands/search_cmd.rs`：`recommend_more_projects` 补充 `project_id: None` 字段。
- 前端：
  - `src/api/search.ts`：`ProjectMatch` 新增 `project_id?: number`，`source` 扩展 `'llm_recommend'`。
  - `src/components/SearchView.tsx`：
    - `ProjectCard` 移到外层静态组件，仅渲染名称、描述（line-clamp-2）、语言（`normalizeLanguage` 兜底解析），去除 URL/徽标/已入库等杂质。
    - `handleProjectItemClick`：本地带 `project_id` 时派发 `openProjectDetail` 事件；非本地调用 `@tauri-apps/plugin-opener` 的 `openUrl`，自动补全协议。
    - `ResultsList` 受控 `expanded`（由 `SearchView.expandedMessages[msg.id]` 维护），点击“再推荐 5 个”成功后自动设置 `expanded=true`，确保追加结果立即展示在视口内；空结果 Toast 提示。
    - 最近项目点击同步调用 `handleProjectItemClick`。

### TDD 与全量验证
- 新增 `src/__tests__/SearchView.cardIssues.test.tsx`（5 个用例：语言清洗、本地点击派发事件、外链点击 openUrl、追加后立即可见、空推荐提示），先红后绿。
- 前端测试：8 测试文件，71 tests passed（含 SearchView.improvements 4 + cardIssues 5 + render 1 + 既有 61）。
- 类型检查：`pnpm typecheck` 通过。
- 后端编译：`cargo check --lib` 通过。
- 构建发布：`pnpm tauri build` 成功产出 MSI 与 NSIS 安装包。
- 产物归档：已同步至 `Previous/os-compass.exe`（SHA-256：`893BA3148CE0893E4136C0F56C5E15934D1E635CE1384AD7001F66C5C8BA0F50`）与 `release/`。

---

### 需求
1. web_results/local_results 合计超过 5 条时默认折叠，下方提供“展开全部（共 N 条）/ 收起”按钮。
2. 结果列表末尾“让大模型再推荐 5 个”按钮，调用后端 `recommend_more_projects` 并把返回按 url 去重追加到对应消息的 `web_results`。
3. 搜索历史每条加 hover 可见 ✕，点击调用 `delete_search_history_item(id)`，单独删除该条。

### 实施
- 后端：`src-tauri/src/commands/search_cmd.rs` 新增 `delete_search_history_item(id)` 与异步 `recommend_more_projects(query, limit?) -> RecommendMoreResult { items }`（复用 `recommend_projects_with_llm`，source=`llm_recommend`），在 `lib.rs` invoke_handler 注册。
- 前端 API：`src/api/search.ts` 新增 `deleteSearchHistoryItem(id)` 与 `recommendMoreByLLM(query, limit?)`。
- 前端 UI：`SearchView.tsx` 新增内部 `ResultsList` 组件（折叠阈值 5、展开/收起按钮、“让大模型再推荐 5 个”按钮，内部 `useState(expanded/asking)`）；新增 `handleDeleteHistoryItem` 与 `askMoreForQuery(messageId, baseQuery)` handler；历史每行改成 `<div group>` 包裹查询按钮 + hover ✕ 删除按钮（`aria-label="search.deleteHistoryItem"`）。
- i18n：zh/en 新增 `search.deleteHistoryItem/showAll/collapse/recommendMore/recommending/recommendFailed/recommendEmpty/historyDeleteFailed`，实现 `{{count}}` 插值。

### TDD 验证
- 新增 `src/__tests__/SearchView.improvements.test.tsx`，3 用例（折叠、追加推荐、✕ 删除单项），用 `vi.mock('../api/search', …)` 替换整个模块（named import captured binding 不被 spy 替换）。
- 三个用例一次运行全绿。

### 全量验证
- 前端：65 tests passed（SearchView.improvements 3 + SearchView.render 1 + 既有 61）
- typecheck：passed
- 后端：cargo check --lib passed
- 构建：pnpm tauri build MSI + NSIS 成功，已同步到 `Previous/os-compass.exe`（SHA-256 `131492A9AE0C657B35F51506B91A6E9814802428FD802FD24919280BBA670C6E`）与 `release/` 安装包。

### 清理
- 删除上一轮向量取证遗留的 `scripts/embedding_connectivity_check.py`（含读取真实 API key 逻辑，不应入库）。

---

## 2026-09-03 23:39 - 向量模型连通性与配置读取修复（端到端验证）

### 用户线索验证结果

用户的怀疑方向完全正确：
1. **向量模型连通性**：SiliconFlow `https://api.siliconflow.cn/v1/embeddings`（`BAAI/bge-m3`）完全健康，单次调用 ~1.5-2.0s，返回 1024 维向量，**API 本身未卡住**。
2. **真正的断裂点**：
   - `plugins.db.embedding_settings` 表列名是 `vss_extension_path`，但 `embedding.rs:237-248` 和 `commands/search_cmd.rs:281` 查询/更新用的是 `vec_extension_path`。
   - 列名不匹配导致 `load_embedding_settings()` 异常，fallback 返回默认配置（API Key 为空），`is_enabled()` 永远返回 `false`。
   - 结果：**用户配置的 SiliconFlow + BAAI/bge-m3 从未真正生效**，系统一直显示"embedding 未启用"，导致搜索直接跳过向量、进入爬虫超时与 40 秒的 LLM 兜底推荐。
3. **另一个致命点**：
   - 仓库库 `project_embeddings` 是普通 SQLite 表（存 JSON 字符串），但 `semantic_search` 原代码用了 sqlite-vec 虚表语法 `WHERE embedding MATCH ? AND k = ? ORDER BY distance`，即便配置读对也会抛 `no such column: distance`。

### 修复

1. **统一列名**：`embedding.rs` 与 `search_cmd.rs` 全部统一读取 `vss_extension_path`，与已有数据库表保持一致。
2. **重写 `semantic_search`**：直接在 Rust 侧读取已存的 20 条项目向量（JSON 数组），并与查询向量做**余弦相似度（cosine similarity）计算**后排序取 Top N。无需依赖 sqlite-vec C 扩展，对已有 20 条向量立即生效。
3. **真实 E2E 验证**：
   - "推荐一个开源视频剪辑工具" → 命中 shotcut（0.7089）、reclip（0.6332）
   - "Rust CLI 工具" → 命中 Kode-CLI（0.5138）
   - 单次搜索总耗时 **~1.7 秒**，不再跑 40 秒的 LLM 兜底。
4. **全链路构建验证**：
   - 前端 62 个测试通过
   - typecheck 通过
   - `cargo check --lib` 通过
   - `pnpm tauri build` MSI + NSIS 成功，产物 SHA-256 与 `Previous/os-compass.exe` 完全一致。

---

## 2026-09-03 22:14 - 意图搜索完成态不退出（真实组件测试复现）

### 证据

- 新增 `SearchView.render.test.tsx`，用 testing-library 真实渲染 `SearchView` 组件，mock IPC API 后模拟“输入 → 点击发送 → intentSearch 返回结果”的完整流程
- 修复前：测试在 3 秒超时内仍显示 “正在分析语义...” 文本，断言失败
- 定位：`updatePhase` 用 `startTransition` 异步执行 `setMessages`，等到批处理执行时读取 `loadingIdRef.current`；但 `finally` 块在此之前已把它置为 `null`，导致原消息找不到，`msg.phase` 始终是 `analyzing`
- 第二次 `setMessages`（注入 results）也读取了同一个 ref，同样失效

### 修复

- `SearchView.tsx` handleSubmit 内改为使用本次请求固定的局部变量 `loadingId` 作为消息匹配键，不再读取会被 finally 清空的 ref
- 验证：组件测试通过、全量 62 passed、typecheck、cargo check --lib、pnpm tauri build MSI+NSIS 均成功；构建产物与 Previous/os-compass.exe SHA-256 一致

---

## 2026-09-03 18:21 - 意图搜索“永远显示正在搜索”前端渲染 Bug 修复

### 铁证

`plugin_search.db.search_history` 表中用户最后一次搜索（id=3, query="你好，请给我推荐几款开源视频处理工具", result_count=8, created_at=2026-09-03 17:51:26）证明：后端已成功完成并写入 8 条结果。前端 SearchView 仍显示“正在搜索”，问题在**前端**而非后端。

### 根因（基于代码与数据库证据）

`SearchView.tsx:428` 原代码 `{msg.phase ? <TypingIndicator /> : 内容}`。`updatePhase('idle', ...)` 将 msg.phase 设置为 `'idle'`，但 `'idle'` 是非空字符串，条件仍为真，结果完成态消息被持续渲染为打字动画。

同时发现测试脱节：`intentSearch.test.ts` 与 `full-system.test.ts` 测试的是各自复刻的 `shouldShowTypingIndicator(MessageItem)` 函数，而生产组件从未使用该函数。61 个测试全过，但测的是测试自己的代码副本。

### 修复

- `SearchView.tsx` 抽出并 export `shouldShowTypingIndicator(phase?: SearchPhase): boolean`，生产渲染改为调用它。
- 两个测试文件改为 import 生产函数，删除本地副本，所有调用从传 `MessageItem` 改为传 `msg.phase`。
- 验证：`pnpm test` 61 个全过、`pnpm typecheck` 通过、`cargo check --lib` 通过、`pnpm tauri build` MSI+NSIS 成功。
- `release/` 与 `Previous/` 已更新到 18:20 构建。

---

## 2026-09-03 意图搜索回归修复（第二轮）

- 根因：`llm_parser.rs` 的 `parse_content_with_llm` 仍用 `block_on` 阻塞；联网依赖 `localhost:8080/crawl`、爬虫不可用时无 LLM 直接推荐兜底。
- 修复：`parse_content_with_llm` 改为原生 async/await；新增 `recommend_projects_with_llm`；`three_layer_search.llm_search` 在爬虫失败/空结果时降级到 LLM 直接推荐并补充耗时日志；清理 `search.rs` 未使用的 `compute_url_hash` 与 `sha2/hex` 导入。
- 验证：`cargo check --lib`、`pnpm typecheck`、`pnpm test`（61 passed）、`pnpm tauri build` 均通过
- 最新安装包：`release/` 下 MSI + NSIS；`Previous/` 已更新

---

## 2026-09-03 意图搜索回归修复（第一轮）

- 根因：`intent_search` 使用 `block_on` 调用异步搜索，且 `three_layer_search` 在异步网络搜索期间持有 `DATABASE` 锁
- 另外修复：仓库配置统一保存目录路径；兼容旧版数据库文件路径与空配置
- 验证：`cargo check --lib`、`pnpm typecheck`、`pnpm test`（61 passed）、`pnpm tauri build` 均通过
- 最新安装包：`release/` 下 MSI + NSIS；`Previous/` 已更新

---

## ⚠️ 已延后问题

| 问题编号 | 模块 | 问题 | 状态 |
|---------|------|------|------|
| ISSUE-001 | 意图搜索 | off_topic 流程不工作 | 🔵 已延后（无实质用途） |

**问题详情**：`docs/P0问题记录_意图搜索off_topic流程不工作.md`

---

## ⚠️ 已解决问题（需用户验证）

| 问题编号 | 模块 | 问题 | 状态 |
|---------|------|------|------|
| ISSUE-002 | 惏雷达 | Windows 系统通知不显示 | 已修复代码，用户需检查 Windows 通知权限 |

**问题详情**：
- 代码正确调用 Tauri notification API
- 日志显示 `Notification sent successfully`
- 问题原因：Windows 通知权限可能被系统阻止
- **解决步骤**：
  1. Windows 设置 → 系统 → 通知
  2. 确保 "来自应用和系统其他发送者的通知" 已开启
  3. 确保 os-compass 的通知权限已开启
  4. 检查是否开启了勿扰模式/关注模式

---

## 项目整体状态

| 阶段 | 状态 | 完成时间 |
|------|------|----------|
| Phase 0 准备期 | ✅ 完成 | 2026-07-06 |
| Phase 1-1 MVP 核心 | ✅ 完成 | 2026-08-19 |
| Phase 1-2 增强功能 | ✅ 完成 | 2026-08-19 |
| M13 情报雷达 + 意图搜索 | ✅ 完成 | - |
| M14 插件系统扩展 | ✅ 完成 | 2026-07-20 |
| M15 遗留问题修复 | ✅ 完成 | 2026-07-24 |
| M16 初始化向导 + 异常处理 | ✅ 完成 | - |
| M17 首页重构 + 向量搜索 | ✅ 完成 | - |
| M18 智能意图搜索 | ✅ 完成 | off_topic 已延后 |
| M18-2 向量搜索改造 | ✅ 完成 | sqlite-vss → sqlite-vec |
| **M19 平台扩展** | ⚠️ 部分完成 | 18h | M19.1/M19.2/M19.3 代码已完成，待用户验证 |
| **M20 批量向量化 + 初始化分类** | ✅ 已完成 | 5h | M20.1 批量向量化 + M20.2 初始化预置分类 |
| **M21 分类自定义导入** | ✅ 已完成 | 2h | 分类管理自定义导入功能 |
| **M22 意图搜索历史** | ✅ 已完成 | 5h | search_history 表 + 历史侧栏 + 点击重搜 |
| **M23 安装打包发布** | 📋 待开发 | 11h | 仅需求文档，代码未开始 |

---

## 📋 当前开发计划

详见：`docs/当前开发计划.md`

| 工作包 | 里程碑归属 | 状态 | 预估 |
|--------|-----------|------|------|
| A. 首页搜索与意图搜索统一 | M17/M18 遗留 | ✅ 已完成（2026-09-03） | 4h |
| B. M23 安装打包与版本发布 | M23 | 📋 待开发 | 11h |
| C. 向量化时机优化 | M17 遗留 | ⏸️ 方案待确认（推荐方案 B） | 2h |

---

### 遗漏需求（未分配到里程碑）

| 需求 | 说明 | 优先级 |
|------|------|--------|
| （暂无，初始化分类已移至 M20） | - | - |

---

## M19 当前进度

| 子功能 | Task | 工时 | 状态 |
|--------|------|------|------|
| **M19.1 平台扩展** | Task 1-6 | 18h | ✅ 完成（待用户验证） |
| **M19.2 广告学习** | Task 7-10 | 9h | ✅ 完成（待用户验证） |
| **M19.3 定时采集** | Task 11-15 | 11h | ✅ 完成（待用户验证通知功能） |

### M19.3 完成进度

| Task | 内容 | 状态 |
|------|------|------|
| Task 11 | 定时采集开关 UI | ✅ 完成 |
| Task 12 | 后台定时任务 | ✅ 完成 |
| Task 13 | 系统通知集成 | ✅ 完成 |
| Task 14 | 数据去重逻辑 | ✅ 完成 |
| Task 15 | 托盘菜单集成 | ✅ 完成 |

### M19.3 Bug 修复

| 时间 | 问题 | 修复 |
|------|------|------|
| 2026-08-23 | useEffect 嵌套 useRef | 拆分 useEffect |
| 2026-08-23 | 页面不刷新 | 后端始终 emit 事件 |
| 2026-08-23 | 通知不出现 | 添加 capabilities 权限 |
| 2026-08-23 | 程序启动失败 | 移除错误的 plugins 配置 |

### M19.2 完成进度

| Task | 内容 | 状态 |
|------|------|------|
| Task 7 | 广告标记 UI | ✅ 完成 |
| Task 8 | LLM 广告判定服务（ad_detector.rs） | ✅ 完成 |
| Task 9 | 屏蔽列表管理（ad_patterns_cmd.rs） | ✅ 完成 |
| Task 10 | 雷达数据过滤集成 | ✅ 完成 |

### M19.2 提交记录

| Commit | 说明 |
|--------|------|
| 3f25926 | feat(M19.2): 雷达采集广告过滤集成 |
| 0d88fc7 | feat(M19.2): 广告标记 UI 实现 |
| 21950d7 | feat(M19.2): 广告学习功能后端实现（修复死锁问题） |

### ⚠️ M19.2 死锁问题修复

**问题**：程序启动时卡死

**根因**：`init_ad_patterns()` 在已持有 SYSTEM_DB 锁的代码块内被调用，导致死锁

**修复**：新增 `init_ad_patterns_with_conn()` 函数，使用已打开的连接

### M19.2 提交记录

| Commit | 说明 |
|--------|------|
| 0d88fc7 | feat(M19.2): 广告标记 UI 实现 |
| 21950d7 | feat(M19.2): 广告学习功能后端实现（修复死锁问题） |
| c6024cd | revert: 回滚 M19.2 - 导致程序卡死 |

### M19.1 完成内容

| 平台 | 文件 | API | 状态 |
|------|------|-----|------|
| NPM | `npm.rs` | `registry.npmjs.org` | ✅ |
| PyPI | `pypi.rs` | `pypi.org/pypi/{pkg}/json` | ✅ |
| Crates.io | `crates.rs` | `crates.io/api/v1/crates` | ✅ |

### M19.1 提交记录

| Commit | 说明 |
|--------|------|
| a0b3cbc | NPM 适配器 + TS 类型修复 |
| 3751f68 | PyPI + Crates.io 适配器 |

---

## 参照文档

| 文档 | 说明 |
|------|------|
| `docs/M19实施计划.md` | Task 分解、工时、验收标准 |
| `docs/M19其他常用网站采集扩展.md` | 需求完整说明 |
| `docs/M19平台扩展详细分析.md` | API + 评分算法 |
| `docs/数据库设计说明书.md` §11-13 | 表结构 |

---

## M20: 批量向量化 + 初始化预置分类

### M20.1 批量向量化功能

**功能概述**：在看板页添加批量向量化按钮，快速向量化当前仓库所有项目。

| Task | 内容 | 工时 | 状态 |
|------|------|------|------|
| Task 1 | 看板页向量化按钮（KanbanView.tsx） | 0.5h | 📋 待开发 |
| Task 2 | 向量扩展路径设置（SettingsDialog.tsx） | 0.5h | 📋 待开发 |
| Task 3 | 后端批量向量化 API（embedding.rs） | 1h | 📋 待开发 |

### M20.2 初始化预置分类（M18 遗漏需求）

**功能概述**：首次初始化时提供默认最佳实践分类模板（技术/学术/商业/创意四级分类树）。

**来源**：
- `requirements.md` MVP-002：创建 3 个预置分类并写入 explain 字段
- `概要设计说明书.md` §8.8：用户创建新仓库时可选择导入预置分类体系
- `详细设计说明书.md` 7.8：四级分类树设计

**预置分类结构**（四级）：
```
技术 (technology)
├── 前端开发 (technology/frontend)
│   ├── Web框架 (technology/frontend/framework)
│   │   ├── React生态 (technology/frontend/framework/react)
│   │   └── Vue生态 (technology/frontend/framework/vue)
│   └── UI组件库 (technology/frontend/ui)
├── 后端开发 (technology/backend)
│   ├── Web框架 (technology/backend/framework)
│   └── 数据库 (technology/backend/database)
├── 移动开发 (technology/mobile)
├── DevOps (technology/devops)
└── AI/ML (technology/ai)

学术 (academic)
├── 基础科学 (academic/science)
├── 工程学科 (academic/engineering)
├── 医学 (academic/medicine)
└── 社会科学 (academic/social)

商业 (business)
├── 企业管理 (business/enterprise)
├── 金融科技 (business/fintech)
├── 市场营销 (business/marketing)
└── 创业 (business/startup)

创意 (creative)
├── 内容创作 (creative/content)
├── 音视频 (creative/av)
├── 设计 (creative/design)
└── 游戏 (creative/game)
```

**每个分类必须包含 explain 字段**（供 AI 分类时作为判定边界）。

**实现要点**：

| Task | 内容 | 工时 | 状态 |
|------|------|------|------|
| Task 1 | `init_schema.sql` 添加预置分类 INSERT 语句 | 1h | 📋 待开发 |
| Task 2 | `db.rs` 新增 `ensure_preset_categories()` 函数 | 1h | 📋 待开发 |
| Task 3 | 仓库创建时调用分类预置（可选导入） | 1h | 📋 待开发 |

**存储位置**：仓库库 `os_compass.db` 的 `categories` 表

**验收标准**：
- [ ] 新建仓库时自动创建预置分类
- [ ] 每个预置分类有 explain 字段（非空）
- [ ] AI 导入项目时能根据 explain 做分类判断
- [ ] 预置分类树为四级结构（parent_id 层级关系正确）

**涉及文件**：
- `scripts/init_schema.sql` - 添加预置分类 INSERT
- `src-tauri/src/db.rs` - 新增 `ensure_preset_categories()` 方法
- `src-tauri/src/lib.rs` - 仓库创建时调用分类预置

### M20 工时汇总

| 模块 | 任务 | 工时 |
|------|------|------|
| 批量向量化 | Task 1-3 | 2h |
| 初始化预置分类 | Task 1-3 | 3h |
| **总计** | 6 Tasks | **5h** |

---

## M17: 首页重构 + 意图搜索向量搜索

### 设计状态

| 文档 | 状态 |
|------|------|
| 原型 `search-view.html` | ✅ 完成 |
| M17 实施计划 | ✅ 完成 |
| 需求确认文档 §10 | ✅ 完成 |
| 详细设计说明书 §4.1.10 | ✅ 完成 |
| 数据库设计说明书 | ✅ 完成 |
| 智能意图搜索流程设计 | ✅ 定版 |

### 原型
`design-system/public/search-view.html`

### 里程碑归属

| 里程碑 | 内容 |
|--------|------|
| M13 | 基础意图搜索（关键词匹配 + LLM 联网） |
| **M17** | 向量搜索增强（sqlite-vss + ChatGPT 界面） |

### 任务清单

| Task | 内容 | 预估工时 | 状态 |
|------|------|----------|------|
| Task 1 | SearchView 组件 | 4h | ✅ 完成 |
| Task 2 | 最近使用侧栏 | 1h | ✅ 完成 |
| Task 3 | Embedding 设置页面 | 2h | ✅ 完成 |
| Task 4 | sqlite-vss 动态加载 | 2h | ✅ 完成 |
| Task 5 | embedding.rs 向量模块 | 3h | ✅ 完成 |
| Task 6 | 向量生成时机 + 向量化按钮 | 2h | ✅ 完成 |
| Task 7 | LLM 联网搜索 | 2h | ✅ 完成 |
| Task 8 | 意图搜索 API 调用 | 1h | ✅ 完成 |
| Task 9 | 测试 + 构建 | 2h | ✅ 完成 |
| **总计** | | **19h** | ✅ |

### ⚠️ M17 遗漏功能（已移至 M20）

| # | 功能 | 说明 | 状态 |
|---|------|------|------|
| 1 | 看板页批量向量化按钮 | 快速向量化当前仓库所有项目 | ✅ 已完成 |
| 2 | sqlite-vss 扩展路径设置 | 设置页面缺少 vss 路径配置入口 | ✅ 已完成 |

### 分类预置功能

| # | 功能 | 说明 |
|---|------|------|
| 1 | 预置分类数据 | 四级分类结构 |
| 2 | 安装时导入 | 新仓库创建时可选导入 |
| 3 | 分类管理 | 用户可增删改 |

### 四级分类结构（扩展版）

```
## M18: 智能意图搜索 + 分类预置

### 意图搜索任务清单

| Task | 内容 | 预估工时 | 状态 |
|------|------|----------|------|
| Task 1 | 意图分析（LLM Prompt + API） | 4h | ✅ 完成 |
| Task 2 | 追问确认 UI | 3h | ✅ 完成 |
| Task 3 | 并行搜索（本地 + LLM 同时） | 4h | ✅ 完成 |
| Task 4 | 智能推荐（分类/标签/扩展） | 3h | ✅ 完成 |
| Task 5 | 会话管理（上下文 + 历史） | 2h | ✅ 完成 |
| Task 6 | 异常处理（超时 + 重试 + 降级） | 2h | ✅ 完成 |
| Task 7 | 缓存策略（本地 + LLM 结果缓存） | 2h | ✅ 完成 |
| Task 8 | 仓库切换（状态保存 + 向量重载） | 2h | ✅ 完成 |
| Task 9 | 测试 + 调优 | 4h | ✅ 完成 |
| **总计** | | **26h** | ✅ |

### 意图搜索优化项

| # | 问题 | 解决方案 | 优先级 | 状态 |
|---|------|----------|--------|------|
| 1 | 结果列表显示原始状态（TO_EXPLORE、DIVING） | 改为中文或隐藏状态标识 | P1 | ✅ 已修复 |
| 2 | `common.loading` 等 i18n 显示 | i18n 翻译正确，无需修改 | P1 | ✅ 已确认 |
| 3 | 切换仓库后搜索失效 | 添加 project_embeddings 表 + vss 重置 | P0 | ✅ **已修复** |
| 4 | 切换仓库后再切回搜索失效 | reset_vss_state 函数 | P0 | ✅ **已修复** |

### 意图搜索问题排查清单

#### 1. 仓库切换后搜索失效

**可能原因**：
- 向量数据未存储在正确的仓库库
- 仓库切换时未重新加载向量索引
- sqlite-vss 扩展加载路径问题

**排查点**：
```
□ 向量数据表是否在仓库库 `os_compass.db` 中
□ 仓库切换时向量模块是否重新初始化
□ sqlite-vss 扩展是否支持多数据库切换
□ SearchView 组件是否监听仓库切换事件
```

#### 2. UI 显示问题

**需要排查的文件**：
- `SearchView.tsx` - 搜索视图
- `HomePage.tsx` - 首页
- `RadarInbox.tsx` - 雷达收件箱

**排查关键词**：
```
common.searching
common.loading
common.error
common.success
```

#### 3. 向量搜索存储验证

**设计要求**：
| 数据 | 位置 | 说明 |
|------|------|------|
| sqlite-vss 扩展 | 系统库目录 | 全局共享 |
| Embedding API 配置 | 系统库 | 全局共享 |
| 向量数据 | 仓库库 `os_compass.db` | 与 projects 表同一文件 |

**需要验证**：
```
□ project_embeddings 表是否存在当前仓库库
□ 向量数据是否与项目 ID 正确关联
□ 切换仓库后是否能查询到向量数据
```

#### 4. 完整排查任务

| # | 任务 | 说明 | 优先级 |
|---|------|------|--------|
| 1 | 搜索组件 UI 文本排查 | 替换所有 `common.*` 为自然语言 | P1 |
| 2 | 仓库切换事件监听 | 确保 SearchView 响应仓库切换 | P0 |
| 3 | 向量数据存储位置验证 | 确认向量在仓库库而非系统库 | P0 |
| 4 | 向量模块重新加载逻辑 | 切换仓库时重新加载 vss 扩展 | P0 |
| 5 | 数据库路径同步检查 | 确保向量模块使用当前仓库路径 | P0 |

### ⚠️ 数据库初始化检查清单（M21 教训）

**新增表时，必须同步更新 `db.rs` 两处定义：**

| 位置 | 用途 | 更新时机 |
|------|------|----------|
| `INIT_SCHEMA_SQL` 常量 | 新建库时使用 | 新增仓库库表 |
| `missing_tables` 数组 | 已有库自动迁移 | 新增仓库库表 |

**检查步骤**：
1. [ ] 新增表是否在 `embedding.rs` 等模块中使用？
2. [ ] `INIT_SCHEMA_SQL` 是否包含该表？
3. [ ] `missing_tables` 数组是否包含该表？
4. [ ] 启动后测试新建库
5. [ ] 启动后测试已有库迁移

### ⚠️ 未实现需求

| # | 需求 | 说明 | 优先级 | 状态 |
|---|------|------|--------|------|
| 1 | 向量化时机优化 | 新增项目时自动/按需向量化 | P2 | 待定 |

**向量化时机方案**：
- 方案 A：导入/爬取完成后触发（数据已就绪）
- 方案 B：点击项目详情时生成（按需）
- 方案 C：后台队列 + 首次搜索前（延迟生成）

### ⚠️ M17 遗漏功能（已移至 M20）

```
├── 1 技术/开发
│   ├── 1.1 前端开发
│   │   ├── 1.1.1 Web 框架（Vue、React、Angular、Svelte）
│   │   ├── 1.1.2 小程序/跨端（微信小程序、UniApp、Taro）
│   │   ├── 1.1.3 桌面应用（Electron、Tauri、WPF）
│   │   └── 1.1.4 可视化/图表（ECharts、D3.js、AntV）
│   ├── 1.2 后端开发
│   │   ├── 1.2.1 Web 框架（Spring、Django、Express、NestJS）
│   │   ├── 1.2.2 微服务/网关（Spring Cloud、Kong、APISIX）
│   │   ├── 1.2.3 数据库/ORM（MySQL、PostgreSQL、Prisma）
│   │   └── 1.2.4 缓存/消息队列（Redis、RabbitMQ、Kafka）
│   ├── 1.3 移动开发
│   │   ├── 1.3.1 Android（Kotlin、Jetpack Compose）
│   │   ├── 1.3.2 iOS（Swift、SwiftUI）
│   │   └── 1.3.3 跨平台（Flutter、React Native）
│   ├── 1.4 AI/机器学习
│   │   ├── 1.4.1 大模型/LLM（GPT、LLaMA、Stable Diffusion）
│   │   ├── 1.4.2 机器学习框架（PyTorch、TensorFlow、JAX）
│   │   ├── 1.4.3 计算机视觉（OpenCV、YOLO、MMDetection）
│   │   └── 1.4.4 自然语言处理（HuggingFace、spaCy、BERT）
│   ├── 1.5 DevOps/云原生
│   │   ├── 1.5.1 容器/K8s（Docker、Kubernetes、Helm）
│   │   ├── 1.5.2 CI/CD（Jenkins、GitLab CI、GitHub Actions）
│   │   ├── 1.5.3 基础设施即代码（Terraform、Pulumi）
│   │   └── 1.5.4 监控/可观测性（Prometheus、Grafana）
│   └── 1.6 数据工程
│       ├── 1.6.1 大数据框架（Hadoop、Spark、Flink）
│       ├── 1.6.2 数据仓库（ClickHouse、StarRocks）
│       └── 1.6.3 ETL/数据治理（Airflow、Dagster、dbt）
│
├── 2 学术/科研
│   ├── 2.1 基础科学
│   │   ├── 2.1.1 数学（计算数学、统计学、运筹学）
│   │   ├── 2.1.2 物理学（量子计算、凝聚态物理）
│   │   ├── 2.1.3 化学（计算化学、药物化学）
│   │   └── 2.1.4 生物学（生物信息学、神经科学）
│   ├── 2.2 工程学科
│   │   ├── 2.2.1 土木工程（BIM、有限元分析）
│   │   ├── 2.2.2 机械工程（CAD、CAE、CAM）
│   │   └── 2.2.3 电子/电气（PCB、嵌入式、信号处理）
│   ├── 2.3 医学/生命科学
│   │   ├── 2.3.1 药物研发（分子对接、AI制药）
│   │   ├── 2.3.2 基因技术（CRISPR、基因测序）
│   │   └── 2.3.3 医学影像（CT、MRI、超声）
│   └── 2.4 社会科学
│       ├── 2.4.1 经济学（计量经济、金融工程）
│       ├── 2.4.2 心理学（认知科学、行为分析）
│       └── 2.4.3 社会学（网络分析、舆情分析）
│
├── 3 商业/金融
│   ├── 3.1 企业管理
│   │   ├── 3.1.1 ERP/企业管理（SAP、Oracle、用友）
│   │   ├── 3.1.2 CRM/客户管理（Salesforce、HubSpot）
│   │   └── 3.1.3 项目管理（Jira、Confluence）
│   ├── 3.2 金融科技
│   │   ├── 3.2.1 支付系统（支付宝、Stripe、PayPal）
│   │   ├── 3.2.2 风险管理（风控模型、反欺诈）
│   │   └── 3.2.3 区块链/加密货币（DeFi、NFT）
│   └── 3.3 市场营销
│       ├── 3.3.1 CRM/CDP（Marketing Cloud）
│       ├── 3.3.2 广告投放（DSP、程序化广告）
│       └── 3.3.3 增长黑客（A/B测试、用户裂变）
│
├── 4 创意/媒体
│   ├── 4.1 内容创作
│   │   ├── 4.1.1 写作工具（Markdown、Notion、Obsidian）
│   │   ├── 4.1.2 图文编辑（Canva、Figma）
│   │   └── 4.1.3 博客/CMS（WordPress、Hexo、Hugo）
│   ├── 4.2 音视频处理
│   │   ├── 4.2.1 视频剪辑（FFmpeg、Premiere、DaVinci）
│   │   ├── 4.2.2 特效/动画（After Effects、Cinema 4D）
│   │   └── 4.2.3 流媒体（OBS、WebRTC）
│   └── 4.3 设计工具
│       ├── 4.3.1 UI/UX 设计（Figma、Sketch）
│       ├── 4.3.2 3D 建模（Blender、3ds Max）
│       └── 4.3.3 CAD/工程制图（AutoCAD、SolidWorks）
│
├── 5 教育/学习
│   ├── 5.1 在线教育
│   │   ├── 5.1.1 LMS 学习系统（Moodle、Canvas）
│   │   ├── 5.1.2 直播/视频课（Zoom、腾讯课堂）
│   │   └── 5.1.3 题库/考试（问卷星、猿题库）
│   ├── 5.2 教育科技
│   │   ├── 5.2.1 自适应学习（Khan Academy）
│   │   ├── 5.2.2 教育游戏（游戏化学习、STEAM）
│   │   └── 5.2.3 智慧校园（校园管理、考勤）
│   └── 5.3 语言学习
│       ├── 5.3.1 翻译工具（DeepL、Google Translate）
│       ├── 5.3.2 词典/背词（Anki、墨墨背单词）
│       └── 5.3.3 语音/口语（流利说、多邻国）
│
├── 6 生活方式
│   ├── 6.1 健康医疗
│   │   ├── 6.1.1 挂号/问诊（微医、丁香医生）
│   │   ├── 6.1.2 健康管理（Keep、华为健康）
│   │   └── 6.1.3 医疗信息化（HIS、LIS、PACS）
│   ├── 6.2 家居生活
│   │   ├── 6.2.1 智能家居（米家、HomeKit）
│   │   ├── 6.2.2 电商/购物（淘宝、京东）
│   │   └── 6.2.3 美食/菜谱（下厨房、豆果美食）
│   └── 6.3 出行旅游
│       ├── 6.3.1 导航/地图（高德、Google Maps）
│       ├── 6.3.2 旅行预订（携程、Airbnb）
│       └── 6.3.3 攻略/社区（小红书、马蜂窝）
│
├── 7 公共/社会
│   ├── 7.1 政府/政务
│   │   ├── 7.1.1 智慧城市（城市大脑、数字孪生）
│   │   ├── 7.1.2 政务服务（最多跑一次、一网通办）
│   │   └── 7.1.3 公共安全（监控、应急管理）
│   ├── 7.2 公益/非营利
│   │   ├── 7.2.1 慈善/捐赠（水滴筹、腾讯公益）
│   │   └── 7.2.2 志愿服务（时间银行）
│   └── 7.3 环保/可持续发展
│       ├── 7.3.1 碳中和/减排（碳核算、碳交易）
│       ├── 7.3.2 垃圾分类（智能回收）
│       └── 7.3.3 能源管理（能耗监控、智慧电网）
│
└── 8 工业/生产
    ├── 8.1 制造业
    │   ├── 8.1.1 MES 生产执行（西门子、施耐德）
    │   ├── 8.1.2 PLC/工控（S7、三菱 PLC）
    │   ├── 8.1.3 工业物联网（SCADA、数字工厂）
    │   └── 8.1.4 质量检测（AOI、机器视觉）
    ├── 8.2 农业科技
    │   ├── 8.2.1 精准农业（无人机植保、传感器）
    │   ├── 8.2.2 智慧养殖（畜牧监控、追溯系统）
    │   └── 8.2.3 供应链/溯源（农产品追溯、冷链）
    └── 8.3 能源/资源
        ├── 8.3.1 新能源（光伏、风电、储能）
        ├── 8.3.2 矿山/冶金（智慧矿山）
        └── 8.3.3 水务/环保（污水处理、监测）
```

### 统计信息

| 层级 | 数量 |
|------|------|
| 一级类目 | 8 个 |
| 二级类目 | 29 个 |
| 三级类目 | 约 85 个 |
| 四级类目 | 约 250+ 个具体技术/场景 |

### 设计原则

| 原则 | 说明 |
|------|------|
| 实用性 | 基于实际应用场景，非纯学术分类 |
| 可扩展 | 支持用户自定义四级、五级分类 |
| 跨领域 | 覆盖技术、学术、商业、创意等 |
| 与时俱进 | 可随技术发展新增（如 AIGC、元宇宙） |

### 待细化
- 开发前进一步细化四级分类内容
- 确定导入方式的具体交互

### M18 工时汇总

| 模块 | 任务 | 工时 |
|------|------|------|
| 意图搜索新功能 | Task 1-6 | 20h |
| 分类预置 | - | 3h |
| 意图搜索问题排查 | UI + 向量存储 + 仓库切换 | 8h |
| **总计** | | **33h** |

---

## M16 任务状态（2026-07-26）

| Task | 内容 | 状态 |
|------|------|------|
| Task 15 | 导航优化 | ✅ 完成 |
| Task 16 | i18n + 构建验证 | ✅ 完成 |
| Task 17 | unwrap 替换 | ✅ 完成 |
| Task 18 | log crate 集成 | ✅ 完成 |
| Task 19 | 切片边界检查 | ✅ 完成 |
| Task 20 | 统一错误类型（AppError） | ✅ 完成 |
| Task 21 | 关键函数入口/出口日志 | ✅ 完成 |
| Task 22 | 前端统一错误组件 | ✅ 完成 |
| Task 23 | 日志轮转配置 | ✅ 完成 |
| Task 24-26 | 文章生成 | ✅ 完成 |

---

**M16 异常处理完成项**：
- ✅ `crawler_service.rs`: 使用 `strip_prefix` 替代直接切片
- ✅ `ai.rs`: 使用 `json.get()` 替代直接切片
- ✅ 创建 `error.rs` 模块，包含 `AppError` 枚举
- ✅ 添加 `tauri-plugin-log` 日志轮转插件

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

---

## M18-2 向量搜索改造

### 核心目标

**意图搜索功能可以正常使用**

| 目标 | 说明 | 验收标准 |
|------|------|----------|
| 向量搜索可用 | sqlite-vec 替代 sqlite-vss | 搜索返回正确结果 |
| 关键词搜索兜底 | 向量搜索失败时降级 | 仍有结果返回 |
| LLM 联网搜索 | GitHub 搜索正常 | 返回推荐项目 |
| 并行搜索 | 本地+联网同时执行 | 速度不受影响 |
| 意图分析 | LLM 意图判断 | 返回 JSON 结构 |

### 技术选型

| 项目 | 原方案 | 新方案 |
|------|--------|--------|
| 向量扩展 | sqlite-vss | sqlite-vec |
| Windows 支持 | ❌ 不支持 | ✅ 支持 |
| 接入方式 | 预编译 DLL | Rust crate |
| 预估工时 | - | 7h |

### 改造任务清单

| Task | 内容 | 预估工时 | 状态 |
|------|------|----------|------|
| Task 1 | Cargo.toml 添加依赖 | 0.5h | ⏳ 待实施 |
| Task 2 | embedding.rs 重构 | 2h | ⏳ 待实施 |
| Task 3 | db.rs 建表语句 | 1h | ⏳ 待实施 |
| Task 4 | search.rs 降级+死锁修复 | 1.5h | ⏳ 待实施 |
| Task 5 | 端到端测试 | 2h | ⏳ 待验证 |
| **总计** | | **7h** | |

### 涉及文件

| 文件 | 修改内容 |
|------|----------|
| `Cargo.toml` | 添加 sqlite-vec 依赖 |
| `embedding.rs` | 向量加载 + 存储 + 搜索 |
| `db.rs` | 建表语句（普通表 → 虚拟表） |
| `search.rs` | 降级策略 + 死锁修复 |

### 里程碑归属

| 里程碑 | 内容 |
|--------|------|
| M13 | 基础意图搜索（关键词匹配 + LLM 联网） |
| **M17** | 向量搜索增强（sqlite-vec + ChatGPT 界面） |
| **M18-2** | 意图搜索功能修复（向量搜索 + 降级策略 + 死锁修复） |
