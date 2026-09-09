# 站点适配扩展说明

`web2md` 默认使用通用 Readability 逻辑提取正文。对于结构特殊的网站，可以通过配置文件或内置 Profile 增加专用提取逻辑。

## 使用配置文件扩展

配置文件的完整字段说明见 [site_config_CN.md](site_config_CN.md)。英文说明见 [site_config.md](site_config.md)。

命令：

```powershell
web2md "<URL>" -n note --site-config examples/sites.example.json
```

配置优先级：

```text
--site-config 自定义规则
  -> 内置 SiteProfile
  -> Readability 通用解析
```

示例：

```json
{
  "version": 1,
  "sites": [
    {
      "name": "example-blog",
      "hosts": ["example.com", "www.example.com"],
      "title": ["h1.article-title", "h1"],
      "content": ["article", ".post-content", "main"],
      "remove": [".ad", ".related", ".sidebar"],
      "image_attrs": ["data-original", "data-src", "src"],
      "video_attrs": ["src"]
    }
  ]
}
```

字段说明：

- `version`：当前必须为 `1`。
- `name`：站点规则名称，用于排查问题。
- `hosts`：匹配域名，支持 `example.com` 和 `127.0.0.1:8080` 这类带端口写法。
- `title`：标题选择器，按顺序尝试。
- `content`：正文选择器，按顺序尝试，必填。
- `remove`：正文中需要删除的元素选择器。
- `image_attrs`：图片真实地址属性优先级，例如 `data-src`、`data-original`、`src`。
- `video_attrs`：视频真实地址属性优先级。

## 当前流程

```text
parser.Parse()
  -> 依次尝试传入的配置型 SiteProfile
  -> 依次尝试内置 SiteProfile
  -> 未命中或不适用：回退到 Readability
```

当前内置 Profile：

- `mp.weixin.qq.com`：优先提取 `#js_content`，标题取 `#activity-name`，图片优先使用 `data-src`。
- `*.notion.site`、带页面 ID 的 `notion.so`：静态 HTML 无正文时，通过公开 Notion block 数据接口提取段落、标题、引用、代码、列表、图片和视频直链；Notion 附件图片会转换为签名 URL 后下载。

## Notion Profile 说明

Notion 公开页面不是传统文章页，服务端返回的 HTML 多数只有 `<div id="notion-app"></div>` 和页面元信息，CSS selector 规则无法直接提取正文。因此 Notion 适配使用内置 Profile，而不是 `--site-config`。

支持范围：

- 公开可访问的 `*.notion.site` 页面。
- `notion.so` URL 中包含 32 位页面 ID 的公开页面。
- 常见 block：正文、标题、引用、代码、项目列表、编号列表、待办、分割线、图片、视频直链。

不支持范围：

- 私有页面、登录态页面、工作区权限限制页面。
- 需要浏览器执行脚本、验证码或人工交互后才能访问的页面。
- Notion 数据接口变更后的兼容保证。

## 新增内置站点 Profile

在 `pkg/parser/` 下新增文件，例如：

```text
pkg/parser/profile_example.go
```

实现接口：

```go
type SiteProfile interface {
    Match(baseURL *url.URL) bool
    Parse(baseURL *url.URL, body []byte) (Result, bool, error)
}
```

约定：

- `Match` 只判断域名或路径是否适用。
- `Parse` 返回 `(Result, true, nil)` 表示已成功提取。
- `Parse` 返回 `(Result{}, false, nil)` 表示该 Profile 不适用，允许继续 fallback。
- 站点解析失败时返回明确错误。

新增 Profile 后，在 `pkg/parser/profiles.go` 的 `builtinProfiles` 中注册：

```go
var builtinProfiles = []SiteProfile{
    weChatProfile{},
    exampleProfile{},
}
```

## 测试要求

每个站点 Profile 应至少包含一个 fixture 测试，覆盖：

- 正文提取完整性。
- 标题提取。
- 图片或视频资源提取。
- 站点特有 lazy-load 属性，例如 `data-src`。

真实网站 smoke 不应放进 `go test ./...`，因为网络、反爬和页面更新会导致不稳定。真实网站验证统一使用：

```powershell
powershell -ExecutionPolicy Bypass -File scripts/smoke-sites.ps1
```

输出统一进入：

```text
test-output/sites/
```

该目录是本地测试产物，不应提交。
