# Changelog

All notable changes to OS-Compass are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Fixed
- 安装包缺失 DLL 导致新机无法启动：zip 关闭默认特性移除 liblzma 动态依赖；WebView2Loader.dll 随 NSIS/MSI 安装到 exe 同目录
- 全新仓库建库失败：删除引用已迁走表（system_variables/source_plugins）的孤儿索引
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
