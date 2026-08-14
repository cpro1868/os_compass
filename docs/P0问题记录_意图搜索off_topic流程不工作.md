# P0 问题记录：意图搜索模块 - off_topic 流程不工作

**日期**: 2026-08-08
**问题编号**: ISSUE-001
**优先级**: P0 (阻塞)
**影响功能**: M15-2 意图搜索
**持续时间**: ~5天
**状态**: 未解决

---

## 1. 问题模块

| 项目 | 内容 |
|------|------|
| 模块名称 | 意图搜索 (Intent Search) |
| 涉及文件 | `src/components/SearchView.tsx` |
| 涉及API | `analyze_user_intent` |
| 功能点 | off_topic 意图处理 |

---

## 2. 问题描述

**复现步骤**：
1. 打开 OS-Compass 应用
2. 进入意图搜索页面
3. 输入"你好"
4. 点击发送

**预期行为**：
- LLM 返回 `{"intent": "off_topic", "guidance": "你好！我是开源项目助手..."}`
- 前端显示 guidance 消息
- UI 不再显示"正在分析语义..."

**实际行为**：
- 后端日志确认返回了 off_topic
- 前端日志确认收到了 off_topic
- 但 UI **始终显示"正在分析语义..."**
- guidance 消息未显示

---

## 3. 代码修改记录

| 日期 | 修改内容 | 结果 |
|------|---------|------|
| 第1天 | 修复 TypeScript 类型 `intent: string` | 测试通过 |
| 第2天 | 添加日志 `log_debug_message` | 确认前端收到 |
| 第3天 | 修改渲染条件 `{msg.phase === 'analyzing'\|\|...}` | 测试通过 |
| 第4天 | 添加 `updatePhase` + `return` | 测试通过 |
| 第5天 | 单元测试+集成测试 61个全部通过 | **仍不工作** |

---

## 4. 为什么这么久没解决？

### 4.1 测试能力不足
- 桌面应用（Tauri）不能像 Web 应用一样直接查看 console.log
- 前端日志无法追溯
- 没有自动化 E2E 测试

### 4.2 验证不充分
- 每次修改都依赖用户手动测试
- 自己没有完成完整验证就交付
- 测试通过但实际不工作

### 4.3 方向可能错误
- 可能一直在修改正确的代码
- 真正的问题在别处
- 没有找到真正的根因

---

## 5. 如何避免未来再出现这种问题

### 5.1 测试能力建设

| 措施 | 说明 | 优先级 |
|------|------|--------|
| 前端日志输出到文件 | console.log → 文件日志 | P0 |
| E2E 测试框架 | Playwright 或 Tauri 测试 | P0 |
| CI/CD 自动测试 | 每次提交自动运行测试 | P1 |

### 5.2 验证流程规范

| 规范 | 说明 |
|------|------|
| 自己完成验证 | 修改后必须自己测试通过再交付 |
| 禁止依赖用户调试 | 用户测试是验收，不是调试 |
| 自动化覆盖 | 所有关键流程必须有自动化测试 |

### 5.3 修改前检查清单

- [ ] TypeScript 类型检查通过
- [ ] 单元测试通过
- [ ] 集成测试通过（如有）
- [ ] 前端日志输出已验证
- [ ] 自己手动测试通过

---

## 6. 当前问题如何解决

### 6.1 问题定位：前端日志输出

**原因**：前端 `console.log` 无法在桌面应用中查看

**解决方案**：配置前端日志输出到 `debug.log`

**修改文件**：`src/main.tsx`

```typescript
const fs = require('fs');
const logPath = './debug.log';

const originalLog = console.log;
console.log = (...args) => {
  const log = args.map(a => typeof a === 'object' ? JSON.stringify(a) : String(a)).join(' ');
  fs.appendFileSync(logPath, `[${new Date().toISOString()}] ${log}\n`);
  originalLog.apply(console, args);
};
```

### 6.2 问题定位：State 更新验证

**原因**：无法确认 updatePhase 是否正确执行

**解决方案**：添加每次 state 更新日志

**修改代码**：
```typescript
const updatePhase = (phase: SearchPhase, content: string) => {
  console.log('[STATE] updatePhase called:', { phase, content, loadingId: loadingIdRef.current });
  setSearchPhase(phase);
  setMessages(prev => {
    const newMsgs = prev.map(msg => {
      return msg.id === loadingIdRef.current ? { ...msg, phase, content } : msg;
    });
    console.log('[STATE] messages updated:', JSON.stringify(newMsgs));
    return newMsgs;
  });
};
```

### 6.3 执行计划

| 步骤 | 任务 | 验证方法 |
|------|------|----------|
| 1 | 配置前端日志到文件 | 重启应用，查看 debug.log |
| 2 | 添加每次 state 更新日志 | 输入"你好"，检查日志 |
| 3 | 验证 updatePhase 被调用 | 日志应有 `[STATE] updatePhase called` |
| 4 | 验证 messages 更新 | 日志应有 `[STATE] messages updated` |
| 5 | 根据日志定位真正问题 | 修复并重新验证 |

---

## 7. 教训总结

| 教训 | 说明 |
|------|------|
| 桌面应用不是 Web | console.log 在桌面应用无效，必须配置文件日志 |
| 测试通过 ≠ 功能正常 | 需要端到端验证 |
| 依赖用户测试是耻辱 | 应该自己完成验证 |
| 问题长期不解决 = 方法错误 | 应该停下来重新分析 |

---

## 8. 下一步计划

### 今天必须完成

1. [ ] 配置前端日志输出到 `debug.log`
2. [ ] 添加 state 更新日志
3. [ ] 重启应用，输入"你好"
4. [ ] 检查 `debug.log` 验证问题位置
5. [ ] 根据日志定位真正的问题

### 本周完成

1. [ ] 添加 E2E 测试覆盖意图搜索
2. [ ] 配置 CI/CD 自动测试
3. [ ] 编写意图搜索测试用例文档
