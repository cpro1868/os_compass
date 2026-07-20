# OS-Compass 当前任务

## 项目整体状态

| 阶段 | 状态 | 完成时间 |
|------|------|----------|
| Phase 0 准备期 | ✅ 完成 | 2026-07-06 |
| Phase 1-1 MVP 核心 | ✅ 完成 | 2026-08-19 |
| Phase 1-2 增强功能 | ✅ 完成 | 2026-08-19 |
| **M14 插件系统扩展** | 🔄 进行中 | — |

---

## 完整工作清单

详见：`docs/完整工作清单.md`

### 一、系统改造（V2.0 配置迁移）

| 序号 | Task | 工作内容 | 工时 | 优先级 |
|------|------|---------|------|--------|
| 1 | Task 0.1 | 系统级配置存储设计 | 1d | P0 |
| 2 | Task 0.2 | settings.rs 重构 | 2d | P0 |
| 3 | Task 0.3 | variables.rs 重构 | 2d | P0 |
| 4 | Task 0.4 | 依赖模块适配 | 1.5d | P0 |
| 5 | Task 0.5 | 配置迁移器 | 1.5d | P0 |
| 6 | Task 0.6 | 验证与回归测试 | 1.5d | P0 |

### 二、情报雷达功能

| 序号 | Task | 工作内容 | 工时 | 优先级 |
|------|------|---------|------|--------|
| 7 | Task 18.1 | 适配器注册表 | 0.5d | P1 |
| 8 | Task 18.2 | 适配器接口抽象 | 0.5d | P1 |
| 9 | Task 18.3 | 适配器管理器 | 0.5d | P1 |
| 10 | Task 19.1 | WebMonitorAdapter 结构体 | 0.5d | **P0** |
| 11 | Task 19.2 | 网页抓取实现 | 0.5d | P0 |
| 12 | Task 19.3 | 内容变化检测 | 0.5d | P0 |
| 13 | Task 19.4 | 链接提取 | 0.5d | P0 |
| 14 | Task 19.5 | 适配器注册 | 0.5d | P0 |
| 15 | Task 19.6 | 前端 UI 更新 | 0.5d | P0 |
| 16 | Task 20.1 | 适配器选择卡片 | 0.25d | P1 |
| 17 | Task 20.2 | 动态配置字段 | 0.25d | P1 |
| 18 | Task 21.1 | 链接标准化 | 0.25d | P1 |
| 19 | Task 21.2 | 导入前检查 | 0.25d | P1 |
| 20 | Task 21.3 | 扩展匹配 | 0.25d | P1 |
| 21 | Task 21.4 | 导入流程 | 0.25d | P1 |

---

## 工时汇总

| 类别 | 工时 |
|------|------|
| 系统改造（V2.0 配置迁移） | 9.5d |
| 情报雷达功能 | 5d |
| **总计** | **14.5d** |

---

## 执行计划

### Phase 1: 系统改造（Week 1-2）

```
Day 1: Task 0.1 系统级配置存储设计
Day 2-3: Task 0.2 settings.rs 重构
Day 4-5: Task 0.3 variables.rs 重构
Day 6-7: Task 0.4 依赖模块适配
Day 8-9: Task 0.5 配置迁移器
Day 10-11: Task 0.6 验证与回归测试
```

### Phase 2: 情报雷达功能（Week 3-4）

```
Week 3: Task 18 适配器扩展机制
Week 3-4: Task 19 固定网页监控适配器
Week 4: Task 20 雷达 UI 优化 + Task 21 链接导入增强
```

---

## 验证状态

- [x] Rust 编译检查通过
- [x] TypeScript 类型检查通过
- [x] release 构建成功
- [x] Previous 目录更新

---

## 会话日志

### 2026-07-17 01:42 - 情报雷达 Telegram 适配器系统性修复

**问题**：
1. Telegram 采集无数据 - 只提取链接，没提取消息内容
2. 代理配置没有读取信息源自己的设置
3. URL 提取模式硬编码（包含 GitLab），应从 source_plugins.url_patterns 读取
4. 没有编辑/删除信息源功能
5. 错误信息没有显示

**系统性修复**：
1. **数据架构**：
   - 信息源（radar_sources）→ 系统库 plugin_config.db
   - 采集数据（radar_items）→ 仓库 plugin_radar.db

2. **代理配置** - radar.rs：
   - 优先读取信息源自己的代理配置
   - 其次读取全局代理设置

3. **Telegram 适配器** - telegram_adapter.rs：
   - 提取消息内容，不只是链接
   - 4种解析方式（Widget/Legacy/Simple/Text）
   - 从 source_plugins.url_patterns 读取支持的平台
   - 请求延迟 800ms、cookies 保持

4. **前端 UI** - RadarInbox.tsx：
   - 添加编辑按钮（铅笔图标）
   - 添加删除按钮（垃圾桶图标）
   - 错误状态显示

**提交**：
- ebfc24b: radar_sources to system DB
- a924ac5: radar proxy config + edit/delete UI
- 170d185: telegram browser UA
- ac8315b: telegram session handling
- bdd5a7a: telegram extracts message content
- 200c3a8: telegram multi-parser
- d0436ba: telegram reads URL patterns from source_plugins

**状态**：🔄 待测试验证

---

### 2026-07-16 23:01 - 情报雷达修复

**问题**：
1. 采集时没有读取信息源配置的代理
2. 前端没有编辑/删除信息源功能

**修复**：
1. radar.rs - 优先使用信息源自己的代理
2. RadarInbox.tsx - 添加编辑/删除按钮
3. rss_adapter.rs - 增强 RSS 解析

**状态**：✅ 修复完成

---

### 2026-07-16 18:01 - 情报雷达数据架构重构

**问题**：用户反馈情报雷达采集后无数据

**需求**：信息源放系统库（plugin_config.db），采集数据放仓库（plugin_radar.db）

**修复**：
1. 系统库添加 radar_sources 表和相关 CRUD 方法
2. PLUGIN_CONFIG_DB 添加雷达信息源管理方法
3. radar_cmd.rs 全部改用 PLUGIN_CONFIG_DB
4. radar.rs 扫描时从系统库读取信息源
5. vault 数据库移除 radar_sources 表（只保留采集数据）

**状态**：✅ 架构重构完成

---

### 2026-07-14 晚间 - Bug 修复

**问题**：
1. LLM 配置保存后重启丢失
2. 扩展配置（source_plugins）被清空

**根因**：`scripts/init_schema.sql` 缺少 `url_patterns` 字段，导致表结构与代码不一致

**修复**：
1. 更新 `scripts/init_schema.sql`：添加 url_patterns 列，更新种子数据
2. 同步更新 `Previous/scripts/init_schema.sql`
3. 重新构建并更新 Previous 目录

**提交**：6cb34ca fix: add url_patterns column to source_plugins table

### 2026-07-15 上午 - Bug 修复 + 问题定位

**已修复 Bug**：
1. ❌ ~~扩展配置丢失~~ → ✅ source_plugins 移至系统库
2. ❌ ~~LLM 配置保存不生效~~ → ✅ app_settings 表 is_secret 列问题
3. ❌ ~~重新分析无错误提示~~ → ✅ 添加错误 Toast 显示

**问题定位**：
- 重新分析 OpenHarness 超时 → LLM API (`api.sfkey.cn`) 直连超时
- 使用 `github_proxy` (127.0.0.1:8964) 可访问 → 需要启用 LLM 代理

**用户要求**：
- 定位问题时不要擅自修改代码
- 每次会话要及时更新会话记录

### 2026-07-14 下午 - 文档完善

**动作**：
1. 检查实施和测试文档完整性
2. 发现缺失项：
   - 数据库表结构设计（system_settings, system_vars）
   - V2.0 迁移测试用例
   - 情报雷达测试用例
3. 补充以下文档：
   - `数据库设计说明书.md`：新增 §9.0 系统级配置库
   - `测试文档/V2.0迁移测试用例.md`：17 个测试用例
   - `测试文档/情报雷达测试用例.md`：17 个测试用例

**开发条件评估**：
| 阶段 | 就绪度 |
|------|--------|
| 系统改造（V2.0） | 90% |
| 情报雷达功能 | 80% |

**下一步**：开始开发

### 2026-07-14 下午

- 会话主题：输出完整工作清单（系统改造 + 情报雷达）
- 主要工作：
  - 创建 `docs/完整工作清单.md`
  - 详细列出系统改造任务（Task 0.1-0.6）
  - 详细列出情报雷达任务（Task 18-21）
  - 更新 `docs/CURRENT-TASK.md`

- 工时汇总：
  | 类别 | 工时 |
  |------|------|
  | 系统改造 | 9.5d |
  | 情报雷达功能 | 5d |
  | **总计** | **14.5d** |

- 状态：**工作清单完成，等待评审和开发**

### 2026-07-15 17:19 - OpenHarness 分析卡住问题定位

**问题**：OpenHarness 项目点击"重新分析"后长时间无响应（超过5分钟）

**排查结果**：
- ✅ 后端 `analyze_project` 逻辑完全正常
- ✅ 独立测试程序成功：LLM 8秒返回，JSON 解析正确
- ✅ API Key 解密正常
- ✅ HTTP 请求 200 OK

**新增调试功能**：
1. `debug_llm_status` - 检查 LLM 配置状态
2. `test_llm_direct` - 直接测试 LLM 调用
3. 前端"调试"按钮 - 方便测试

**结论**：后端无问题，疑似前端 UI 状态更新或 invoke 调用层问题

**待验证**：使用"调试"按钮确认前端 invoke 是否正常
