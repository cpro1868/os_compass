# Site Config Reference

`sites.example.json` demonstrates selector-based extraction rules for websites whose article layout does not work well with the default Readability parser.

Use it with:

```bash
web2md "https://example.com/article" -n article --site-config examples/sites.example.json
```

## Resolution Order

When `--site-config` is provided, extraction runs in this order:

```text
custom rules from --site-config
  -> built-in site profiles
  -> generic Readability extraction
```

Custom rules can therefore override built-in behavior for the same host.

## File Shape

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

## Fields

- `version`: Required. Must be `1`.
- `sites`: Required. List of site rules.
- `name`: Required. Human-readable rule name used for debugging.
- `hosts`: Required. Hostnames this rule matches, for example `example.com`. Local test hosts with ports such as `127.0.0.1:8080` are also supported.
- `title`: Optional. CSS selectors tried in order. The first matching selector becomes the Markdown title.
- `content`: Required. CSS selectors tried in order. The first matching selector becomes the article body.
- `remove`: Optional. CSS selectors removed from the selected body before conversion.
- `image_attrs`: Optional. Image URL attributes tried in order. Use this for lazy-loaded images such as `data-src`.
- `video_attrs`: Optional. Video URL attributes tried in order.

## Selector Rules

Selectors use goquery/CSS selector syntax:

```json
{
  "title": ["h1", ".article-title", "#activity-name"],
  "content": ["article", ".post-content", "#js_content"],
  "remove": [".ad", "script", "style"]
}
```

For each selector list, order matters. Put the most precise selector first and broader fallbacks later.

## Lazy-Loaded Images

Many sites do not store the real image URL in `src`. Configure priority attributes:

```json
"image_attrs": ["data-src", "data-original", "src"]
```

The first non-empty attribute is copied into `src` before resources are collected and downloaded.

## Common Patterns

Generic blog:

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

WeChat-like article:

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

## Debugging

If a rule does not work:

- Confirm the URL host exactly matches an item in `hosts`.
- Inspect the page HTML and verify `content` selectors match real elements.
- Start with one precise `content` selector, then add fallbacks.
- Add unwanted areas to `remove`.
- For missing images, inspect whether the real URL is stored in `data-src`, `data-original`, or another attribute.

Run real-site checks into an isolated folder:

```powershell
powershell -ExecutionPolicy Bypass -File scripts/smoke-sites.ps1
```

Smoke output is written under `test-output/`.
