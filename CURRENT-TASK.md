# 当前任务

## 任务标题

Phase 1 打磨：github_token 防御性修复 + i18n 覆盖率提升

## 状态

done

## 完成内容

### 1. github_token 根因排查与防御性修复

- [x] 根因分析：`cleanup_undecryptable_secrets()` 和 `get_variable_value_internal()` 在解密失败时静默清空密文，导致密钥临时不匹配时数据永久丢失
- [x] `variables.rs` cleanup 函数：不再清空无法解密的密文，改为仅记录警告日志
- [x] `variables.rs` get_variable_value_internal：解密失败时保留密文，不再执行 UPDATE SET value=''
- [x] `settings.rs` get_setting：解密失败时保留密文，不再清空
- [x] `lib.rs` 启动时添加加密服务验证步骤
- [x] `lib.rs` legacy 密钥迁移路径也调用 cleanup（之前因 early return 跳过）
- [x] 诊断日志：set_system_variable 记录保存操作，get_system_variables 记录解密失败详情

### 2. i18n 覆盖率提升（9/20 → 17/20 组件）

- [x] MoveCategoryDialog 接入 useTranslation
- [x] ManualAddDialog 接入 useTranslation
- [x] ImportConfirmDialog 接入 useTranslation
- [x] MarkdownEditor 接入 useTranslation
- [x] ClonePanel 接入 useTranslation
- [x] ReleasesPanel 接入 useTranslation
- [x] ImportModal 补齐剩余硬编码字符串
- [x] VaultDialog 接入 useTranslation
- [x] ProjectDetailDialog 接入 useTranslation
- [x] SettingsDialog 补齐翻译标签页和代理设置硬编码字符串
- [x] 修复 en.json 重复 detail 块问题
- [x] 翻译资源大幅扩展（markdownEditor 命名空间 + clone/releases/import/manualAdd/moveCategory/importConfirm/settings 扩展）

### 3. 回归验证

- [x] TypeScript 类型检查通过（pnpm tsc --noEmit 无错误）
- [x] release 构建成功
- [x] 产物已复制到 Previous 目录

## 构建命令

```cmd
$env:PATH="D:\Soft\msys64\ucrt64\bin;D:\Soft\msys64\usr\bin;$env:PATH"
pnpm tauri build
```
