# 当前任务

## 任务标题

M13 Phase 2 开发：意图搜索与情报雷达

## 状态

in_progress

## 阶段目标

实现二期 M13 功能：系统插件管理基础设施、信息源引擎、情报雷达插件（RSS/网站/TG 监控 + 收件箱）、意图搜索插件（三层优先级搜索 + ChatGPT 对话 UI）。

## 当前 Task

**Task 1: FeaturePlugin Trait + 类型定义**

## Task 依赖链

```
Task 1 (FeaturePlugin trait) ──> Task 3 (PluginManager) ──> Task 4 (插件命令)
Task 2 (DB 表) ────────────────────────────────────────────> Task 4
Task 3 ──> Task 5 (SourceAdapter) ──> Task 6 (LLM Parser) ──> Task 7 (RadarPlugin) ──> Task 9 ──> Task 11 ──> Task 13 ──> Task 14
Task 3 ──> Task 5 ──> Task 6 ──> Task 8 (SearchPlugin) ──> Task 10 ──> Task 12 ──> Task 13 ──> Task 14
```

## 参照文档

- `docs/插件接入规范.md`
- `docs/M13实施计划.md`
- `docs/数据库设计说明书.md` §9
- `docs/API接口设计说明书.md` §3.14-3.16

## 全局约束

- **二期功能标记**：所有代码和文档必须标注为二期（Phase 2）功能
- **Rust 编译检查**：`cargo check --lib` 验证编译
- **前端测试**：`cd os-compass; pnpm test`（vitest）
- **类型检查**：`cd os-compass; pnpm tsc --noEmit`
- **提交前检查**：`powershell -ExecutionPolicy Bypass -File scripts\pre-commit.ps1`
- **暗色模式**：所有新增 UI 必须包含 `dark:` 样式
- **i18n**：所有新增 UI 字符串必须使用 `useTranslation` + t() 调用
- **不添加注释**：代码中不添加任何注释（除非用户要求）

## 完成进度

- [ ] Task 1: FeaturePlugin trait + 类型定义
- [ ] Task 2: feature_plugins 数据库表
- [ ] Task 3: PluginManager + 占位插件
- [ ] Task 4: 插件管理 Tauri 命令
- [ ] Task 5: SourceAdapter + 4 种适配器
- [ ] Task 6: LLM 解析器
- [ ] Task 7: RadarPlugin 完整实现
- [ ] Task 8: SearchPlugin 完整实现
- [ ] Task 9: 启动初始化 + 前端 API 层
- [ ] Task 10: RadarInbox 前端视图
- [ ] Task 11: SearchView 前端视图
- [ ] Task 12: 前端集成
- [ ] Task 13: i18n + 暗色 + 构建验证
