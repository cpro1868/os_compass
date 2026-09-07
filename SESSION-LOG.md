> **⚠️ 会话日志**：记录当前会话的关键操作和异常。新会话请先从 `CONTEXT.md` 和 `CURRENT-TASK.md` 恢复上下文。

# Session Log

## 2026-09-05 22:06 - 意图搜索结果卡片三项缺陷修复

### 关键操作
1. 取证定位：
   - 杂乱问题：`projects.languages` 存 JSON 数组（`["C++","QML"]`），Rust 侧原样塞入 `ProjectMatch.language`，前端无解析直出；卡片上充斥 source/已入库/Star/健康分等冗余信息。
   - 详情失败：`invoke('open_project_detail')` 命令在 Rust 侧根本不存在，主应用实际通过 `openProjectDetail` CustomEvent 接收 `projectId` 打开弹窗。
   - 推荐无反应：新结果追加在 `web_results` 尾部，而列表按 5 条截断；本地结果已满时新项目全被折叠；且内部 `expanded` state 随重渲染丢失。
2. 修复实施：
   - `search.rs` / `embedding.rs` / `search_cmd.rs`：`ProjectMatch` 增 `project_id: Option<i64>`，多语言 JSON 自动解析提取首个主语言。
   - `SearchView.tsx`：精简 `ProjectCard`（只保留名称/2行描述/语言）；本地点击派发 `openProjectDetail`，外链调用 `openUrl`；`ResultsList` 接入父级 `expandedMessages`，追加推荐成功后自动展开全部。
   - `api/search.ts`：补齐 TS 类型。
3. TDD 验证：新增 `SearchView.cardIssues.test.tsx` 5 用例，全量 71 tests passed，typecheck/cargo check/tauri build 均通过。产物归档至 Previous 和 release。

## 2026-09-04 16:18 - 意图搜索三项体验优化全部完成

### 实施
- 后端新增 `delete_search_history_item(id)` 与 `recommend_more_projects(query, limit?)`，在 `lib.rs` invoke_handler 注册
- 前端 API 新增 `deleteSearchHistoryItem` 与 `recommendMoreByLLM`
- `SearchView.tsx`：抽出 `ResultsList` 内部组件（折叠/展开 + 让大模型再推荐），新增 `handleDeleteHistoryItem`/`askMoreForQuery` handler；历史每行 hover ✕
- i18n：zh/en 新增 7 个键，含 `{{count}}` 插值

### TDD
- 新增 `SearchView.improvements.test.tsx`，3 用例（折叠、追加、✕ 删除），使用 `vi.mock('../api/search', …)` 替换整个模块（named import captured binding 不被 spy 替换）；最终全绿

### 验证
- 65 tests passed / typecheck / cargo check --lib / tauri build MSI+NSIS 全部成功
- `Previous/os-compass.exe` 已同步（SHA-256 `131492A9AE0C657B35F51506B91A6E9814802428FD802FD24919280BBA670C6E`）
- 删除临时取证脚本 `scripts/embedding_connectivity_check.py`（含读取真实 API key 逻辑）

## 2026-09-03 23:39 - 向量模型连通性与配置读取修复

### 关键结论

用户的排查线索完全命中了系统核心问题：
1. 向量模型（SiliconFlow BAAI/bge-m3）连通性 100% 正常（1.5s 响应）。
2. 代码因为列名错乱（`vec_extension_path` vs `vss_extension_path`）读不到配置，导致 `is_enabled()` 永远 false，系统绕过向量搜索走 40s 的 LLM 兜底。
3. 仓库库存的是普通 JSON 数组，但代码用 sqlite-vec 虚表语法导致 `distance` 错误。

### 修复与验证

- 列名对齐到 `vss_extension_path`
- `semantic_search` 改为 Rust 内存余弦相似度计算，对库内 20 个项目实测 1.7s 返回 top-5（视频剪辑精确命中 shotcut / reclip）
- 全量 62 个测试、typecheck、cargo check、tauri build 均通过
- Previous/os-compass.exe 已更新

---

## 2026-09-03 22:14 - 意图搜索完成态不退出（组件测试复现并修复）

### 证据

- 新增真实 SearchView 组件渲染测试，完整模拟输入→发送→intentSearch 返回结果
- 修复前测试失败（3 秒仍显示“正在分析语义...”），准确复现问题
- 根因：startTransition 异步 setMessages 读取 loadingIdRef.current，已被 finally 置 null，消息永远停在 analyzing

### 修复

- SearchView.tsx 改用本次请求固定的 loadingId 局部变量匹配消息
- 验证：62 passed、typecheck、cargo check、tauri build 均通过；构建产物与 Previous SHA-256 一致

---

## 2026-09-03 18:21 - 意图搜索前端“永远正在搜索”渲染 Bug

### 证据

- `plugin_search.db.search_history` id=3（result_count=8，17:51:26）证明后端已完成
- `SearchView.tsx:428` 原条件 `msg.phase ?` 对 `'idle'` 仍为真，TypingIndicator 永久显示

### 修复

- SearchView.tsx 抽出并使用 `shouldShowTypingIndicator(phase?: SearchPhase)`
- intentSearch.test.ts / full-system.test.ts 改为 import 生产函数，删除本地副本
- 验证：pnpm test 61 passed、typecheck、cargo check --lib、tauri build MSI+NSIS 均成功
- release/、Previous/ 已更新

---

## 2026-09-03 16:09 - 意图搜索卡死第二轮修复

### 结果

- `llm_parser.rs` 的 `parse_content_with_llm` 移除 `block_on`，改为纯 async/await
- 新增 `recommend_projects_with_llm`：爬虫不可用时直接 LLM 生成项目推荐
- `three_layer_search.llm_search` 增加爬虫失败/空结果降级到 LLM 直接推荐，补充日志
- 清理 `search.rs` 无用 `compute_url_hash`/`sha2`/`hex` 导入
- 验证：cargo check、typecheck、61 前端测试、Tauri MSI/NSIS 构建全部通过
- `release/` 与 `Previous/` 已更新

---

## 2026-09-03 14:05 - 意图搜索回归修复

### 结果

- 定位到 `block_on` + 异步期间持有 DATABASE 锁导致搜索阻塞
- `intent_search` 改为 async/await
- 本地数据库锁限制在 SQL 查询期间
- 统一仓库目录路径，并兼容旧配置
- cargo check、typecheck、61 个前端测试、Tauri MSI/NSIS 构建均通过
- `release/` 和 `Previous/` 已更新

---

## 2026-09-03 - M23 安装包归档目录

### 完成内容

- 确认 `pnpm tauri build` 已生成 MSI 与 NSIS 安装包
- 原始产物位于 `os-compass/src-tauri/target/release/bundle/`
- 新建项目根目录 `release/`
- 已归档：`release/os-compass_0.1.0_x64_en-US.msi`（15MB）
- 已归档：`release/os-compass_0.1.0_x64-setup.exe`（10MB）
- `scripts/build-release.ps1` 后续会自动同步安装包到 `release/`
- M23 文档已补充产物归档路径

### 验证

- MSI 文件存在 ✅
- NSIS 文件存在 ✅
- 构建脚本语法检查 ✅
- `git diff --check` ✅

---

## 2026-09-03 01:52 - 工作包 A 完成：首页搜索与意图搜索统一

### 完成内容

将 HomePage 简化的搜索流程对齐到 SearchView 完整意图搜索：

- 接入 analyzeIntent（30s 超时，2h 缓存）+ 追问澄清 + 会话上下文
- 复用 intentCache/searchCache，仓库切换清空
- 结果卡片带 source.local/source.llm 标签
- 智能推荐 + LLM 文本 + 搜索历史侧栏
- 项目点击改用 open_project_detail

### 修改文件

- `os-compass/src/components/HomePage.tsx`（重写，~565 行）
- `os-compass/src/locales/{zh,en}.json`（+6 键：clarification/smartRecommendation/relatedCategories/relatedTags/expandedSearch）

### 验证

- typecheck ✅
- vitest ✅ 61 passed
- pnpm tauri build ✅ 1m 51s
- Previous/os-compass.exe ✅ 01:50, 40.8MB

### 提交

```
feat(首页搜索): 补齐意图分析/追问/历史/缓存/联网结果
docs(开发计划): 首页搜索与意图搜索统一完成
```

---



### 完成内容

1. 核对未完成工作包与里程碑归属：
   - 工作包 A：首页搜索与意图搜索统一 → M17/M18 遗留（用户已批准）
   - 工作包 B：M23 安装打包发布 → M23（仅需求文档）
   - 工作包 C：向量化时机优化 → M17 遗留（方案待确认，推荐方案 B）
2. 修正文档滞后：M20/M21/M22 实际已完成，CURRENT-TASK.md 原标"待开发"
3. 新建 `docs/当前开发计划.md`，更新 `CURRENT-TASK.md`、`docs/工作进展记录.md`

### 关键发现

- HomePage.tsx 为简化搜索（直接调 intentSearch，无追问/历史/会话/联网卡片）
- SearchView.tsx 为完整版（analyzeIntent、clarification、history、缓存、聊天）
- 后续开发顺序：A → B → C

### 提交

```
（见 git log）
```

---

### 问题

1. **定时采集不刷新页面**：后端在 `new_items > 0` 时才 emit 事件
2. **useEffect 内使用 useRef**：React Hooks 规则不允许

### 修复

1. 后端无论是否有新数据都 emit `radar-scan-complete`
2. 前端拆分为独立 useEffect 监听事件
3. 移除 useRef，改用独立 useEffect

### 提交

```
7c1f6ed fix(M19.3): 使用 useRef 保存最新函数引用
75af8da fix(M19.3): 无论是否有新数据都通知前端刷新
1e29e04 fix(M19.3): 添加调度日志以便调试
c38e495 fix(M19.3): 修复定时采集间隔逻辑
db8b232 fix(M19.3): 修复编译错误 + 更新文档
c38e495 fix(M19.3): 修复定时采集间隔逻辑
```

---

## 2026-08-22 - M19.3 定时采集功能完成

### 完成内容

M19.3 定时采集功能（Task 11-15）全部完成：
- Task 11: 定时采集开关 UI ✅
- Task 12: 后台定时任务 ✅
- Task 13: 系统通知集成 ✅
- Task 14: 数据去重逻辑 ✅
- Task 15: 托盘菜单集成 ✅

### 技术修改

1. **Cargo.toml**: 添加 `tauri-plugin-notification`
2. **plugin_config_db.rs**: 新增 `radar_schedule`/`radar_notifications` 表操作
3. **radar_cmd.rs**: 新增定时任务命令
4. **lib.rs**: 托盘菜单 + 定时任务初始化
5. **RadarInbox.tsx**: 定时采集 UI 组件
6. **api/radar.ts**: 定时采集 API

### 编译修复

1. 修复 `start_radar_scheduler` Send 错误：使用独立 Tokio Runtime
2. 修复 `RadarScheduleUpdate` 类型：添加 Serialize/Deserialize derive
3. 修复 `get_radar_schedule` 返回类型：改为 `Result<RadarScheduleRow, String>`

### 构建验证

| 检查项 | 结果 |
|--------|------|
| TypeScript | ✅ |
| Rust 编译 | ✅ |
| Release 构建 | ✅ |
| Previous 目录 | ✅ |

### 提交

```
bf97ae2 chore: 更新 M19.3 定时采集完成状态
796c324 feat(M19.3): 定时采集功能
```

---

## 2026-08-19 17:30 - M19.2 UI 完成，程序正常

### 完成内容

| Task | 内容 | 状态 |
|------|------|------|
| Task 7 | 广告标记 UI | ✅ |
| Task 8 | LLM 广告判定服务 | ✅ |
| Task 9 | 屏蔽列表 CRUD API | ✅ |
| Task 10 | 雷达数据过滤集成 | 📋 待开发 |

### 测试验证

| 检查项 | 结果 |
|--------|------|
| 程序启动 | ✅ Responding = True |

### 提交

```
0d88fc7 feat(M19.2): 广告标记 UI 实现
da3ac2d docs: M19.2 UI 完成，更新状态
```

### 待开发

Task 10: 雷达数据过滤集成

---

## 2026-08-19 17:10 - M19.2 重新实现（修复死锁）

### 问题

程序启动时卡死（`Responding = False`）

### 根因

`init_ad_patterns()` 函数内部尝试获取 `SYSTEM_DB` 锁，但它是在已经持有该锁的代码块内被调用的，导致**死锁**：

```rust
// lib.rs: 在 SYSTEM_DB 锁内调用
if let Ok(system_db) = system_db::SYSTEM_DB.lock() {
    commands::init_ad_patterns()  // 这又尝试获取锁 -> 死锁
}
```

### 修复

新增 `init_ad_patterns_with_conn()` 函数，直接使用已打开的数据库连接：

```rust
pub fn init_ad_patterns_with_conn(conn: &rusqlite::Connection) -> Result<(), String> {
    conn.execute_batch(...)  // 不再获取锁
}
```

### 测试验证

| 检查项 | 结果 |
|--------|------|
| 程序启动 | ✅ Responding = True |

### 提交

```
21950d7 feat(M19.2): 广告学习功能后端实现（修复死锁问题）
3bbf9ff docs: 更新 M19.2 状态 - 修复死锁问题
```

### 状态

⚠️ 后端完成，待前端 UI 和雷达过滤集成

---

## 2026-08-19 16:45 - M19.2 广告学习功能回滚

### 问题发现

用户反馈程序卡死无法使用。测试发现 `Responding = False`。

### 根因分析

M19.2 广告学习功能代码在初始化或运行时存在阻塞问题。

### 处理措施

1. **回滚代码**：恢复到 3751f68（上一个稳定版本）
2. **删除文件**：
   - `ad_patterns_cmd.rs`
   - `ad_detector.rs`
   - `adPatterns.ts`
3. **恢复文件**：
   - `RadarInbox.tsx`
   - `radar.rs`
   - `lib.rs`
   - `commands/mod.rs`
4. **重新构建**：成功
5. **测试验证**：程序恢复正常，`Responding = True`

### 提交记录

```
c6024cd revert: 回滚 M19.2 广告学习功能 - 导致程序卡死
dcc2a53 docs: 记录 M19.2 回滚
```

### 版本

| 文件 | 时间 |
|------|------|
| `Previous/os-compass.exe` | 2026/08/19 16:43:14 |

### 教训

1. **必须自行测试**：只做编译检查不够，必须实际运行程序
2. **分步验证**：每次小修改后都要验证程序正常运行
3. **不能依赖编译通过**：编译通过不代表程序能正常运行

### 待办

M19.2 广告学习功能需要重新设计，修复导致卡死的问题。

---

## 2026-08-08 15:00 - P0 问题记录：意图搜索 off_topic 流程不工作

### 问题描述

| 项目 | 内容 |
|------|------|
| 问题编号 | ISSUE-001 |
| 优先级 | P0 |
| 模块 | 意图搜索 (Intent Search) |
| 持续时间 | ~5天 |
| 状态 | 未解决 |

**现象**：输入"你好"后，后端返回 off_topic，但 UI 始终显示"正在分析语义..."，guidance 消息未显示。

### 代码修改记录

| 日期 | 修改内容 | 结果 |
|------|---------|------|
| 第1天 | 修复 TypeScript 类型 `intent: string` | 测试通过 |
| 第2天 | 添加日志 `log_debug_message` | 确认前端收到 |
| 第3天 | 修改渲染条件 `{msg.phase === 'analyzing'\|\|...}` | 测试通过 |
| 第4天 | 添加 `updatePhase` + `return` | 测试通过 |
| 第5天 | 单元测试+集成测试 61个全部通过 | **仍不工作** |

### 为什么这么久没解决？

1. **测试能力不足**：桌面应用不能直接查看 console.log
2. **验证不充分**：依赖用户手动测试
3. **方向可能错误**：可能一直在修改正确的代码
4. **缺乏系统性分析**：没有完整的流程图

### 文档记录

- `docs/P0问题记录_意图搜索off_topic流程不工作.md`

---

## 2026-07-29 03:30 - M20 批量向量化功能完成

### 完成内容

1. **看板页 KanbanView.tsx**
   - 工具栏添加「向量化全部」按钮
   - 批量操作栏添加「向量化选中」按钮
   - 添加 vectorizing 状态显示

2. **设置页面 SettingsDialog.tsx**
   - 添加 VSS 扩展路径配置项
   - 支持自定义 sqlite-vss 扩展路径

3. **API search.ts**
   - 添加 rebuildEmbeddings 函数
   - 添加 generateProjectEmbedding 函数

### 验证结果

| 检查项 | 结果 |
|--------|------|
| TypeScript typecheck | ✅ 通过 |
| Tauri build | ✅ 成功 |
| Previous 目录 | ✅ 已更新 |

### Git Commit

```
feat(M20): 添加批量向量化功能
```

### M20 完成状态

| 功能 | 状态 |
|------|------|
| 看板页批量向量化按钮 | ✅ 完成 |
| sqlite-vss 扩展路径设置 | ✅ 完成 |

---

## 2026-07-29 02:30 - 文档提交完成

### 提交记录

```
65bcab5 docs: 更新原型和需求确认文档
f2bc091 docs: 更新各类实施计划和设计文档
b310b2a docs: 添加 M19 其他常用网站采集扩展文档
180157e chore: 依赖版本更新
df31796 chore: 后端模块增强和修复
626d427 refactor: 内容过滤和翻译模块增强
bc27b57 feat: 搜索和雷达插件模块增强
2ecd299 feat: SearchView 重构 + RadarInbox 增强
534bff1 docs: 更新 M18/M19/M20 需求文档
e44dceb fix(M18): 修复仓库切换后向量搜索失效问题
65635b9 fix(M18): SearchView 状态显示修复
```

### 本次会话工作完成

1. **M18 Task 10（P1）**：SearchView 状态显示修复
2. **M18 Task 8（P0）**：仓库切换后向量搜索失效修复
3. **文档提交**：所有变更已提交

### 剩余未跟踪文件

- ArticleGenerationPanel.tsx（新功能）
- InitWizard.tsx（初始化向导）
- NavigationDialog.tsx（导航对话框）
- article.rs / article_cmd.rs（文章生成）
- prompts/ / services/（LLM 服务）

---

## 2026-07-29 01:30 - M18 Task 10 问题排查（P1）开发完成

### 完成内容

1. **SearchView.tsx 状态显示修复**
   - 添加 `STATUS_CONFIG` 常量
   - 第 412 行：`project.lifecycle_status` → `STATUS_CONFIG[...].label`

2. **UI 显示检查**
   - ✅ `common.loading` - i18n 翻译已存在且正确
   - ✅ HomePage.tsx - 已使用 statusMap
   - ✅ ListView.tsx - 已使用 statusText 函数

3. **构建验证**
   - ✅ TypeScript typecheck 通过
   - ✅ Tauri build 成功
   - ✅ Previous 目录已更新

### 修改文件

- `src/components/SearchView.tsx` - 添加状态映射，修复显示

### 下一步

继续 M18 其他任务开发：
- P0 问题：仓库切换后搜索失效
- P1 问题：意图搜索新功能

---

## 2026-07-28 23:45 - M19 功能扩展 + M18 分类升级

### 完成内容

1. **M19 新增功能**
   - 广告学习功能（Task 7-10，9h）
   - 定时采集功能（Task 11-15，11h）
   - 总工时更新：38h

2. **M18 四级分类扩展**
   - 从 3 级扩展到 4 级分类
   - 新增 8 大领域：技术、学术、商业、创意、教育、生活、公共、工业
   - 统计：8 个一级、29 个二级、85 个三级、250+ 个四级

### 文档更新

- `docs/M19其他常用网站采集扩展.md` - 新增广告学习 + 定时采集
- `CURRENT-TASK.md` - 四级分类结构
- `工作进展记录.md` - M18/M19 进展

### 待继续

- M18 智能意图搜索实施
- M19 平台适配器开发

---

## 2026-07-28 22:20 - M18 智能意图搜索需求定版

### 完成内容

1. **核心流程设计**
   - 意图分析 → 追问确认 → 并行搜索 → 智能推荐
   - 任意阶段可追问

2. **文档更新**
   - 需求确认文档.md §10 → M18 智能意图搜索
   - 详细设计说明书.md §4.1.10 → M18 智能搜索
   - 智能意图搜索流程设计.md → 临时设计文档

### 待开发

M18 智能意图搜索（20h 工时）

---

## 2026-07-28 12:25 - M17 Bug 修复（第四轮）

### 修复内容

1. **向量模型自定义输入**
   - 获取模型后选择"手动输入"时显示自定义输入框
   - 与大模型设置页面保持一致

### 状态

✅ 构建成功，Previous 已更新

---

## 2026-07-28 12:15 - M17 Bug 修复（第三轮）

### 修复内容

1. **HomePage placeholder 修复**
   - 问题：使用了不存在的翻译 key `home.searchPlaceholder`
   - 修复：改为使用正确的 `home.placeholder`

2. **SearchView 主题修复**
   - 问题：使用固定的 slate 颜色，不支持亮/暗主题切换
   - 修复：改用 gray 颜色方案 + dark: 前缀，与项目其他组件一致

3. **翻译补全**
   - 添加缺失的翻译 key（中英文）

### 状态

✅ 构建成功，Previous 已更新

---

## 2026-07-28 11:34 - M17 开发完成

### 完成内容

1. **Task 1: SearchView 组件** ✅
   - ChatGPT 风格搜索界面
   - 打字动画指示器
   - 搜索历史侧栏
   - ProjectCard 组件

2. **Task 2: 最近使用侧栏** ✅
   - 调用 getRecentProjects API
   - 显示最近 5 个项目

3. **Task 3: Embedding 设置页面** ✅
   - 新增"向量搜索"标签页
   - 配置项：开关、API 类型、URL、Key、模型、维度
   - 测试连接功能
   - 后端 API：get_embedding_settings, save_embedding_settings

4. **后端更新** ✅
   - SearchResult 结构更新为双轨（local_results + web_results）
   - ProjectMatch 字段更新（name, url, stars, forks 等）

5. **构建成功** ✅
   - Previous/os-compass.exe 已更新

### 状态

- Task 1-3 完成
- Task 4-8 待完成（sqlite-vss 向量模块）
- Task 9 进行中

---

## 2026-07-27 17:00 - M17 开始开发

### 会话主题：意图搜索向量搜索升级 + 界面重构

### 完成的工作

1. **需求升级**
   - 意图搜索从"三层优先级搜索"改为"双轨搜索"
   - 第一轨：本地向量搜索（sqlite-vss）
   - 第二轨：LLM 联网搜索

2. **界面重构**
   - 参考 ChatGPT + 首页风格
   - 更新原型 `search-view.html`
   - 添加打字动画、项目卡片、来源标签

3. **文档更新**
   - `M17实施计划.md`：新增布局设计、组件结构、样式规范
   - `需求确认文档.md` §10：新增界面设计章节
   - `详细设计说明书.md` §4.1.10：新增 SearchView 前端组件设计
   - `数据库设计说明书.md`：新增 project_embeddings 向量表
   - `概要设计说明书.md`：插件描述更新
   - `API接口设计说明书.md`：接口变更

### 里程碑归属

| 里程碑 | 内容 |
|--------|------|
| M13 | 基础意图搜索（关键词匹配 + LLM 联网） |
| **M17** | 向量搜索增强（sqlite-vss + ChatGPT 风格界面） |

### M17 任务清单

| Task | 内容 | 预估工时 |
|------|------|----------|
| Task 1 | SearchView 组件 | 4h |
| Task 2 | 意图搜索 API 调用 | 1h |
| Task 3 | 最近使用侧栏 | 1h |
| Task 4 | sqlite-vss 扩展集成 | 4h |
| Task 5 | embedding.rs 向量模块 | 3h |
| Task 6 | 向量生成时机 | 2h |
| Task 7 | LLM 联网搜索 | 2h |
| Task 8 | 测试 + 构建 | 2h |
| **总计** | | **19h** |

### 评估结果

**具备开发条件**：✅ 是

**待确认事项**：
1. sqlite-vss 扩展 Tauri 打包方式
2. Embedding API 选择（OpenAI/本地模型）
3. 现有项目向量生成策略

### 提交记录

无（仅文档更新）

---

## 2026-07-18 14:20 - 情报雷达功能完善

### 完成的工作

1. **Telegram 采集优化**
   - 修复消息时间顺序（HTML 中旧 → 新）
   - 使用消息 ID 分页 `?before=xxx`
   - 时间范围过滤：1天/7天/30天/全部
   - 本地时间显示（UTC → Local）

2. **扫描逻辑修复**
   - 保留已有数据，只采集新内容
   - 修复按钮状态（扫描完成后正确停止旋转）
   - 时间范围只影响采集，不影响显示

3. **收藏功能重构**
   - "导入" → "收藏"
   - 移除链接平台限制，所有条目都可收藏
   - 收藏只是标记状态，不创建项目

4. **UI 修复**
   - 选项卡计数正确显示
   - 移除"不感兴趣"按钮和标签页
   - 移除信息源表单中的"平台"字段
   - 添加 URL 类型验证

### 提交记录
- `e01815c` - 修复选项卡计数
- `bcd7b31` - 重构收藏功能
- `2a30b76` - 修复扫描逻辑
- `f6b5886` - 修复 Telegram 时间顺序
- `76da598` - 时间范围过滤
- `31de09b` - 添加时间范围选择框

---

## 2026-07-17 21:40

### 会话主题：修复 SOCKS5 代理支持

**根本原因**：
- 数据库代理协议是 `socks5`
- 但代码生成 URL 时用 `http://` 前缀
- 导致 reqwest 无法识别 SOCKS5 代理

**修复**：
- 根据 `source.proxy_protocol` 动态生成代理 URL
- 现在会生成 `socks5://127.0.0.1:8964`

**提交**：9a0748e

---

## 2026-07-17 17:30

### 会话主题：修复代理认证 + Telegram 解析

**问题**：
- Telegram 无法获取数据
- 代理配置没有处理用户名密码认证

**修复**：
- 修复 radar.rs 代理配置，支持带认证的代理
- 用 Node.js 测试验证了解析逻辑正确

**提交**：117af79

**注意**：中国大陆访问 Telegram 需要代理，确保代理配置正确

---

## 2026-07-17 03:45

### 会话主题：修复 Telegram 乱码

**问题**：获取的数据是乱码，没有解码 HTML 实体

**修复**：
- 添加 HTML 实体解码
- 解码 numeric entities
- 正确提取消息文本块

**提交**：7b969a0

---

## 2026-07-17 03:15

### 会话主题：修复 Telegram 正则表达式解析

**问题**：正则表达式不匹配实际 HTML 结构

**修复**：改进正则表达式，使用 fallback 方法直接提取文本块

**提交**：c2549bb

---

## 2026-07-17 03:00

### 会话主题：修复 radar 去重逻辑错误

**问题**：
1. 错误过滤 GitHub/Gitee
2. 使用 url_hash 去重，但 url 可能为空

**修复**：
1. 移除 GitHub/Gitee 过滤
2. 使用 content hash 作为唯一标识

**提交**：5d27f8b

---

## 2026-07-17 02:50

### 会话主题：Telegram 适配器 HTML 解析修复

**问题**：代理可访问，但解析逻辑不正确

**修复**：
- 分析 Telegram HTML 结构
- 正确提取 `tgme_widget_message_wrap` 中的消息
- 添加 fallback 解析方式

**提交**：712704f

---

## 2026-07-17 02:30

### 会话主题：修复 Telegram 采集理解错误

**问题**：
- 错误理解：只采集包含 GitHub/Gitee 链接的消息
- 正确理解：采集所有信息，后续再根据扩展配置分析

**修复**：
- 移除 GitHub/Gitee 过滤
- 采集所有消息内容
- 提取所有 URL

**提交**：547d140

---

## 2026-07-17 02:15

### 会话主题：情报雷达并发扫描 + Telegram 采集完善

**修复**：

1. **并发扫描问题**
   - 原设计：`tokio::spawn` 并发执行，因 `rusqlite::Connection` 不是 `Send` 导致编译失败
   - 修复方案：顺序执行，每个信息源之间延迟 1 秒

2. **Telegram 采集完善**
   - 从页面 HTML 中提取消息内容
   - 过滤包含 GitHub/Gitee 链接的消息
   - 从消息块中提取标题、链接、发布时间
   - 两种提取模式：结构化提取 + 直接 HTML 提取

3. **数据存储**
   - 信息源（radar_sources）→ 系统库 plugin_config.db
   - 采集数据（radar_items）→ 仓库 plugin_radar.db（父目录）

**提交**：6f54c55

---

## 2026-07-17 01:42

### 会话主题：情报雷达 Telegram 适配器修复

**问题**：
1. Telegram 采集无数据 - 之前只提取链接，没有提取消息内容
2. 代理配置没有读取信息源自己的设置
3. URL 提取模式硬编码了 GitLab，应该从 source_plugins.url_patterns 读取
4. 没有编辑/删除信息源功能
5. 错误信息没有显示

**系统性修复**：

1. **数据架构**
   - 信息源（radar_sources）→ 系统库 plugin_config.db
   - 采集数据（radar_items）→ 仓库 plugin_radar.db

2. **代理配置** - radar.rs
   - 优先读取信息源自己的代理配置
   - 其次读取全局代理

3. **Telegram 适配器** - telegram_adapter.rs
   - 提取消息内容，不只是链接
   - 4种解析方式（Widget/Legacy/Simple/Text）
   - 从 source_plugins.url_patterns 读取支持的平台
   - 请求延迟、cookies 保持

4. **前端 UI** - RadarInbox.tsx
   - 添加编辑按钮
   - 添加删除按钮
   - 错误状态显示

**提交记录**：
- ebfc24b: radar_sources to system DB
- a924ac5: radar proxy config + edit/delete UI
- 170d185: telegram adapter browser UA
- ac8315b: telegram session handling
- bdd5a7a: telegram extracts message content
- 200c3a8: telegram multi-parser extraction
- d0436ba: telegram reads URL patterns from source_plugins

---

## 2026-07-16 18:01

### 会话主题：情报雷达数据架构重构

**问题**：用户反馈情报雷达采集后无数据

**需求**：信息源放系统库，采集数据放仓库

**修复**：
1. 系统库 plugin_config.db 添加 radar_sources 表
2. PLUGIN_CONFIG_DB 添加 CRUD 方法
3. radar_cmd.rs 全部改用 PLUGIN_CONFIG_DB
4. radar.rs 扫描时从系统库读取信息源
5. vault 数据库移除 radar_sources 表

**构建**：成功

**提交**：ebfc24b

---

## 2026-07-15 下午 (18:00)

### 会话主题：修复加密密钥架构问题

**问题**：切换仓库后 API Key 无法解密

**根因**：存在两个不同的密钥
- `com.administrator.os-compass\.cryptokey`（系统级）
- `vault\.cryptokey`（vault 密钥）

`system_settings.db` 使用系统级密钥加密，但启动时加载 vault 密钥。

**修复**：
1. 始终使用系统级密钥（`app_data_dir/.cryptokey`）
2. 不再跟随 vault 切换改变密钥

**提交**：bb8c9da fix: use system-level crypto key for all encrypted data

---

## 2026-07-15 下午 (17:45)

### 会话主题：修复切换仓库导致 API Key 无法解密问题

**问题**：切换仓库后 LLM API Key 无法加载

**根因**：`open_vault` 函数在切换仓库时会用新 vault 的加密密钥重新初始化加密服务

**修复**：移除 `open_vault` 中的 `init_crypto` 调用

**提交**：51805fe fix: prevent crypto re-initialization on vault switch

---

## 2026-07-15 下午 (17:35)

### 会话主题：README 显示问题修复

**问题**：
1. README 显示格式混乱，markdown 解析不彻底
2. 部分图片加载不出来，怀疑是相对路径问题

**解决方案**：
1. 给 `MarkdownRenderer` 添加 `baseUrl` prop
2. 新增 `resolveRelativeImageUrls()` 函数处理相对路径
3. 将 GitHub URL 转换为 Raw URL
4. 启用 marked 的 `breaks` 和 `gfm` 选项

**提交**：5607e13 fix: improve MarkdownRenderer to handle relative image URLs

---

## 2026-07-15 下午 (17:19)

### 会话主题：OpenHarness 分析卡住问题定位

**问题**：OpenHarness 项目点击"重新分析"后长时间无响应（超过5分钟）

**排查结果**：
1. **后端验证** - 创建独立测试程序 `debug_full` 模拟 `analyze_project` 流程
   - ✅ API Key 解密正常：`sk-o4KSSPt7QqGI18ozxCRoy5Jc9wRAKmKlv4kqpd0sKUH7tafJ`
   - ✅ HTTP 请求成功：8秒返回 200 OK
   - ✅ JSON 解析成功：数据完整

2. **后端代码改进**：
   - LLM 超时从 60s 增加到 180s
   - 添加详细错误分类（超时/连接/401/429/500）
   - 添加 `debug_llm_status` 命令检查配置
   - 添加 `test_llm_direct` 命令直接测试

3. **前端调试功能**：
   - 在 AI 报告区域添加"调试"按钮
   - 调用 `test_llm_direct` 并显示结果

**结论**：
- 后端 `analyze_project` 逻辑完全正常
- 问题疑似在前端 UI 状态更新或 Tauri invoke 调用层
- 用户确认调试功能可用

**提交**：4a0b750 feat: add LLM debug commands and improve error handling

---

## 2026-07-15 上午 (11:23)

### 会话主题：LLM 代理问题定位

**问题**：重新分析 OpenHarness 项目时长时间无响应

**定位过程**：
1. 测试直连 `api.sfkey.cn` → ❌ Connection timed out
2. 使用 `github_proxy` (127.0.0.1:8964) 测试 → ✅ 返回 401（网络通了）

**结论**：
- LLM API 需要代理才能访问
- 当前 `llm_proxy_enabled = 0`（未启用）
- 建议用户在 LLM 设置中启用代理，或将 `github_proxy` 作为全局代理

### 用户要求
- 定位问题时不要擅自修改代码
- 每次会话要及时更新会话内容和进度记录

---

## 2026-07-15 凌晨 (00:12)

- 会话主题：V2.0 架构严重错误 - source_plugins 位置搞反

- 用户反馈问题：
  1. 程序启动报错 "Database not initialized"
  2. 扩展配置无法加载

- 根本原因：
  之前完全搞混了系统库和仓库库的位置！
  - source_plugins 应该放在 **系统库** system_settings.db
  - 之前错误地放在了 **仓库库** os_compass.db

- 修复操作：
  1. 在 system_db.rs 中添加 source_plugins 表定义和种子数据
  2. 修改 extensions.rs 使用 system_db 而不是 DATABASE
  3. 从 db.rs 中移除 source_plugins 相关代码
  4. 同步更新 scripts/init_schema.sql 和 Previous/scripts/init_schema.sql

- V2.0 正确架构：
  | 数据 | 存储位置 |
  |------|----------|
  | source_plugins | **系统库** system_settings.db |
  | system_variables | **系统库** system_settings.db |
  | app_settings | **系统库** system_settings.db |
  | projects 等业务数据 | **仓库库** os_compass.db |

- 提交：2b0ee6e fix: move source_plugins to system_db (V2.0 architecture)

- 状态：**V2.0 架构修复完成，构建成功，Previous 已更新**

## 2026-07-14 晚间 (22:57)

- 会话主题：修复数据库初始化问题 + 扩展配置丢失 Bug
- 用户反馈问题：
  1. LLM 配置保存后重启丢失
  2. 扩展配置（source_plugins）被清空，无法从 GitHub/Gitee 采集项目

- 问题根因：
  1. `scripts/init_schema.sql` 和 `Previous/scripts/init_schema.sql` 缺少 `url_patterns` 字段
  2. 导致 source_plugins 表结构与代码定义不一致
  3. INSERT OR IGNORE 语句无法正确执行

- 修复操作：
  1. 更新 `scripts/init_schema.sql`：
     - 添加 `url_patterns TEXT` 列到 source_plugins 表
     - 更新 INSERT 语句添加 url_patterns 字段（github: ["github.com", "www.github.com"]，gitee: ["gitee.com", "www.gitee.com"]，crawler: []）
  2. 同步更新 `Previous/scripts/init_schema.sql`

- 构建验证：
  - Rust 编译通过（40 warnings）
  - 前端类型检查通过
  - Release 构建成功

- 更新 Previous 目录：
  - os-compass.exe (30693290 bytes)
  - WebView2Loader.dll
  - scripts/init_schema.sql

- 状态：**Bug 修复完成，构建成功**
- 主要工作：
  - 检查项目文档状态（里程碑、WBS、PRD、需求确认、测试文档等）
  - 创建 `docs/开发计划清单.md`：完整的功能改造和功能开发清单
  - 更新 `M13实施计划.md`：
    - 新增 Task 18：雷达适配器扩展机制
    - 新增 Task 19：固定网页监控适配器
    - 新增 Task 20：雷达 UI 适配器选择优化
    - 新增 Task 21：雷达链接导入增强
  - 更新 `里程碑和开发计划.md`：新增雷达适配器扩展任务（工时 22.6d）
  - 更新 `WBS分解.md`：新增雷达适配器扩展 WBS（8 个功能点）
  - 更新 `CURRENT-TASK.md`：完整开发计划

- V2.0 配置迁移改造评估：
  | 改造模块 | 工作量 |
  |----------|--------|
  | 后端 settings 表访问层 | 高 |
  | 后端 LLM/翻译/下载模块 | 高 |
  | 后端 扩展 Token 模块 | 高 |
  | 前端 settingsStore | 高 |
  | 前端 SettingsDialog | 高 |

- 雷达适配器扩展：
  | 功能 | 优先级 |
  |------|--------|
  | 固定网页监控 | **P0** |
  | 适配器注册表 | P1 |
  | 微博/Twitter/公众号监控 | P2 |

- 工时汇总：
  | 阶段 | 工时 |
  |------|------|
  | V2.0 配置迁移 | 9.5d |
  | M13 剩余任务 | 1.5d |
  | 雷达适配器扩展 | 4d |
  | **总计** | **15d** |

- 详细拆解 V2.0 配置迁移（Task 0.1-0.6）：
  - 后端改造：settings.rs, variables.rs, llm.rs, translate.rs, crawler.rs, radar.rs, lib.rs
  - 前端无需改动（API 层调用后端）
  - 含迁移器 + 回归测试

- 状态：**文档更新完成，等待开发**

---

## 2026-07-13 下午

- 会话主题：M13 插件系统前端 UI 重构 + 文档完善
- 主要工作：
  - 补充 i18n 翻译键（pluginConfig、storage、pluginLevel 命名空间）
  - 重构 Settings → 功能插件 tab UI（PluginCard、PluginConfigModal）
  - 新增 get_vault_dir 命令
  - 更新 M13实施计划、详细设计说明书、测试用例文档
- 构建成功，Previous 已更新
- 状态：**待用户测试验证**
- 下一步：测试三层存储、仓库切换

## 2026-07-13 下午

- 会话主题：M13 插件架构完善 — 存储架构重构 + 设计文档更新
- 主要工作：
  - 发现问题：插件配置存储在主库导致功能管理页面显示"暂无插件"
  - 讨论设计：插件分类（系统级/仓库级）、操作域层次（插件域/仓库域/业务域）
  - 实现：新增 `plugin_config_db.rs`，插件配置存储在 `${AppData}/.os-compass/plugins.db`
  - 更新设计文档：
    - `插件接入规范.md`：补充操作域层次、PluginContext 结构
    - `API接口设计说明书.md`：补充存储架构、PluginContext
    - `数据库设计说明书.md`：补充 plugins.db 表结构
    - `原型功能对照表.md`：M13 页面状态更新
  - 更新 WBS 和里程碑文档：
    - 新增 Task 14-17（架构完善 + 前端重构）
  - 更新测试文档：
    - 新增 TC-M13-006/007（操作域验证）
- 变更文件：plugin_config_db.rs, plugin_manager.rs, lib.rs, 4份设计文档, WBS, 里程碑
- 新增 commit：feat(M13): implement plugin config storage with system-level and vault-level separation
- 状态：**设计文档完善完成，前端 UI 重构待完成**
- 下一步：重构前端 Settings → 功能插件 tab，按新原型实现

## 2026-07-12 10:00

- 会话主题：M13 Phase 2 开发 — Tasks 1-8 完成
- 主要工作：
  - Task 1-6：FeaturePlugin trait、DB表、PluginManager、SourceAdapter、LLM解析器
  - Task 7: RadarPlugin 完整实现（init_radar_db、scan_sources、URL去重、LLM解析、8个雷达命令）
  - Task 8: SearchPlugin 完整实现（init_search_db、三层搜索、10个搜索命令）
- 变更文件：feature_plugin.rs, db.rs, plugin_manager.rs, plugin_cmd.rs, radar_cmd.rs, search_cmd.rs, source_engine/*, radar.rs, search.rs
- 新增 commit：8 个 feat(M13) commit
- 验证：Rust 编译 ✅，TypeScript 类型 ✅，vitest 18/18 ✅
- 状态：Tasks 1-8 完成，下一步 Task 9 启动初始化 + 前端 API
- 下一步：继续 Task 9-13

## 2026-07-12 10:00

- 会话主题：M13 Phase 2 开发 — Tasks 1-6 完成

## 2026-07-11 15:07

- 会话主题：Phase 1 打磨收尾 — 残余 i18n + 暗色模式全覆盖
- 主要工作：
  - VaultDialog 33 个残余中文字符串替换为 t() 调用
  - ProjectDetailDialog 21 个残余字符串国际化（含后台任务类型字符串）
  - SettingsDialog 5 个残余字符串、ClonePanel 2 个残余字符串
  - 翻译键大幅扩展：vault 命名空间 +30 键，detail 命名空间 +15 键
  - 暗色模式全覆盖：18 个组件添加 1026 个 dark: 变体类
  - 构建成功，产物已复制到 Previous/
- 变更文件：16 个 .tsx（dark:）+ 4 个 .tsx（i18n）+ zh.json + en.json
- 状态：**Phase 1 打磨收尾完成，i18n 和暗色模式全覆盖**
- 下一步：等待用户指定（Phase 2 M13 意图搜索或用户测试）

## 2026-07-11 13:56

- 会话主题：工程化改进 — Git 仓库初始化 + 测试框架 + CI
- 主要工作：
  - 创建 .gitignore，初始化 Git 仓库，首次提交全部源码
  - 移除 crawler/webtomd 内嵌 .git，改为直接纳入主仓库
  - 安装 vitest + @testing-library/react + jsdom，创建 vitest.config.ts
  - 新增 18 个前端测试（types.test.ts + i18n.test.ts）
  - i18n 测试发现 en.json 缺失 detail.aiAnalysis 键，已修复
  - 创建 GitHub Actions CI（.github/workflows/ci.yml）
  - 创建 scripts/pre-commit.ps1 本地提交前检查脚本
  - AGENTS.md 新增开发命令和 Git 规范
- 变更文件：.gitignore, AGENTS.md, package.json, en.json, vitest.config.ts, types.test.ts, i18n.test.ts, ci.yml, pre-commit.ps1
- 状态：**工程化改进完成，Git 仓库就绪**
- 下一步：等待用户指定（Phase 2 M13 意图搜索或继续打磨）

## 2026-07-11 10:46

- 会话主题：Phase 1 打磨 — github_token 防御性修复 + i18n 覆盖率提升
- 主要工作：
  - 根因分析：`cleanup_undecryptable_secrets()` / `get_variable_value_internal()` / `get_setting()` 在解密失败时静默清空密文 → 密钥临时不匹配导致数据永久丢失
  - 修复 3 处清空逻辑为保留+警告日志（variables.rs、settings.rs）
  - 启动时添加加密服务验证步骤，legacy 路径补调 cleanup（lib.rs）
  - 11 个组件接入 useTranslation（MoveCategoryDialog、ManualAddDialog、ImportConfirmDialog、MarkdownEditor、ClonePanel、ReleasesPanel、ImportModal、VaultDialog、ProjectDetailDialog、SettingsDialog 补齐）
  - 修复 en.json 重复 detail 块问题
  - 翻译资源大幅扩展（zh.json/en.json 新增 markdownEditor 命名空间，扩展 clone/releases/import/manualAdd/moveCategory/importConfirm/settings）
- 变更文件：variables.rs, settings.rs, lib.rs, 10 个 .tsx 组件, zh.json, en.json
- 状态：**Phase 1 打磨第一批完成**
- 下一步：等待用户指定（继续打磨或进入 Phase 2）

## 2026-07-06 02:35

- 会话主题：克隆功能设计文档更新
- 主要工作：
  - 与用户确认克隆功能设计：**独立 project_clone 表存储**
  - 用户明确：克隆地址以项目为维度记录，新建表不影响现有功能
  - 更新设计系统原型：`design-system/public/project-detail.html` 克隆标签页
  - 更新需求确认文档：#45/#50，新增 4.23 克隆功能重构
  - 更新概要设计说明书：3.5 本地联动模块
  - 更新详细设计说明书：4.4 本地联动模块
  - 更新数据库设计说明书：新增 3.10 project_clone 表
  - 更新 API 接口设计说明书：Settings 类型 + 新增 get_clone_settings/save_clone_settings/execute_clone 命令
  - 更新测试用例文档：TC-019~TC-025 克隆功能测试用例
  - 更新 patterns/README.md：新增 ClonePanel 组件说明
  - 更新 components/README.md：新增 4.1 克隆标签页设计章节
- 变更文件：6份设计文档 + design-system原型
- 状态：**克隆功能设计文档更新完成**
- 下一步：实现克隆功能后端和前端

## 2026-07-05 16:00

- 会话主题：翻译引擎设置 + Markdown 外链 + 看板拖拽重构 + 状态即时更新
- 主要工作：
  - 后端 AppSettings 添加 translate_engine 等 5 个字段
  - 编辑保存不关闭详情页（onDeleted 回调 + 本地 state）
  - Markdown 外链用 opener 插件在系统浏览器打开
  - 看板拖拽从 HTML5 Drag API 重构为 Pointer Events
  - 修复 update_project_status 参数名 lifecycle_status → lifecycleStatus
  - 详情页状态切换即时更新
  - 文档全面同步（需求确认项 #46~#49，测试用例 TC-018T~TC-018W）
- 变更文件：settings.rs, SettingsDialog.tsx, App.tsx, ProjectDetailDialog.tsx, MarkdownRenderer.tsx, KanbanView.tsx, api/index.ts, 3份文档
- 状态：**看板拖拽和多个小问题修复完成**

## 2026-07-05 15:09

- 会话主题：翻译引擎设置修复 + 编辑保存不关闭详情页 + Markdown 外链浏览器打开
- 主要工作：
  - 后端 AppSettings 添加 translate_engine 等 5 个字段
  - 编辑保存后不关闭详情页（新增 onDeleted 回调）
  - Markdown 外链用 opener 插件在系统浏览器打开
- 变更文件：settings.rs, App.tsx, ProjectDetailDialog.tsx, MarkdownRenderer.tsx
- 状态：**三个小问题修复完成**

## 2026-07-05 13:52

- 会话主题：项目详情页分类显示 + 编辑功能 + 物理删除 + 克隆移除 + 本地时间修复
- 主要工作：
  - 修复本地时间：INSERT 显式传入 datetime('now', 'localtime')
  - 项目分类显示：全路径（parent_id 递归），位于状态栏上方
  - 项目编辑功能：名称+分类可编辑，名称唯一性校验
  - 项目物理删除：确认弹框，修复 delete_project 命令未注册
  - 移除两处克隆按钮
  - 修复分类下拉框重复"未分类"
  - 原型 project-detail.html 同步更新
  - 需求文档新增确认项 #42~#45 和 4.20~4.23 节
- 变更文件：project.rs, lib.rs, db.rs, 6个commands文件, ProjectDetailDialog.tsx, project-detail.html, 需求确认文档.md
- 状态：**项目详情页分类/编辑/删除功能完成**

## 2026-07-05 01:30

- 会话主题：翻译稳定性彻底修复 + 文档全面同步
- 主要工作：
  - 修复后台任务未翻译 AI 分析结果（新增 translate_ai_field）
  - 明确翻译引擎使用范围：概览用 LLM，README 用自动选择
  - 新增 translate_text_with_llm 函数和命令
  - LLM 请求添加 60 秒超时
  - 后台翻译重试 3 次间隔 3 秒，字段间延迟 500ms
  - 前端自动补译机制（all done 时检查缺失字段）
  - 手动补译（translateAndSave 跳过已有翻译）
  - AI 分析 README 预处理（过滤图片、按场景截取）
  - 全面更新需求/设计/API/测试文档
- 变更文件：translate.rs, translate_cmd.rs, import_cmd.rs, llm.rs, ai.rs, runbook_cmd.rs, lib.rs, ProjectDetailDialog.tsx, 6份文档
- 状态：**翻译稳定性问题彻底修复，文档全面同步**

## 2026-07-04 22:42

- 会话主题：AI 分析的 README 预处理优化
- 主要工作：
  - 新增 README 预处理函数（过滤图片、按字符截取、合并空行）
  - AI 分析用 6000 字符，标签生成用 2000 字符，RUNBOOK 用 12000 字符
  - 修复原字节截取导致中文乱码问题
  - 仅用于 AI 分析输入，不影响 README 模块展示和翻译
- 变更文件：llm.rs, ai.rs, runbook_cmd.rs
- 状态：**README 预处理优化完成**

## 2026-07-04 21:57

- 会话主题：导入异步化问题修复 + 后台任务提醒 + 文档同步
- 主要工作：
  - 修复后台任务被取消问题：用 `tauri::async_runtime::spawn` 脱离前端生命周期
  - 优化导入页面关闭逻辑：入库成功自动关闭，失败停留显示统计
  - 项目详情页添加后台任务进度提醒（蓝色标识 + 具体任务列表）
  - 同步更新需求确认文档和概要设计说明书
- 变更文件：import_cmd.rs, ImportModal.tsx, ProjectDetailDialog.tsx, 需求确认文档.md, 概要设计说明书.md
- 状态：**导入异步化修复完成，后台任务独立运行**

## 2026-07-04 19:22

- 会话主题：导入URL唯一性校验 + 导入流程异步化
- 主要工作：
  - 需求文档更新：新增确认项 #37/#38，新增 4.15/4.16 节
  - 后端 URL 规范化和重复检查（normalize_url、check_url_exists）
  - 后端新增 post_import_tasks 异步任务命令（AI分析、翻译、标签）
  - 后端通过 Tauri 事件通知前端任务进度
  - 前端处理重复提示，提供"查看项目"按钮
  - 前端 ProjectDetailDialog 监听任务进度事件，自动刷新
- 变更文件：import_cmd.rs, lib.rs, api/index.ts, ImportModal.tsx, ProjectDetailDialog.tsx, App.tsx
- 状态：**导入唯一性校验和异步化完成**

## 2026-07-04 16:23

- 会话主题：翻译功能完整链路修复
- 主要工作：
  - 定位实际数据库路径（AppData\Roaming\com.administrator.os-compass）
  - 修复翻译保存失败（改用 save_translation_cmd 逐条保存）
  - 修复翻译加载字段名不匹配（useCases → use_cases）
  - 修复 clear_translations 未清空 translations 表
  - 修复核心 Bug：AiAnalysisResult.use_cases 缺少 serde rename，导致前端 useCases 永远为 null
  - 增强 JSON 解析容错（支持数组格式、中文字段名）
- 变更文件：ai.rs, project_extra.rs, lib.rs, ProjectDetailDialog.tsx
- 状态：**翻译功能完整链路修复完成**

## 2026-06-29 22:03

- 会话主题：详情弹窗 + 数据加密 ✅
- 主要成果：
  - 项目详情弹窗（概览/AI分析/笔记）
  - AES-256-GCM 加密模块
  - 密钥生成/派生
- 验证：应用启动成功 PID 30580
- 状态：**基本功能完善**

## 2026-06-29 20:35

- 会话主题：严格测试验证 ✅
- 验证结果：
  - TypeScript: ✅ 无错误
  - Rust Clippy: ✅ 无警告
  - 完整构建: ✅ 成功
  - 应用启动: ✅ PID 3224
- 状态：**测试通过**

## 2026-06-29 20:18

- 会话主题：前端界面完善 ✅
- 主要成果：
  - 侧边栏（分类/标签管理）
  - 设置弹窗（LLM/Token/代理）
  - 健康度评分显示
- 状态：**前端界面基本完善**
- 下一步：启动测试

## 2026-06-29 19:48

- 会话主题：M3 AI 分析模块 ✅
- 主要成果：
  - LLM 客户端（OpenAI 兼容 API）
  - 健康度评分算法
  - 项目 AI 分析命令
  - 设置存储
- 状态：**M3 AI 分析模块完成**
- 下一步：M4 分类管理 或 M5 标签系统

## 2026-06-29 19:04

- 会话主题：M2 项目导入模块 ✅
- 主要成果：
  - GitHub API 插件（获取项目信息 + README）
  - Gitee API 插件
  - 爬虫回退支持
  - 导入预览 + 确认流程
- 状态：**M2 项目导入模块完成**
- 下一步：M3 AI 分析模块（LLM 集成）

## 2026-06-29 18:30
- 主要问题：
  - MSYS2 MinGW ordinal 链接错误
  - MSVC 工具链缺失 msvcrt.lib
- 解决方案：
  1. 创建 `.cargo/config.toml` 配置 MSYS2 MinGW 链接器路径
  2. 指定 `linker = "D:\\Soft\\msys64\\mingw64\\bin\\gcc.exe"`
- 编译结果：
  - exe：`os-compass.exe` (19MB)
  - MSI + NSIS 安装包
- 状态：**阻塞已解决，编译成功**
- 下一步：恢复数据库功能（rusqlite ordinal 问题待验证）

## 2026-06-29 17:30

- 会话主题：解决 Tauri 编译阻塞
- 主要问题：
  - MSYS2 MinGW ordinal 链接错误
  - MSVC 工具链缺失 msvcrt.lib
- 尝试的解决方案：
  1. 切换 MSVC 工具链 → 失败
  2. 设置 RUSTFLAGS → 失败
  3. 手动配置 PATH → 失败
- 当前状态：**阻塞，需要安装完整的 VS C++ Build Tools**
- 下一步：用户安装 VS C++ Build Tools

## 2026-06-29 17:00

- 会话主题：Phase 1 MVP 开发
- 主要成果：
  - Phase 0 完成：环境配置、Tauri 工程初始化、依赖安装
  - 开始 Phase 1 M1 基础架构模块开发
  - 记录网络代理：`http://127.0.0.1:8964`
- 当前任务：M1 基础架构（IPC通信、数据库迁移、插件框架）
- 状态：**进行中**

## 2026-06-27 12:30

- 会话主题：验证 Tauri 环境可行性
- 主要成果：
  - 发现问题：rustup MinGW 工具链不完整，dlltool CreateProcess 失败
  - 用户安装 MSYS2 (D:\Soft\msys64)
  - 安装完整 MinGW 工具链 (binutils + gcc + winpthreads)
  - 配置 PATH 后 Tauri 编译成功
  - exe 和安装包生成正常，应用程序启动成功
- 状态：**已完成，Tauri 方案可行**
- 备注：需将 D:\Soft\msys64\ucrt64\bin 添加到系统 PATH

## 2026-06-27 02:00

- 会话主题：补充翻译功能需求文档和原型
- 主要成果：
  - 更新 `docs/概要设计说明书.md`：新增 3.2.4 翻译功能模块
  - 更新 `docs/requirements.md`：新增 5.9 翻译功能模块章节
  - 更新 `docs/需求确认文档.md`：补充确认事项
  - 更新 `import-form.html`：新增"自动翻译 README"选项
  - 更新 `project-detail.html`：README 标签页添加"重新翻译"按钮
  - 更新 `settings.html`：新增"自动翻译 AI 报告"选项
- 状态：已完成
- 备注：翻译功能需求已完整补充，原型与设计文档保持一致

## 2026-06-27 01:50

- 会话主题：补充平台插件化架构设计
- 主要成果：
  - 更新 `docs/需求确认文档.md`：新增平台插件化设计章节
  - 更新 `docs/概要设计说明书.md`：重构 3.1 项目导入模块为插件化架构
- 状态：已完成
- 备注：插件接口 + 差异化评分已定义

## 2026-06-27 00:10

- 会话主题：调整原型布局和分类管理
- 主要成果：
  - 更新 main-interface.html：左侧功能菜单（导入/搜索/视图切换/分类/设置/插件），右侧看板内容
  - 更新 list-view.html：左侧功能菜单 + 顶部筛选区（搜索、分类下拉、标签、状态）
  - 更新 category-manager.html：多层级树形结构，支持展开/折叠，显示 path、explain 字段
  - 更新 settings.html：补充完整标签页（通用/下载/LLM），LLM 配置含代理、API Key、测试连接
  - 更新 project-detail.html：6 个标签页全部实现，**用户信息标签页**含数据保险箱（敏感字段加密显示）
  - 更新 patterns/README.md：新增核心布局模式说明（左侧功能栏 + 主内容区）
  - 更新 components/README.md：补充新组件说明（CategoryTree、UserInfoPanel、ProxyForm 等）
- 状态：已完成
- 备注：
  - 看板视图不需要分类筛选
  - 列表视图需要分类筛选（树状下拉）
  - 分类管理是独立页面，支持多层级

## 2026-06-26 23:40

- 会话主题：重建设计系统文档
- 主要成果：
  - 补充 design-system/README.md（总览、目录、设计决策）
  - 补充 design-system/使用文档.md（使用指南、组件规范、FAQ）
  - 补充 design-system/components/README.md（12+ 组件规范）
  - 补充 design-system/patterns/README.md（9 种页面模式）
  - 补充 design-system/tokens/README.md（设计令牌完整定义）
  - 确认 HTML 原型文件基本可用（index, main-interface, project-detail, settings, category-manager, import-form）
- 状态：已完成
- 备注：HTML 原型内容可能需后续根据实际需求调整

## 2026-06-27 01:30

- 会话主题：补充语言翻译和 AI 提炼功能
- 主要成果：
  - URL 导入流程图补充语言检测和翻译步骤
  - 概览标签页添加 AI 一句话总结、适用场景
  - README 标签页说明翻译 + 提炼、简化
  - 设置中心补充语言相关设置
  - project-detail.html 原型已更新
- 状态：已完成

## 2026-06-26 23:10

- 会话主题：PRD 梳理与工作规范确认
- 主要成果：
  - 确认 AGENTS.md 结构正确
  - 确认暂不初始化 Git 仓库
  - 开始梳理 PRD 文档，确认为唯一完整设计文档
- 状态：进行中
- 备注：
  - PRD (docs/产品需求文档 (PRD).md) 是唯一完整文档
  - design-system/public 下的 HTML 原型需重建，内容可能已过时
  - Windows 工具链阻塞问题待解决

## 2026-06-26 22:45

- 会话主题：恢复因 `create-tauri-app --force` 误操作丢失的项目文件
- 主要成果：
  - 重新创建根目录关键文件：`AGENTS.md`、`CONTEXT.md`、`README.md`、`SESSION-LOG.md`
  - 重新创建 `docs/` 目录并恢复核心文档（含连续性机制、检查清单、工作进展记录、WBS、里程碑计划、PRD/概要/详细/数据库/API/架构/安全等框架）
  - 创建 `checkpoints/` 目录并生成首个检查点
  - 创建 `design-system/` 目录结构及占位原型文件
- 状态：已完成
- 备注：
  - 原 `docs/` 下的 PRD、概要设计、详细设计、数据库设计、API接口设计、技术架构设计、安全工程方法论等详细文档内容需要重新生成或从用户处获取补充。
  - 原 `design-system/` 下的 HTML 原型文件需要重新设计或从用户处获取补充。

## 2026-06-26 22:30

- 会话主题：尝试 GNU toolchain 初始化 Tauri 工程
- 主要成果：
  - 切换 Rust toolchain 到 `stable-x86_64-pc-windows-gnu`
  - 执行 `npx create-tauri-app@latest . --manager pnpm --template react-ts --yes --tauri-version 2 --force`
  - 配置 pnpm 代理和 `node-linker: hoisted`
  - 成功运行 `pnpm install`
- 异常：
  - `pnpm tauri dev` 编译 Rust 时失败：`dlltool.exe` 未找到
  - 将 rustup self-contained 目录加入 PATH 后，`dlltool` 报 `CreateProcess` 错误，缺少 `as.exe`
  - 尝试通过 winget 安装 WinLibs MinGW-w64 失败：`InternetOpenUrl() failed: 0x80072efd`
- 状态：失败
- 严重副作用：`create-tauri-app --force .` 覆盖了根目录，导致原 `docs/`、`design-system/`、`AGENTS.md`、`CONTEXT.md`、`SESSION-LOG.md` 等文件丢失。

## 2026-06-26 22:20

- 会话主题：尝试 MSVC toolchain 安装 Tauri CLI
- 主要成果：
  - 切换 Rust toolchain 到 `stable-x86_64-pc-windows-msvc`
  - 尝试安装 `cargo tauri` CLI
- 异常：
  - 编译失败：`LINK : fatal error LNK1104: 无法打开文件“msvcrt.lib”`
  - 尝试通过 VS Installer 安装 VCTools 和 Windows SDK 失败：VC++ Redistributable 安装返回 1603/1714
  - 尝试通过 winget 安装 VC++ Redistributable 同样失败
- 状态：失败
- 结论：本机 MSVC 工具链不可用

## 2026-06-26 11:50

- 会话主题：逐项确认开发前检查清单
- 主要成果：
  - 检查并记录本地环境版本
  - 确认设计文档和原型存在
  - 标记 MVP 范围已冻结
  - 标注 Git 初始化延后
- 状态：已完成

## 2026-06-26 11:40

- 会话主题：创建开发前检查清单
- 主要成果：
  - 创建 `docs/开发前检查清单.md`
  - 更新 `CURRENT-TASK.md`
- 状态：已完成

## 2026-06-26 11:33

- 会话主题：将开发连续性保障机制重构为工具无关方案
- 主要成果：
  - 在 `docs/开发连续性保障机制.md` 中新增"工具无关性"核心原则
  - 将关键上下文文件迁移/创建到项目根目录
- 状态：已完成

---

## 2026-07-26 13:32 - M16 初始化向导完成 + i18n 完善

### 完成的工作

1. **初始化向导 UI 优化**
   - Logo 尺寸缩小（80x80 → 56x56）
   - 标题字号缩小（4xl → 2xl）
   - 步骤指示器缩小
   - 卡片图标和内边距缩小

2. **返回按钮**
   - 左上角返回按钮，仅手动触发时可见
   - 首次启动无返回按钮

3. **按钮顺序调整**
   - 初始化向导按钮移至设置按钮下方

4. **i18n 完善**
   - InitWizard 组件添加 useTranslation
   - 新增 20+ 翻译键到 zh.json 和 en.json
   - 覆盖欢迎信息、步骤指示、按钮文字等

### 验证状态
- [x] TypeScript 类型检查通过
- [x] release 构建成功
- [x] Previous 目录更新

### M16 任务状态
- Task 15: 导航优化 - ✅ 完成
- Task 16: i18n + 构建验证 - ✅ 完成
- Task 17: unwrap 替换 - ✅ 完成
- Task 18: log crate 集成 - ✅ 完成（radar.rs）
- Task 24-26: 文章生成 - 待完成

- 状态：M16 进行中

---

## 2026-07-26 14:15 - M16 导航优化完成 + 异常处理开始

### 完成的工作

1. **Task 15: 导航优化**
   - Logo 添加渐变背景
   - Logo 添加点击事件打开导航页
   - Logo 添加 hover 缩放效果
   - 导航按钮添加 i18n 支持

2. **Task 16: i18n 完善**
   - InitWizard 组件添加 useTranslation
   - 新增 20+ 翻译键

3. **Task 17: unwrap 替换**
   - download.rs 关键位置替换 unwrap
   - 添加错误处理

4. **Task 18: log crate 集成**
   - radar.rs 添加 log crate
   - 替换 20+ 处 println! 为 log::info!/debug!/error!

### 验证状态
- [x] TypeScript 类型检查通过
- [x] Rust 编译通过
- [x] release 构建成功
- [x] Previous 目录更新

- 状态：M16 进行中

---

## 2026-07-26 14:45 - Task 18 println 替换（lib.rs）

### 完成的工作

1. **Task 18: log crate 集成 - lib.rs**
   - 添加 `use log;`
   - 替换 lib.rs 中所有 println! 为 log::info!
   - 替换 eprintln! 为 log::error!
   - 约 20 处修改

2. **剩余待处理文件**（100+ 处）：
   - system_db.rs（15 处）
   - vault.rs（8 处）
   - llm.rs（3 处）
   - health.rs（1 处）
   - translate.rs（10 处）
   - plugin_manager.rs（5 处）
   - plugin_loader.rs（1 处）
   - plugin_config_db.rs（2 处）
   - db.rs（5 处）
   - content_filter.rs（3 处）
   - source_engine/*.rs（22 处）
   - commands/*.rs（10 处）

### 验证状态
- [x] Rust 编译通过
- [x] release 构建成功
- [x] Previous 目录更新

- 状态：M16 进行中

---

## 2026-07-26 18:07 - M16 文章生成功能完成

### 完成的工作

1. **Task 24: 后端 API**
   - 创建 `prompts/` 目录和 5 个 Prompt 模板文件
   - 创建 `article.rs`：Prompt 模板管理
   - 创建 `services/article_service.rs`：文章生成服务
   - 创建 `commands/article_cmd.rs`：Tauri 命令

2. **Task 25: 前端 UI**
   - 创建 `ArticleGenerationPanel.tsx` 组件
   - 集成到 `ProjectDetailDialog.tsx` 的文章标签页
   - 添加 i18n 翻译键

3. **Task 26: 导出功能**
   - 支持 Markdown 和 TXT 格式导出
   - 使用系统保存对话框

### 新增文件
- `src-tauri/src/prompts/mod.rs`
- `src-tauri/src/prompts/tech_popular.txt`
- `src-tauri/src/prompts/business.txt`
- `src-tauri/src/prompts/tech_blog.txt`
- `src-tauri/src/prompts/brief.txt`
- `src-tauri/src/prompts/humor.txt`
- `src-tauri/src/article.rs`
- `src-tauri/src/services/mod.rs`
- `src-tauri/src/services/article_service.rs`
- `src-tauri/src/commands/article_cmd.rs`
- `src/components/ArticleGenerationPanel.tsx`

### 验证状态
- [x] TypeScript 类型检查通过
- [x] Rust 编译通过
- [x] release 构建成功
- [x] Previous 目录更新

- 状态：M16 基本完成

---

## 2026-07-26 18:20 - 文章生成功能增强

### 完成的改进

1. **Markdown 编辑器优化**
   - 添加源码/预览模式切换按钮
   - 使用 marked + DOMPurify 渲染 Markdown 预览
   - 预览模式使用 prose 样式，美化排版

2. **PDF 导出功能**
   - 使用 html2pdf.js 库
   - 支持导出为 PDF 格式
   - PDF 包含美化的样式（标题、代码块、表格等）

### 文件变更
- `src/components/ArticleGenerationPanel.tsx` - 重写组件
- `package.json` - 添加 html2pdf.js 依赖

### 验证状态
- [x] TypeScript 类型检查通过
- [x] release 构建成功
- [x] Previous 目录更新

- 状态：M16 完成

---

## 2026-07-26 20:07 - M16 异常处理完成

### 完成的工作

1. **切片边界修复**
   - `crawler_service.rs`: 使用 strip_prefix 替代 url[7..] 防止 panic
   - `ai.rs`: 使用 json.get() 替代直接切片

2. **统一错误类型**
   - 创建 `error.rs` 模块
   - 包含 AppError 枚举（Database、Io、Parse、Config、Llm 等）
   - 实现 From trait 用于错误转换

3. **日志轮转配置**
   - 添加 tauri-plugin-log 依赖
   - 配置日志输出到 LogDir
   - 使用 RotationStrategy::KeepAll

4. **前端错误组件**
   - ErrorState 组件已完善

### 文件变更
- `src-tauri/src/error.rs` - 新增
- `src-tauri/src/commands/ai.rs` - 切片修复
- `src-tauri/src/crawler_service.rs` - 切片修复
- `src-tauri/src/lib.rs` - 添加 tauri-plugin-log
- `src-tauri/Cargo.toml` - 添加依赖

### 验证状态
- [x] cargo check 通过
- [x] release 构建成功
- [x] Previous 目录更新
- [x] git commit 提交

### 提交
- `7140148` - M16: 异常处理完成

---

## 2026-07-26 22:56 - M17 设计讨论

### 讨论的问题

1. **极简首页定位**
   - 决定：替代当前直接加载看板视图，作为系统启动默认页
   - 入口：左侧导航按钮下面

2. **URL 识别平台**
   - 决定：支持 GitHub, Gitee, GitLab, NPM, PyPI, Crates.io

3. **交互流程**
   - 决定：确认后导入（显示预览 → 用户确认 → 导入）

4. **其他决策**
   - Ctrl+K 提示：不保留
   - 页面风格：与系统主题保持一致

5. **M18 需求**
   - 默认页选择功能移至 M18

### 待确认
- 等待用户确认 M17 开发

- 状态：M17 设计讨论完成

---

## 2026-07-26 23:42 - M17/M18 需求讨论

### M17 需求确认
1. 首页入口：侧边栏 Logo 下方
2. URL 识别 + 预览：复用 ImportModal 和已有 API
3. 主题：跟随 App 设置
4. 默认启动页：极简首页
5. 最近项目、快捷入口

### M18 需求初稿：通用分类预置
- 用户建议：预置一套科学的项目分类标准
- 参考来源：GitHub Topics、PyPI Classifiers、crates.io
- 分类结构：三级分类
- 预置 9 个一级分类、26 个二级分类
- 安装时可选导入

### 待完成
- M17 需求最终确认
- M18 开发前进一步细化分类

- 状态：M17/M18 需求讨论完成

---

## 2026-07-27 09:52 - 规范文档更新

### 完成的工作
1. M17 需求最终确认
2. M18 需求初稿（通用分类预置）
3. **将 Vibe Coding 八荣八耻添加到 AGENTS.md**

### 八荣八耻已记录
- 位置：`AGENTS.md` 规则 3
- 内容：查档求证、对齐需求、请示规则、复用存量、完备校验、恪守规范、坦诚存疑、分步迭代

- 状态：规范文档已更新

---

## 2026-07-27 10:27 - M17 开发完成

### 完成的功能
1. **HomePage 组件**
   - 基于 home.html 原型创建
   - URL 平台识别（6个平台）
   - 项目预览确认
   - 最近项目（5个）
   - 主题适配

2. **侧边栏入口**
   - Logo 下方添加首页按钮

3. **默认启动页**
   - App 启动默认显示首页

4. **系统托盘**
   - 托盘图标
   - 托盘菜单（显示/隐藏、退出）
   - 关闭按钮最小化到托盘

### 文件变更
- `src/components/HomePage.tsx` - 新增
- `src/components/Sidebar.tsx` - 添加首页入口
- `src/App.tsx` - 默认视图改为首页
- `src/api/project.ts` - 添加 getRecent
- `src/api/index.ts` - 导出 getRecentProjects
- `src-tauri/src/commands/project.rs` - 添加后端命令
- `src-tauri/src/lib.rs` - 托盘初始化
- `src-tauri/Cargo.toml` - 添加 tray-icon feature

### 验证状态
- [x] TypeScript 类型检查通过
- [x] release 构建成功
- [x] Previous 目录更新

- 状态：M17 开发完成

---

## 2026-07-27 14:52 - M17 首页重构设计讨论

### 需求变更
1. 首页主功能从"项目导入框"改为"意图搜索（聊天模式）"
2. 最近使用从下方改为右侧固定栏
3. ChatGPT 风格布局

### 设计决策
- 左侧：搜索结果区域（可滚动）
- 右侧：最近使用（固定侧栏，竖向排列）
- 底部：搜索输入框（固定）
- 顶部：Logo + 导航

### 文档更新
- `CURRENT-TASK.md` - 更新 M17 描述
- `docs/M17实施计划.md` - 新建实施计划

### 待完成
- HomePage 组件重写
- 意图搜索集成
- 最近使用侧栏

- 状态：M17 设计完成，待开发

---

## 2026-08-02 19:45 - M17/M18 VSS DLL 路径问题修复

### 问题描述
意图搜索功能卡死，无法正常工作

### 根因分析

| # | 问题 | 根因 |
|---|------|------|
| 1 | VSS DLL 路径错误 | 使用 `BaseDirs::data_dir()` 返回 LOCALAPPDATA，但系统库应在 APPDATA |
| 2 | 死锁问题 | `three_layer_search` 持有 DATABASE 锁时调用 `ensure_vss_loaded()` 又尝试获取同一锁 |

### 关键结论确认

**系统库目录**：`C:\Users\Administrator\AppData\Roaming\.os-compass`
- plugins.db
- vss0.dll（sqlite-vss 扩展，全局共享）

**仓库库目录**：`C:\Users\Administrator\AppData\Local\com.administrator.os-compass\vaults\default`
- os_compass.db（包含 project_embeddings 向量表）

**数据存放规则**：
| 数据类型 | 存放位置 | 说明 |
|----------|----------|------|
| sqlite-vss 扩展 (vss0.dll) | 系统库 | 全局共享，所有仓库共用 |
| 向量数据 (project_embeddings 表) | 仓库库 | 每个仓库独立，互不影响 |

### 修复内容

1. **修改 preload_vss_once()** - 启动时预加载 VSS（数据库初始化后）
2. **修改 ensure_vss_loaded()** - 跳过加载，仅检查缓存（避免死锁）
3. **添加详细日志** - 写入 `C:\Users\Administrator\AppData\Roaming\.os-compass\debug.log`

### 待完成

1. 修改 VSS DLL 路径为正确的系统库路径（APPDATA）
2. 下载 vss0.dll 并放入系统库目录
3. 更新 tauri.conf.json 打包配置

### 状态：修复中，待验证

---

## 2026-08-17 22:53 - M19 平台扩展开发

### 完成内容

| # | 任务 | 状态 |
|---|------|------|
| 1 | TS 类型修复 | ✅ |
| 2 | NPM 适配器 | ✅ |
| 3 | PyPI 适配器 | ✅ |
| 4 | Crates.io 适配器 | ✅ |
| 5 | 编译验证 | ✅ |
| 6 | 版本发布 | ⏳ |

### 提交

- a0b3cbc: feat(M19): NPM 适配器实现 + TS 类型修复
- 3751f68: feat(M19): PyPI + Crates.io 适配器实现

---

## 2026-08-19 11:15 - M19.2 广告学习功能开发

### 完成内容

| Task | 内容 | 状态 | 验证 |
|------|------|------|------|
| Task 7 | 广告标记 UI | ✅ | TypeScript 编译通过 |
| Task 8 | LLM 广告判定服务 | ✅ | Rust 编译通过 |
| Task 9 | 屏蔽列表 CRUD API | ✅ | Rust 编译通过 |
| Task 10 | 雷达采集过滤集成 | ✅ | Rust 编译通过 |

### 新增文件

- `src-tauri/src/services/ad_detector.rs` - LLM 广告判定
- `src-tauri/src/commands/ad_patterns_cmd.rs` - 屏蔽列表 API
- `src/api/adPatterns.ts` - 前端 API

### 修改文件

- `src/components/RadarInbox.tsx` - 添加标记广告按钮
- `src/plugins/radar.rs` - 采集时广告过滤

### API 命令

- `mark_as_ad` - 标记广告并学习
- `list_ad_patterns` - 列出屏蔽规则
- `check_ad_pattern` - 检查是否广告
- `add_ad_whitelist` - 添加白名单
- `list_ad_whitelist` - 列出白名单

### 提交

- af59626: feat(M19.2): 广告学习功能后端实现
- 27fdd18: feat(M19.2): 广告标记 UI 实现
- 38ef339: feat(M19.2): 雷达采集广告过滤集成
- 5589d2e: docs: M19.2 全部完成

### 版本发布

- `Previous/os-compass.exe` - 已更新
- `Previous/WebView2Loader.dll` - 已更新

### 待测试功能

1. 雷达采集时广告过滤是否生效
2. 标记广告按钮是否正常工作
3. LLM 分析是否正确返回结果
4. 白名单功能是否正常

### 状态：⚠️ 开发完成，待手动测试验证

### 待继续

- M19.2 广告学习（Task 7-10）
- M19.3 定时采集（Task 11-15）

---

## 2026-08-19 13:25 - M19.2 规范验证（重新学习 AGENTS.md）

### 问题反思

| 违规规则 | 具体行为 |
|----------|----------|
| 规则 4：测试验证 | 只做编译检查，没有实际运行验证 |
| 规则 5：提交规范 | 提交前没有运行 git diff 检查 |
| 3.3 会话结束规范 | 缺少完整验证流程 |
| 3.1 会话开始 | 没有更新 SESSION-LOG.md |

### 规范验证结果（2026-08-19 13:25）

| 检查项 | 结果 | 说明 |
|--------|------|------|
| Rust 编译 | ✅ | cargo check --lib 通过 |
| TypeScript 编译 | ✅ | pnpm typecheck 通过 |
| exe 文件存在 | ✅ | Previous/os-compass.exe |
| 提交记录 | ✅ | 5 个提交，详见下方 |
| SESSION-LOG.md | ✅ | 已更新 |
| 工作进展记录 | ✅ | 已更新 |

### M19.2 完整提交记录

| Commit | 说明 |
|--------|------|
| af59626 | feat(M19.2): 广告学习功能后端实现 |
| 27fdd18 | feat(M19.2): 广告标记 UI 实现 |
| 38ef339 | feat(M19.2): 雷达采集广告过滤集成 |
| 5589d2e | docs: M19.2 全部完成 |
| c0e41b6 | docs: 更新会话记录 |
| af8a5b2 | docs: 测试验证完成 |

### 待手动测试

1. 启动程序 → 雷达收件箱
2. 点击"标记广告"按钮 → 验证 LLM 分析
3. 触发雷达采集 → 检查日志 `[AD-FILTER]`

### 状态：⚠️ 编译通过，功能待手动验证

## 2026-09-07 11:36 - 搜索相关性阈值 + 再推荐反馈优化实施完成

### 关键操作
1. 按已确认的修复计划实施：A1 阈值过滤（SEMANTIC_SEARCH_MIN_SCORE = 0.45）+ 维度不一致跳过；A2 兜底条件经阈值过滤后语义自动正确；A3 向量诊断（45 项目/20 向量/全部 1024 维/25 个未向量化含最相关项目）；B1-B3 前端去重扩展到本地结果、0 新增 Toast、内联 loading、滚动定位、后端 URL 日志。
2. TDD：新增 SearchView.recommend-feedback.test.tsx 3 用例，首跑抓出真 bug（去重漏比对本地结果），修复后全绿。
3. 验证：75 tests / typecheck / cargo check --lib --tests / tauri build 全过；Previous 已同步（SHA-256 AFCBF92B...）。
4. cargo test CLI 运行报 STATUS_ENTRYPOINT_NOT_FOUND，属 AGENTS.md 已记录的环境限制。

### 用户侧待办
运行新版本后点看板页"向量化全部"补齐 25 个缺失向量。

### 提交
见 git log（fix(search): ...）
