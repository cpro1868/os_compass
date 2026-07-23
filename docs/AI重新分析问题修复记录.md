# AI 重新分析问题修复记录

**创建日期**：2026-07-23
**问题**：点击"重新分析"后长时间无输出（超过 30 秒）
**状态**：✅ 已修复（待验证）

---

## 问题根因

### 2026-07-24 - 代码块解析 Bug（已修复）

**日志分析**：
```
[AI_ANALYZE] json_str created, len=1251
[AI_ANALYZE] find ```json: Some(0), find ```: Some(0)
```

**问题**：当 LLM 返回以 ` ```json ` 开头的 JSON 时：
1. `find("```json")` 返回 `Some(0)`
2. `find("```")` 也在位置 0 找到
3. 代码执行 `json_str[start + 7..start + end]` = `json_str[7..7]` = **空切片**
4. 后续 JSON 解析因空字符串而卡住

**错误代码**：
```rust
if let Some(start) = json_str.find("```json") {
    if let Some(end) = json_str[start..].find("```") {
        json_str = json_str[start + 7..start + end].trim().to_string();
        //                                        ^^^^^^^^ ^^^^^^^^
        //                                        start=0, end=0 → 空切片 json_str[7..7]！
    }
}
```

**正确代码**：
```rust
if let Some(start) = json_str.find("```json") {
    if let Some(end) = json_str[start..].find("```") {
        let actual_end = start + end;  // 计算绝对结束位置
        json_str = json_str[start + 7..actual_end].trim().to_string();
    }
}
```

**修复文件**：`src-tauri/src/commands/ai.rs`

---

## 愚蠢建议记录（避免再犯）

### 2026-07-23 - 桌面应用不应该用 F12 调试

**错误建议**：
```
请打开浏览器开发者控制台（F12），运行"重新分析"
```

**问题**：这是 Tauri 桌面应用，不是浏览器，没有 F12 控制台。

**正确做法**：
- 使用后端文件日志：`%AppData%\.os-compass\logs\ai_analyze.log`
- 或者使用 Tauri 的 webview 日志输出

---

## 问题描述

| 项目 | 内容 |
|------|------|
| 功能 | 项目详情页"重新分析"按钮 |
| 预期 | 点击后调用 LLM API 生成 AI 分析报告 |
| 实际 | 点击后长时间无响应（超过 30 秒） |
| 严重程度 | 🔴 严重（核心功能不可用） |

### 2026-07-23 - 具体现象

**测试项目**：clawfeed

**复现步骤**：
1. 打开 clawfeed 项目
2. 点击"重新分析"按钮

**现象**：
1. 滚动条会一直滚（Loading 状态）
2. 按钮始终显示：`AI 已返回结果 (1324 字符)，正在解析..`
3. 始终无法处理完成，状态卡死

**关键信息**：
- LLM 已成功返回结果（1324 字符）
- 问题出在**解析阶段**（`正在解析..`）
- 不是超时问题，而是 JSON 解析卡住

---

## 历史记录

### 2026-07-15 - 首次发现

**问题**：OpenHarness 项目点击"重新分析"后长时间无响应（超过5分钟）

**排查结果**：
1. 后端验证 - 创建独立测试程序 `debug_full` 模拟 `analyze_project` 流程
   - API Key 解密正常
   - HTTP 请求成功：8秒返回 200 OK
   - JSON 解析成功：数据完整

2. **根本原因发现**：
   - 测试直连 `api.sfkey.cn` → Connection timed out
   - 使用 `github_proxy` (127.0.0.1:8964) 测试 → 返回 401（网络通了）
   - **LLM API 需要代理才能访问**
   - 当前 `llm_proxy_enabled = 0`（未启用）

**结论**：后端 `analyze_project` 逻辑完全正常，问题疑似在前端 UI 状态更新或 Tauri invoke 调用层

**提交**：4a0b750

---

### 2026-07-22 - M15 完成

**状态**：认为问题已解决（可能误解为配置问题）

---

### 2026-07-23 - 问题重现

**状态**：问题仍然存在，用户反馈重新分析仍然卡住

---

## 排查进展

### 排查 1：后端 JSON 解析代码分析

**位置**：`src-tauri/src/commands/ai.rs` 第 258-340 行

**发现的问题**：
1. JSON 解析有复杂的 fallback 逻辑（第 270-338 行）
2. `extract_field` 闭包中有嵌套循环和字符串操作（第 278-312 行）
3. 可能的性能问题：
   - `rest.find('"')` 可能匹配到错误位置
   - 循环处理大量数据时可能卡住
   - 第 294 行 `split(',')` + `trim_matches` 可能在某些输入上表现不佳

**可疑代码**：
```rust
// ai.rs 第 288-309 行
for pattern in &patterns {
    if let Some(pos) = json.find(pattern) {
        let value_start = pos + pattern.len();
        let rest = &json[value_start..];
        if rest.starts_with('[') {
            // 数组处理...
        } else if let Some(end) = rest.find('"') {
            let val = &rest[..end];  // 可能截取错误
            // ...
        }
    }
}
```

### 排查 2：前端状态

- [x] handleAnalyze 函数逻辑正确
- [x] finally 块会执行 setAnalyzing(false)
- [x] 无明显死循环代码

### 排查 3：建议添加日志

**需要添加**：
- [ ] 在 JSON 解析前打印 LLM 返回的原始内容
- [ ] 在每个解析阶段打印进度
- [ ] 测量每个阶段的耗时

---

## 修复步骤

### Step 1: 添加详细日志（必须）

在 `ai.rs` 的 `analyze_project` 函数中添加日志：
1. 打印 LLM 返回的原始 response
2. 在 JSON 解析每个阶段打印进度
3. 测量解析耗时

### Step 2: 简化 fallback 逻辑

如果日志显示是 fallback 逻辑卡住，考虑：
1. 限制处理的数据量
2. 简化字符串操作
3. 添加超时机制

### Step 3: 验证

- [ ] 重新分析在合理时间内完成（< 60s）
- [ ] 分析结果正确显示

---

## 修复日志

### 2026-07-23 - 添加文件日志

**修改文件**：`src-tauri/src/commands/ai.rs`

**添加的功能**：
1. 添加 `log_to_file` 闭包，同时写日志到文件和 stderr
2. 日志文件位置：`%AppData%\com.os-compass.os-compass\logs\ai_analyze.log`
3. 每个关键步骤后调用 `log_to_file`，确保日志被写入

**编译检查**：✅ 通过
**构建状态**：✅ release 构建成功

**问题分析**：
1. 用户说 LLM 已返回 1324 字符，但一直显示"正在解析..."
2. 这说明后端 invoke 已完成，但可能卡在 JSON 解析阶段
3. 需要通过日志定位具体卡在哪一步

**测试方法**：
1. 运行 `Previous\os-compass.exe`
2. 打开 clawfeed 项目，点击"重新分析"
3. 等待问题复现
4. 查看日志文件：`%AppData%\com.os-compass.os-compass\logs\ai_analyze.log`

**日志格式**：
```
[2026-07-23 22:30:00.123] [analyze_project] Waiting for LLM response (timeout: 180s)...
[2026-07-23 22:30:08.456] [AI_ANALYZE] LLM response received: 1324 chars, time: ...
[2026-07-23 22:30:08.789] [AI_ANALYZE] Raw response (first 800 chars): ...
[2026-07-23 22:30:08.890] [AI_ANALYZE] >>> About to call serde_json::from_str, json_len=...
[2026-07-23 22:30:08.901] [AI_ANALYZE] Step 3: JSON parse OK/FAILED, time: ...
```

**预期日志位置**：
- Windows: `%AppData%\com.os-compass.os-compass\logs\ai_analyze.log`

---

**最后更新**：2026-07-23 22:30
**下一步**：运行测试，查看日志文件定位问题
