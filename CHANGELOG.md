# Changelog

All notable changes to OS-Compass are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed
- 首页搜索与意图搜索统一：意图分析阶段 + 追问澄清 + 搜索历史 + 联网/本地结果分类展示 + 缓存
- 分类管理：导入按钮、未分类挂子分类拦截、project_count 字段

### Added
- 分类自定义导入：txt / json 双格式 + 预览 + UTF-8/GBK 编码兜底
- 预置分类 JSON 版本（`scripts/preset_categories.json`）

### Fixed
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
