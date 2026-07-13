# 当前任务

## 任务标题

M13 Phase 2 开发：意图搜索与情报雷达（架构完善）

## 状态

in_progress

## 阶段目标

完善插件系统架构设计：
1. 插件分类（系统级/仓库级）
2. 操作域层次（插件域/仓库域/业务域）
3. 存储架构（配置库独立）
4. 前端 UI 重构

## 完成进度

- [x] Task 1: FeaturePlugin trait + 类型定义 (4c7f38d)
- [x] Task 2: feature_plugins 数据库表 (2907ca5)
- [x] Task 3: PluginManager + 占位插件 (b473b3e)
- [x] Task 4: 插件管理 Tauri 命令 (9f661EB)
- [x] Task 5: SourceAdapter + 4 种适配器 (1c2d20d)
- [x] Task 6: LLM 解析器 (c0efc71)
- [x] Task 7: RadarPlugin 完整实现 (193464f)
- [x] Task 8: SearchPlugin 完整实现 (2fcb5d4)
- [x] Task 9: 启动初始化 + 前端 API 层 (6e87ec1)
- [x] Task 10: RadarInbox 前端视图 (59449c0)
- [x] Task 11: SearchView 前端视图 (59449c0)
- [x] Task 12: 前端集成 (844ac4e)
- [x] Task 13: i18n + 暗色 + 构建验证 (745eed1)
- [x] Task 14: 插件配置存储架构重构 (1866c22)
  - 新增 `plugin_config_db.rs`：独立插件配置库
  - 配置存储在 `${AppData}/.os-compass/plugins.db`
- [x] Task 15: 设计文档完善
  - 插件接入规范.md：新增操作域层次
  - API接口设计说明书.md：补充存储架构说明
  - 数据库设计说明书.md：补充 plugins.db 表结构
  - 原型功能对照表.md：M13 页面状态更新
- [ ] Task 16: 前端 UI 重构（Settings → 功能插件 tab）
- [ ] Task 17: 插件配置弹窗实现

## 设计确认

### 操作域层次

```
插件域（系统级）: ${AppData}/.os-compass/plugins.db
仓库域（当前仓库）: 当前仓库 os_compass.db
业务域（跟随仓库）: {vault_dir}/plugin_*.db
```

### API 接口

| 接口 | 说明 |
|------|------|
| list_feature_plugins | 列出所有插件 |
| set_plugin_enabled | 启用/禁用插件 |
| get_plugin_config | 获取插件配置 |
| save_plugin_config | 保存插件配置 |
| plugin_get_db_path | 获取插件数据库路径 |

## 验证状态

- [x] Rust 编译检查通过
- [x] TypeScript 类型检查通过
- [x] release 构建成功
- [x] Previous 目录更新
