# Changelog

All notable changes to OS-Compass are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.6] - 2026-10-30

### Fixed
- 修复 GitHub Actions Ubuntu Rust 检查因缺少 Tauri Linux 系统依赖而失败的问题，并固定 CI Rust 检查环境为 Ubuntu 22.04。

### Improved
- 本地相似项目优先展示 AI 一句话摘要，语言字段不再显示原始 JSON 数组，并限制摘要长度。

## [0.2.5] - 2026-10-29

### Added
- M28 相似项目：项目详情新增相似项目选项卡，支持本地向量相似度匹配、GitHub/Gitee/GitLab/NPM/Crates.io 免费网络搜索、7 天仓库缓存和手动 LLM 推荐兜底；LLM 推荐仅使用项目名称、描述和语言，不读取用户信息。

### Improved
- 项目详情 URL 支持截断显示、超链接和一键复制。
- 看板分类筛选支持层级多选及父子级联。
- 列表搜索扩展到名称、描述、URL、语言和 AI 摘要。
- 用户笔记纳入项目向量文本，笔记保存、修改、删除后自动刷新项目向量。

## [0.2.4] - 2026-10-27

### Fixed
- LLM「测试连接」按钮失效：前端误调 `analyze_project(-1)`，后端对不存在的项目返回 `Ok` 导致恒显示成功；改为调用真实的 `test_llm_connection` 命令（发送真实请求，支持 LLM 代理参数，401/404/429 返回具体原因并在界面展示错误详情）
- Embedding「测试连接」为恒真桩函数：后端 `test_embedding_connection` 改为真实调用一次 `generate_embedding` 验证连通性，失败时界面展示具体错误

## [0.2.3] - 2026-10-25

### Fixed
- 分类管理：点击"编辑"图标无反应（openEditCategory 漏了打开表单弹窗的 setShowNewForm）
- 分类管理：新增/编辑分类补上设计中的"分类说明"（explain）字段——categories 表加列、老库启动时自动 ALTER TABLE 补列、后端读写贯通、表单渲染说明输入框

## [0.2.2] - 2026-10-20

### Fixed
- 文章生成一直转圈无响应：README 超过 2000 字节时按字节硬切片截在多字节字符中间导致 Rust panic，Tauri 命令永不返回（改为字符边界安全截断）
- 雷达定时调度时区计算错误：last_run 本地时间被当 UTC 解析，elapsed 下溢为 u64 巨值，导致扫描背靠背无限循环（改为本地朴素时间比较 + 负值钳零）
- 安全：清除 git 公开历史中泄漏的 API Key（filter-repo 重写历史），CI 安全扫描范围扩大到根目录脚本
- 依赖治理：补登 react-markdown（幻影依赖导致 CI 干净构建失败）

## [0.2.1] - 2026-10-14

### Fixed
- 安装包缺失 DLL 导致新机无法启动：zip 关闭默认特性移除 liblzma 动态依赖；WebView2Loader.dll 随 NSIS/MSI 安装到 exe 同目录
- 全新仓库建库失败（no such table: main.system_variables）：外部 init_schema.sql 孤儿索引删除、fallback 真回退、启动覆盖同步脚本、内嵌 schema 改 include_str! 单一来源
- 新机默认英文：i18n 探测去掉 navigator，启动时应用后端 default_language

## [0.2.0] - 2026-10-09

### Added
- M24 项目探索：项目详情第 10 个选项卡，以项目资料（README + AI 报告 + Runbook + 笔记）为知识库的对话式深挖，混合上下文策略（短资料全文 + README 分块向量检索），支持一键存入项目笔记
- M25 GitLab 平台插件：API v4 项目信息 + README + releases，识别/导入/预览/README 刷新全链路接入
- M25 四平台 README 抓取：NPM readme 字段 / PyPI description / Gitee readme base64 / Crates readme
- M25 NPM/PyPI/Crates 健康度评分接入导入流程
- M26 统一网络代理 `network_proxy`：Gitee/GitLab/NPM/PyPI/Crates 统一走该代理，GitHub 保留 `github_proxy` 专用覆盖
- 情报雷达图片渲染：Telegram 照片/视频、RSS 正文图片以 Markdown 语法入库，前端渲染缩略图
- 雷达正文 Markdown 链接 `[文本](url)` 渲染
- 分类自定义导入：txt / json 双格式 + 预览 + UTF-8/GBK 编码兜底
- 预置分类 JSON 版本（`scripts/preset_categories.json`）

### Changed
- 项目详情弹窗加大加高（w-92vw / max-w-1600px / h-92vh），分类与状态同行展示
- 首页搜索与意图搜索统一：意图分析阶段 + 追问澄清 + 搜索历史 + 联网/本地结果分类展示 + 缓存
- 分类管理：导入按钮、未分类挂子分类拦截、project_count 字段
- 系统变量读写切换到系统库（方案B），凭据跨仓库共享

### Fixed
- crates.io 请求必带 UA（此前裸请求必 403，导入完全不可用）
- NPM scoped 包解析错误（`@types/node` 被截成 `node` 修错包）
- crates 字段误用：stars 不再塞下载量、latest_commit 不再塞 rust_version
- NPM/PyPI/Crates 导入失败爬虫兜底
- 雷达链接与后随文本粘连：`url@用户名`、`urlAI探索` 正确拆分（锚标签边界保留 + CJK 区间排除 + 尾随中英文标点剥离）
- 纯图片 Telegram 消息不再被丢弃
- 项目笔记保存后详情页不实时刷新、笔记无 Markdown 解析
- 分类管理页面被 OrganizationManager 误渲染（App.tsx 路由修正）
- 子分类缩进不累加（多层级 ml-* 改为内联 marginLeft）
- i18n 单花括号插值不生效（统一为 i18next v26 双花括号语法）

## [0.1.0] - 2026-07-06

### Added
- Phase 0：开发环境搭建 + Tauri 2.0 工程初始化
- Phase 1-1（M1-M12）：MVP 核心功能（项目导入、AI 分析、本地联动）
- Phase 1-2：批量操作、归档系统、统计面板、国际化与体验优化
- M13：情报雷达 + 意图搜索基础版
- M14：插件系统扩展
- M15：数据保险箱 + 雷达搜索增强 + 广告过滤
- M16：初始化向导 + 异常处理
- M17：向量语义搜索（sqlite-vec）
- M18：智能意图搜索（意图分析 + 追问 + 智能推荐）
- M18-2：向量搜索改造（sqlite-vss → sqlite-vec）
