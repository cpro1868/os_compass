# 当前任务

## 任务标题

工程化改进：Git 仓库初始化 + 测试框架 + CI

## 状态

done

## 完成内容

### 1. Git 仓库初始化
- [x] 创建 .gitignore（排除 node_modules、target、*.db、*.cryptokey、.env、Previous/、session-*.md 等）
- [x] 初始化 Git 仓库，首次提交全部源码
- [x] 移除 crawler/webtomd 内嵌 .git，改为直接纳入主仓库

### 2. 测试框架（vitest）
- [x] 安装 vitest + @testing-library/react + jsdom
- [x] 创建 vitest.config.ts（jsdom 环境）
- [x] 新增 types.test.ts（13 个测试）
- [x] 新增 i18n.test.ts（5 个测试，zh/en 键一致性校验）
- [x] 修复 i18n 测试发现的 en.json 缺失 detail.aiAnalysis 键
- [x] package.json 新增 test/test:watch/typecheck 脚本

### 3. CI 配置
- [x] 创建 .github/workflows/ci.yml（GitHub Actions）
- [x] 创建 scripts/pre-commit.ps1（本地提交前检查）

### 4. 文档更新
- [x] AGENTS.md 新增第 7 节：开发命令和 Git 规范

## Git 提交历史

- `28a7d9c` chore: initial commit - Phase 1 MVP complete (M1-M12)
- `709880f` fix: add crawler/webtomd source files
- `2201846` chore: add test framework (vitest) + CI + pre-commit script

## 构建命令

```cmd
$env:PATH="D:\Soft\msys64\ucrt64\bin;D:\Soft\msys64\usr\bin;$env:PATH"
cd "G:\Projects\kimicode\os_compass\os-compass"
pnpm tauri build
```
