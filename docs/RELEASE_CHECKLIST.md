# OS-Compass 安全发布规范与检查清单 (RELEASE_CHECKLIST)

> 依据：`docs/安全工程方法论.md`  
> 适用对象：版本维护者、发布流水线、Release 审核人

---

## 一、发布前必检红线（自动化拦截）

发布流程已由 `scripts/release.ps1` 和 `.github/workflows/release.yml` 自动化执行下列规则：

| 检查项 | 规则说明 | 判定与后果 |
|---|---|---|
| **禁止硬编码凭据** | 源码中严禁出现 `sk-[a-zA-Z0-9]{20,}` 或 `ghp_[a-zA-Z0-9]{20,}` 等高熵 API Key | 命中立即终止发布，必须清除并轮换泄露的 Key |
| **禁止硬编码代理** | 严禁在 Rust/TS 源码中写入具体的本地开发端口（如 `127.0.0.1:8964`、`7890`） | 命中立即终止，代理地址一律由用户在界面设置 |
| **单点版本对齐** | `Cargo.toml`、`tauri.conf.json`、`package.json` 版本号必须 100% 严格一致 | 通过 `pnpm release:sync-version` 自动对齐 |
| **单元测试与类型** | `pnpm test` (78+ tests) 与 `pnpm typecheck` 必须全绿 | 任何失败直接阻断发布打包 |

---

## 二、标准化发布操作流程

### 1. 快速一键发布命令（本地准备 + 打 Tag）

```bash
# 自动同步版本、静态安全合规扫描、全量测试、更新安装包并创建 Git Tag
powershell scripts/release.ps1 -Version 0.1.1
```

### 2. 推送到远程仓库触发 CI/CD 自动发布

```bash
# 确认无误后推送主分支与新版本标签
git push origin master
git push origin v0.1.1
```

推送后，GitHub Actions 的 `Release` 工作流将自动：
1. 在干净的 Windows 环境下执行二次静态安全审计；
2. 自动化执行类型检查与单元测试；
3. 编译 Windows MSI 安装包与 NSIS Setup 安装包；
4. 自动创建并发布 GitHub Release，并将安装包挂载供全球用户下载。

---

## 三、版本号升级与 CHANGELOG 维护

遵循 [Semantic Versioning (语义化版本)](https://semver.org/lang/zh-CN/) 规范：
- **MAJOR（主版本）**：发生重大数据库结构断代或架构重塑时（如 1.0.0）；
- **MINOR（次版本）**：新增功能模块（如新增平台适配器、新增定时采集等，0.2.0）；
- **PATCH（修订版本）**：日常 Bug 修复与体验调优（如 0.1.1、0.1.2）。

每次发布前，请简要更新 `CHANGELOG.md` 中对应版本的变更说明。
