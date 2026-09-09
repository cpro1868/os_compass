# web2md 安装与部署指南

本文说明如何在本机安装、构建和分发 `web2md`。`web2md` 是单文件 CLI 工具，不需要后台服务、数据库或浏览器运行时。

## 环境要求

运行已构建的二进制文件时：

- Windows：直接运行 `web2md.exe`。
- macOS/Linux：运行对应平台构建出的 `web2md`。
- 不需要安装 Go。

从源码构建时：

- Go 1.23 或更新版本。
- 能访问 Go module 代理或 GitHub，用于下载依赖。

## 本地构建

Windows：

```powershell
go mod tidy
go test ./...
go build -o web2md.exe .
```

构建完成后验证：

```powershell
.\web2md.exe --help
.\web2md.exe "https://example.com" -n example
powershell -ExecutionPolicy Bypass -File scripts/smoke-public-url.ps1
```

macOS/Linux：

```bash
go mod tidy
go test ./...
go build -o web2md .
./web2md --help
```

## 跨平台构建

Windows 目标：

```powershell
$env:GOOS="windows"
$env:GOARCH="amd64"
go build -o dist/web2md-windows-amd64.exe .
```

macOS 目标：

```powershell
$env:GOOS="darwin"
$env:GOARCH="arm64"
go build -o dist/web2md-darwin-arm64 .
```

Linux 目标：

```powershell
$env:GOOS="linux"
$env:GOARCH="amd64"
go build -o dist/web2md-linux-amd64 .
```

构建多个平台前先创建 `dist/`：

```powershell
New-Item -ItemType Directory -Force dist
```

## 安装到 PATH

Windows 示例：

1. 创建目录，例如 `C:\Tools\web2md`。
2. 将 `web2md.exe` 放入该目录。
3. 把 `C:\Tools\web2md` 加入系统 `PATH`。
4. 重新打开终端，执行：

```powershell
web2md --help
```

macOS/Linux 示例：

```bash
chmod +x web2md
sudo mv web2md /usr/local/bin/web2md
web2md --help
```

## 使用示例

普通模式：

```powershell
web2md "https://mp.weixin.qq.com/s/Y_uRMYBmdLWUPnz_ac7jWA" -n research-note
```

严格模式：

```powershell
web2md "https://example.com/article" -n article --strict
```

使用站点扩展配置：

```powershell
web2md "https://example.com/article" -n article --site-config examples/sites.example.json
```

输出：

```text
research-note.md
assets/
```

`research-note.md` 中会包含文章标题、原文链接、抓取时间和正文。资源链接会尽量改写为 `./assets/<filename>`。

媒体文件会尽量保留或推断扩展名。对于微信公众号这类常见的无扩展名图片地址，程序会根据 URL 查询参数（如 `wx_fmt=jpeg`）或响应 `Content-Type` 保存为 `.jpg`、`.png` 等文件。

## 分发建议

对外分发时建议只打包以下文件：

```text
web2md.exe
README_CN.md
docs/deploy_CN.md
```

不要打包本地测试生成物，例如：

- `.gocache/`
- `.gomodcache/`
- `.gopath/`
- `test-output/`
- `assets/`
- `wechat-real*.md`
- `wx-*.md`

## 常见问题

### URL 中有 `&` 时命令失败

请用引号包住完整 URL：

```powershell
web2md "https://example.com/a?x=1&y=2" -n note
```

### 微信文章提示验证码或环境校验

这说明目标站点触发了风控。当前版本不会绕过验证码，也不会保存验证码页。可以稍后重试，或在浏览器中确认该链接能无验证访问。

### 图片没有全部离线

默认宽容模式下，下载失败的资源会保留原始远程 URL。使用 `--strict` 可以让任何资源失败都直接中断。

### assets 里文件没有扩展名

请使用最新版本重新抓取。当前版本会为无扩展名资源自动推断扩展名；旧版本已经生成的无扩展名文件不会被自动重命名。

### 输出 Markdown 被覆盖

同名 Markdown 文件会覆盖，这是当前设计。需要保留历史版本时，请更换 `-n` 名称。

## 真实网站 Smoke 测试

真实网站测试统一输出到 `test-output/`，避免污染项目根目录：

```powershell
powershell -ExecutionPolicy Bypass -File scripts/smoke-public-url.ps1
powershell -ExecutionPolicy Bypass -File scripts/smoke-sites.ps1
```

输出示例：

```text
test-output/
  smoke/
  sites/
    example/
    go-blog/
    wechat/
```

真实网站 smoke 依赖网络和目标站点状态，不建议放进 `go test ./...`。
