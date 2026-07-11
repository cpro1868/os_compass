package parser

import (
	"bytes"
	"fmt"
	"html"
	"net/url"
	"regexp"
	"strconv"
	"strings"

	"github.com/PuerkitoBio/goquery"
)

type weChatProfile struct{}

var (
	weChatContentNoEncodePattern = regexp.MustCompile(`content_noencode:\s*'((?:\\'|[^'])*)'`)
	weChatDescPattern            = regexp.MustCompile(`desc:\s*'((?:\\'|[^'])*)'`)
	weChatCDNURLPattern          = regexp.MustCompile(`cdn_url:\s*'([^']*)'`)
)

func (weChatProfile) Match(baseURL *url.URL) bool {
	return strings.EqualFold(baseURL.Host, "mp.weixin.qq.com")
}

func (weChatProfile) Parse(baseURL *url.URL, body []byte) (Result, bool, error) {
	doc, err := goquery.NewDocumentFromReader(bytes.NewReader(body))
	if err != nil {
		return Result{}, false, fmt.Errorf("parse wechat HTML: %w", err)
	}

	if isWeChatImageArticle(body, doc) {
		if result, ok, err := parseWeChatImageArticle(baseURL, doc, body); err != nil || ok {
			return result, ok, err
		}
	}

	content := doc.Find("#js_content").First()
	if content.Length() == 0 {
		content = doc.Find("#js_content_container").First()
	}
	if content.Length() == 0 {
		return Result{}, false, nil
	}

	cleanWeChatContent(content)

	html, err := content.Html()
	if err != nil {
		return Result{}, false, fmt.Errorf("render wechat content: %w", err)
	}
	html = strings.TrimSpace(html)
	if html == "" {
		return Result{}, false, nil
	}
	text := strings.TrimSpace(content.Text())
	if text == "" {
		return Result{}, false, nil
	}

	resources, err := collectResources(baseURL, html)
	if err != nil {
		return Result{}, false, err
	}

	title := strings.TrimSpace(doc.Find("#activity-name").First().Text())
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

func cleanWeChatContent(content *goquery.Selection) {
	content.Find("script, style, noscript, mp-style-type, [style*='display: none'], [style*='display:none']").Each(func(_ int, selection *goquery.Selection) {
		selection.Remove()
	})
	content.Find("img[data-src]").Each(func(_ int, selection *goquery.Selection) {
		dataSrc, ok := selection.Attr("data-src")
		dataSrc = strings.TrimSpace(dataSrc)
		if ok && dataSrc != "" {
			selection.SetAttr("src", dataSrc)
		}
	})
	content.Find("*").Each(func(_ int, selection *goquery.Selection) {
		keepSafeMarkdownAttrs(selection)
	})
}

func keepSafeMarkdownAttrs(selection *goquery.Selection) {
	node := selection.Get(0)
	if node == nil {
		return
	}

	allowed := allowedAttrsForTag(goquery.NodeName(selection))
	filtered := node.Attr[:0]
	for _, attr := range node.Attr {
		if allowed[strings.ToLower(attr.Key)] {
			filtered = append(filtered, attr)
		}
	}
	node.Attr = filtered
}

func allowedAttrsForTag(tag string) map[string]bool {
	switch strings.ToLower(tag) {
	case "a":
		return map[string]bool{"href": true, "title": true}
	case "img":
		return map[string]bool{"src": true, "alt": true, "title": true}
	case "video":
		return map[string]bool{"src": true, "poster": true, "controls": true}
	case "source":
		return map[string]bool{"src": true, "type": true}
	case "td", "th":
		return map[string]bool{"colspan": true, "rowspan": true}
	default:
		return map[string]bool{}
	}
}

func isWeChatImageArticle(body []byte, doc *goquery.Document) bool {
	bodyText := string(body)
	if !strings.Contains(bodyText, "picture_page_info_list") {
		return false
	}
	if strings.Contains(bodyText, "item_show_type = '8'") || strings.Contains(bodyText, "item_show_type: '8'") {
		return true
	}
	return strings.Contains(doc.Text(), "向上滑动看下一个")
}

func parseWeChatImageArticle(baseURL *url.URL, doc *goquery.Document, body []byte) (Result, bool, error) {
	bodyText := string(body)
	scriptContent := strings.TrimSpace(extractWeChatScriptContent(bodyText))
	images := extractWeChatPictureURLs(bodyText)
	if scriptContent == "" && len(images) == 0 {
		return Result{}, false, nil
	}

	var builder strings.Builder
	if looksLikeHTMLFragment(scriptContent) {
		cleanedHTML, err := cleanWeChatHTMLFragment(scriptContent)
		if err != nil {
			return Result{}, false, err
		}
		builder.WriteString(cleanedHTML)
	} else {
		for _, paragraph := range splitWeChatScriptParagraphs(scriptContent) {
			builder.WriteString("<p>")
			builder.WriteString(html.EscapeString(paragraph))
			builder.WriteString("</p>")
		}
	}
	for _, imageURL := range images {
		if strings.Contains(builder.String(), imageURL) {
			continue
		}
		builder.WriteString(`<p><img src="`)
		builder.WriteString(html.EscapeString(imageURL))
		builder.WriteString(`" alt=""></p>`)
	}

	rendered := strings.TrimSpace(builder.String())
	if rendered == "" {
		return Result{}, false, nil
	}

	resources, err := collectResources(baseURL, rendered)
	if err != nil {
		return Result{}, false, err
	}

	title := strings.TrimSpace(doc.Find("#activity-name").First().Text())
	if title == "" {
		title = strings.TrimSpace(doc.Find("meta[property='og:title']").AttrOr("content", ""))
	}
	if title == "" {
		title = strings.TrimSpace(doc.Find("title").First().Text())
	}

	return Result{
		HTML:       rendered,
		Title:      title,
		Resources:  resources,
		HasContent: true,
	}, true, nil
}

func extractWeChatScriptContent(bodyText string) string {
	raw := firstWeChatScriptString(bodyText, weChatContentNoEncodePattern)
	if raw == "" {
		raw = firstWeChatScriptString(bodyText, weChatDescPattern)
	}
	return strings.TrimSpace(decodeWeChatScriptString(raw))
}

func splitWeChatScriptParagraphs(decoded string) []string {
	decoded = strings.TrimSpace(decoded)
	if decoded == "" {
		return nil
	}

	var paragraphs []string
	for _, part := range regexp.MustCompile(`\n\s*\n`).Split(decoded, -1) {
		part = strings.TrimSpace(part)
		if part != "" {
			paragraphs = append(paragraphs, part)
		}
	}
	return paragraphs
}

func looksLikeHTMLFragment(value string) bool {
	lower := strings.ToLower(value)
	for _, marker := range []string{"<p", "<section", "<div", "<h1", "<h2", "<h3", "<figure", "<img"} {
		if strings.Contains(lower, marker) {
			return true
		}
	}
	return false
}

func cleanWeChatHTMLFragment(fragment string) (string, error) {
	doc, err := goquery.NewDocumentFromReader(strings.NewReader(`<div id="web2md-root">` + fragment + `</div>`))
	if err != nil {
		return "", fmt.Errorf("parse wechat script HTML: %w", err)
	}
	root := doc.Find("#web2md-root").First()
	cleanWeChatContent(root)
	html, err := root.Html()
	if err != nil {
		return "", fmt.Errorf("render wechat script HTML: %w", err)
	}
	return strings.TrimSpace(html), nil
}

func firstWeChatScriptString(bodyText string, pattern *regexp.Regexp) string {
	match := pattern.FindStringSubmatch(bodyText)
	if len(match) != 2 {
		return ""
	}
	return match[1]
}

func extractWeChatPictureURLs(bodyText string) []string {
	block := extractWeChatPictureListBlock(bodyText)
	if block == "" {
		return nil
	}

	seen := map[string]bool{}
	var httpsURLs []string
	var otherURLs []string
	for _, cdnMatch := range weChatCDNURLPattern.FindAllStringSubmatch(block, -1) {
		if len(cdnMatch) != 2 {
			continue
		}
		imageURL := strings.TrimSpace(decodeWeChatScriptString(cdnMatch[1]))
		if imageURL == "" || seen[imageURL] {
			continue
		}
		seen[imageURL] = true
		if strings.HasPrefix(imageURL, "https://") {
			httpsURLs = append(httpsURLs, imageURL)
		} else {
			otherURLs = append(otherURLs, imageURL)
		}
	}
	if len(httpsURLs) > 0 {
		return httpsURLs
	}
	return otherURLs
}

func extractWeChatPictureListBlock(bodyText string) string {
	start := strings.Index(bodyText, "picture_page_info_list:")
	if start < 0 {
		return ""
	}
	for _, marker := range []string{"window.appmsgalbuminfo", "window.name", "window.desc"} {
		if end := strings.Index(bodyText[start:], marker); end > 0 {
			return bodyText[start : start+end]
		}
	}
	return bodyText[start:]
}

func decodeWeChatScriptString(raw string) string {
	decoded := regexp.MustCompile(`\\x([0-9a-fA-F]{2})`).ReplaceAllStringFunc(raw, func(token string) string {
		value, err := strconv.ParseInt(token[2:], 16, 32)
		if err != nil {
			return token
		}
		return string(rune(value))
	})
	replacer := strings.NewReplacer(
		`\\`, `\`,
		`\n`, "\n",
		`\r`, "\r",
		`\t`, "\t",
		`\"`, `"`,
		`\'`, `'`,
	)
	return html.UnescapeString(replacer.Replace(decoded))
}
