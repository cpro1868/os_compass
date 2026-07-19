# 检查点：情报雷达简化版本

**创建时间**：2026-07-19 22:13

## 版本说明

情报雷达数据存储简化设计：
- 信息源 → 系统库 `plugins.db`
- 采集数据 → 系统目录 `$env:APPDATA/.os-compass/plugin_radar.db`

## 包含功能

1. 情报雷达信息源管理（CRUD）
2. 情报雷达采集数据展示
3. 情报雷达扫描采集
4. 仓库图标（数据库图标）
5. 情报雷达数据跟随仓库切换保持不变

## 提交记录

- `a7cc588` - docs: simplify radar storage to system dir in AGENTS.md
- `b2d8116` - fix: radar - simplify to use system dir for radar data
- `4faef39` - docs: update radar data storage architecture

## 关键文件

- `os-compass/src/components/Sidebar.tsx` - 仓库图标（数据库图标）
- `os-compass/src-tauri/src/commands/radar_cmd.rs` - 雷达命令
- `os-compass/src-tauri/src/plugins/radar.rs` - 雷达插件
- `AGENTS.md` - 规则 2 已更新
- `docs/插件接入规范.md` - 已更新
- `docs/数据库设计说明书.md` - 已更新

## 注意事项

1. 情报雷达数据存储在系统目录，切换仓库时不丢失
2. 如需恢复仓库隔离设计，请参考早期提交
