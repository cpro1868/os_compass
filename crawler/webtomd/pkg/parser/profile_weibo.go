package parser

import (
	"bytes"
	"fmt"
	"net/url"
	"strings"

	"github.com/PuerkitoBio/goquery"
)

type weiboArticleProfile struct{}

func (weiboArticleProfile) Match(baseURL *url.URL) bool {
	host := strings.ToLower(baseURL.Hostname())
	return host == "weibo.com" || host == "www.weibo.com" || host == "card.weibo.com" || host == "m.weibo.cn"
}

func (weiboArticleProfile) Parse(baseURL *url.URL, body []byte) (Result, bool, error) {
	doc, err := goquery.NewDocumentFromReader(bytes.NewReader(body))
	if err != nil {
		return Result{}, false, fmt.Errorf("parse weibo article HTML: %w", err)
	}

	content := firstNonEmptySelection(doc, []string{
		".WB_editor_iframe_new",
		".article-content",
		".article .content",
		".article",
		"article",
	})
	if content == nil || content.Length() == 0 {
		if isWeiboArticleLikeURL(baseURL) {
			return Result{}, false, fmt.Errorf("微博页面未返回可提取正文，可能需要使用 --browser-profile 复用浏览器会话，或最后使用 --cookie")
		}
		return Result{}, false, nil
	}

	content.Find("script, style, noscript").Each(func(_ int, selection *goquery.Selection) {
		selection.Remove()
	})
	content.Find("img[node-type='lazyload'], img[data-src]").Each(func(_ int, selection *goquery.Selection) {
		for _, attr := range []string{"data-src", "src"} {
			value := strings.TrimSpace(selection.AttrOr(attr, ""))
			if value != "" {
				selection.SetAttr("src", value)
				break
			}
		}
	})

	html, err := content.Html()
	if err != nil {
		return Result{}, false, fmt.Errorf("render weibo article content: %w", err)
	}
	html = strings.TrimSpace(html)
	if html == "" || strings.TrimSpace(content.Text()) == "" {
		return Result{}, false, nil
	}

	resources, err := collectResources(baseURL, html)
	if err != nil {
		return Result{}, false, err
	}

	title := strings.TrimSpace(doc.Find(".title, h1").First().Text())
	if title == "" {
		title = strings.TrimSpace(doc.Find("title").First().Text())
	}

	return Result{
		HTML:       html,
		Title:      title,
		Resources:  resources,
		HasContent: true,
	}, true, nil
}

func isWeiboArticleLikeURL(baseURL *url.URL) bool {
	path := strings.ToLower(baseURL.Path)
	return strings.Contains(path, "ttarticle") ||
		strings.Contains(path, "/article/") ||
		strings.Contains(path, "/status/")
}

func firstNonEmptySelection(doc *goquery.Document, selectors []string) *goquery.Selection {
	for _, selector := range selectors {
		selection := doc.Find(selector).First()
		if selection.Length() > 0 && strings.TrimSpace(selection.Text()) != "" {
			return selection
		}
	}
	return nil
}
