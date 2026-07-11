# 站点配置文件说明

`sites.example.json` 用于演示基于 CSS 选择器的站点提取规则。对于默认 Readability 解析效果不理想的网站，可以通过 `--site-config` 引用配置文件，不必修改 Go 代码。

使用方式：

```powershell
web2md "https://example.com/article" -n article --site-config examples/sites.example.json
```

## 解析优先级

提供 `--site-config` 后，解析顺序如下：

```text
--site-config 中的自定义规则
  -> 内置站点 Profile
  -> 通用 Readability 解析
```

因此，自定义配置可以覆盖同一域名的内置行为。

## 文件结构

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

## 字段说明

- `version`：必填，当前必须为 `1`。
- `sites`：必填，站点规则数组。
- `name`：必填，规则名称，便于排查问题。
- `hosts`：必填，匹配域名，例如 `example.com`。本地测试时也支持 `127.0.0.1:8080` 这类带端口写法。
- `title`：可选，标题 CSS 选择器，按顺序尝试，第一个命中的内容会作为 Markdown 标题。
- `content`：必填，正文 CSS 选择器，按顺序尝试，第一个命中的元素会作为正文。
- `remove`：可选，从正文中删除的 CSS 选择器，例如广告、推荐阅读、评论区。
- `image_attrs`：可选，图片真实地址属性优先级，例如 `data-src`、`data-original`、`src`。
- `video_attrs`：可选，视频真实地址属性优先级。

## 选择器规则

选择器使用 goquery/CSS selector 语法：

```json
{
  "title": ["h1", ".article-title", "#activity-name"],
  "content": ["article", ".post-content", "#js_content"],
  "remove": [".ad", "script", "style"]
}
```

数组顺序很重要。建议把最精确的选择器放前面，把兜底选择器放后面。

## 懒加载图片

很多网站不会把真实图片地址放在 `src`，而是放在 `data-src` 或 `data-original`。可以这样配置：

```json
"image_attrs": ["data-src", "data-original", "src"]
```

程序会取第一个非空属性，并写回 `src`，之后再统一下载和改写 Markdown 链接。

## 常见配置模式

普通博客：

```json
{
  "name": "generic-blog",
  "hosts": ["blog.example.com"],
  "title": ["h1.post-title", "h1"],
  "content": ["article", ".post-content"],
  "remove": [".share", ".related-posts", ".comments"],
  "image_attrs": ["data-src", "src"],
  "video_attrs": ["src"]
}
```

微信公众号类页面：

```json
{
  "name": "wechat",
  "hosts": ["mp.weixin.qq.com"],
  "title": ["#activity-name", "h1"],
  "content": ["#js_content"],
  "remove": ["script", "style"],
  "image_attrs": ["data-src", "data-original", "src"],
  "video_attrs": ["data-src", "src"]
}
```

## 调试方法

如果规则没有生效：

- 确认 URL 的域名和 `hosts` 完全匹配。
- 查看网页 HTML，确认 `content` 选择器能命中真实正文元素。
- 先只写一个最精确的 `content` 选择器，成功后再添加兜底选择器。
- 对广告、推荐阅读、评论区等干扰元素添加 `remove`。
- 图片缺失时检查真实地址是否在 `data-src`、`data-original` 或其他属性中。

真实网站测试建议输出到隔离目录：

```powershell
powershell -ExecutionPolicy Bypass -File scripts/smoke-sites.ps1
```

输出目录为 `test-output/`，该目录是本地产物，不应提交。
