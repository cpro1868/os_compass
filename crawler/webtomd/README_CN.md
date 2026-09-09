# web2md 中文说明

`web2md` 是一个命令行工具，用于把网页文章保存为本地 Markdown，并自动下载正文中的图片和直链视频资源，适合配合 Obsidian、Typora 等本地笔记工具使用。

## 功能特性

- 将网页正文转换为 Markdown。
- 自动下载正文图片和常见直链视频资源到 `assets/`。
- 自动把正文资源链接改写为本地相对路径。
- 自动从 URL 参数或响应类型推断媒体扩展名，例如微信公众号图片会保存为 `.jpg`、`.png` 等可识别文件。
- 支持微信公众号文章的正文容器提取。
- 支持公开 Notion 页面（`*.notion.site`）的正文 block 提取和附件图片下载。
- 默认宽容模式：资源下载失败时保留原始网络链接。
- `--strict` 严格模式：任一资源下载失败即返回错误。

## 快速使用

```powershell
.\web2md.exe "https://mp.weixin.qq.com/s/Y_uRMYBmdLWUPnz_ac7jWA" -n my-note
```

生成结果：

```text
my-note.md
assets/
```

Markdown 文件格式大致如下：

```markdown
# 文章标题

> 原文链接：https://example.com/article
> 抓取时间：2026-04-21 10:00:00

正文内容...
```

包含 `&`、`?`、`=` 的 URL 必须用引号包住，否则 PowerShell 或其他 shell 可能会截断参数。

## 安装方式

### 方式一：直接使用二进制文件

Windows 下将 `web2md.exe` 放到任意目录，然后在该目录执行：

```powershell
.\web2md.exe "<URL>" -n <文件名>
```

如果希望全局使用，把 `web2md.exe` 所在目录加入系统 `PATH`，之后可直接运行：

```powershell
web2md "<URL>" -n <文件名>
```

### 方式二：从源码构建

需要先安装 Go 1.23 或更新版本。

```powershell
go mod tidy
go build -o web2md.exe .
```

验证：

```powershell
.\web2md.exe --help
go test ./...
```

## 常用命令

```powershell
web2md "<URL>" -n note
web2md "<URL>" -n note --strict
web2md "<URL>" -n note --site-config examples/sites.example.json
web2md "<URL>" -n note --cookie "SUB=xxx; SUBP=yyy"
web2md "<URL>" -n note --browser-profile "$env:LOCALAPPDATA\Google\Chrome\User Data\Default"
go run . "<URL>" -n note
```

参数说明：

- `<URL>`：必填，目标文章 URL。
- `-n, --name`：必填，输出 Markdown 文件名，不含 `.md` 后缀。
- `--strict`：可选，资源下载失败时立即返回错误。
- `--site-config`：可选，引用站点扩展规则 JSON 文件，自定义特殊网站的标题、正文、清理和媒体属性选择器。
- `--cookie`：可选，请求页面时附加 Cookie。适合浏览器能打开、CLI 直接访问会触发登录态或权限校验的页面。
- `--browser-profile`：可选，指定 Chrome/Edge Profile 目录。程序会复制会话文件到临时目录后渲染页面，不会修改原始 Profile。

## 微信公众号说明

微信公众号文章通常可以直接抓取，但仍可能受到微信风控影响：

- 如果触发验证码或环境校验，程序会直接报错，不会保存验证码页。
- 如果网络不稳定，微信页面可能出现 `unexpected EOF` 或超时，重新执行通常可恢复。
- 当前版本不支持登录态、浏览器渲染、扫码验证或付费/受限文章。

## 受限页面、浏览器 Profile 与 Cookie

部分站点会返回 Visitor System、验证码页或“暂无权限查看”。程序会识别这类页面并停止写入 Markdown，避免把风控页当正文保存。

如果目标页面在浏览器中可正常打开，优先使用浏览器 Profile 方式：

```powershell
web2md "<URL>" -n note --browser-profile "$env:LOCALAPPDATA\Google\Chrome\User Data\Default"
```

程序会把 `Local State` 和 Cookie 数据库复制到临时 Profile，再用 Chrome/Edge 渲染页面；原浏览器数据不会被写入。也可以通过环境变量 `WEB2MD_BROWSER_PROFILE_DIR` 指定同一路径。

如果 Profile 方式仍不稳定，再从浏览器开发者工具复制该站点 Cookie，并传给程序：

```powershell
web2md "<URL>" -n note --cookie "SUB=xxx; SUBP=yyy"
```

Cookie 只用于当前命令，不会保存到配置文件。验证码、扫码或付费授权页面仍不支持自动通过。

## Notion 页面说明

Notion 公开页面的静态 HTML 通常只有应用壳，没有正文。`web2md` 对 `*.notion.site` 和带页面 ID 的 `notion.so` URL 使用内置 Notion Profile：

- 通过页面 ID 读取公开 block 数据并转换为 Markdown。
- 支持常见正文块：段落、标题、引用、代码块、列表、分割线、图片和视频直链。
- Notion 附件图片会先换成签名下载地址，再保存到本地 `assets/`。
- 私有页面、需要登录的页面、受限工作区页面不支持。

示例：

```powershell
web2md "https://iyouport.notion.site/S07E05-24c34ca2d46d808985a0f63a22dde6c7" -n notion-s07e05
```

## 输出与覆盖规则

- Markdown 输出路径：`./<name>.md`。
- 资源目录：`./assets/`。
- 同名 Markdown 会被覆盖。
- 已存在的资源文件不会被覆盖，新资源会自动追加 `_1`、`_2` 等后缀。
- 对无扩展名的图片/视频 URL，会优先根据 `wx_fmt`、`format`、`fmt` 等查询参数推断扩展名；没有参数时根据 `Content-Type` 推断。

## 开发验证

```powershell
go test ./...
go build -o web2md.exe .
powershell -ExecutionPolicy Bypass -File scripts/smoke-public-url.ps1
powershell -ExecutionPolicy Bypass -File scripts/smoke-sites.ps1
```

真实网站 smoke 输出统一写入 `test-output/`，避免和项目源码、文档文件混在一起。`test-output/` 是可删除的本地产物目录，不应提交。

开发入口：

- `cmd/`：CLI 参数与命令。
- `pkg/app/`：主流程编排。
- `pkg/fetcher/`：网页抓取。
- `pkg/parser/`：正文和媒体解析。
- `pkg/downloader/`：资源下载。
- `pkg/converter/`：Markdown 转换与格式整理。

## 站点扩展

特殊网站有两种扩展方式：

1. 使用 `--site-config` 引用 JSON 规则文件，适合用户自定义站点。
2. 在 `pkg/parser` 中新增内置 Site Profile，适合需要长期维护的核心站点。

配置规则优先级高于内置 Profile；如果配置没有命中，会回退到内置 Profile，再回退到通用 Readability。

示例配置见 [examples/sites.example.json](examples/sites.example.json)。

配置文件字段说明见 [docs/site_config_CN.md](docs/site_config_CN.md)。内置 Profile 开发说明见 [docs/site_profiles_CN.md](docs/site_profiles_CN.md)。
