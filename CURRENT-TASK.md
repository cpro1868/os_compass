# 当前任务

## 任务标题

M13 Phase 2 开发：意图搜索与情报雷达

## 状态

done

## 阶段目标

实现二期 M13 功能：系统插件管理基础设施，信息源引擎、情报雷达插件（RSS/网站/TG 监控 + 收件箱）、意图搜索插件（三层优先级搜索 + ChatGPT 对话 UI）。

## 完成进度

- [x] Task 1: FeaturePlugin trait + 类型定义 (4c7f38d)
- [x] Task 2: feature_plugins 数据库表 (2907ca5)
- [x] Task 3: PluginManager + 占位插件 (b473b3e)
- [x] Task 4: 插件管理 Tauri 命令 (9f661eb)
- [x] Task 5: SourceAdapter + 4 种适配器 (1c2d20d)
- [x] Task 6: LLM 解析器 (c0efc71)
- [x] Task 7: RadarPlugin 完整实现 (193464f)
- [x] Task 8: SearchPlugin 完整实现 (2fcb5d4)
- [x] Task 9: 启动初始化 + 前端 API 层 (6e87ec1)
- [x] Task 10: RadarInbox 前端视图 (59449c0)
- [x] Task 11: SearchView 前端视图 (59449c0)
- [x] Task 12: 前端集成 (844ac4e)
- [x] Task 13: i18n + 暗色 + 构建验证 (745eed1)

## 验证状态

- [x] Rust 编译检查通过
- [x] TypeScript 类型检查通过
- [x] vitest 测试 18/18 通过
- [x] release 构建成功
- [x] Previous 目录更新
