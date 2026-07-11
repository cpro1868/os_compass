package parser

import (
	"bytes"
	"fmt"
	"net/url"
	"strings"

	"github.com/PuerkitoBio/goquery"

	"webtomd/pkg/siteconfig"
)

type configProfile struct {
	site siteconfig.Site
}

func (p configProfile) Match(baseURL *url.URL) bool {
	hostname := strings.ToLower(baseURL.Hostname())
	hostWithPort := strings.ToLower(baseURL.Host)
	for _, configured := range p.site.Hosts {
		configured = strings.ToLower(strings.TrimSpace(configured))
		if configured == hostname || configured == hostWithPort {
			return true
		}
	}
	return false
}

func (p configProfile) Parse(baseURL *url.URL, body []byte) (Result, bool, error) {
	doc, err := goquery.NewDocumentFromReader(bytes.NewReader(body))
	if err != nil {
		return Result{}, false, fmt.Errorf("parse configured site %s HTML: %w", p.site.Name, err)
	}

	content := firstMatchingSelection(doc, p.site.Content)
	if content.Length() == 0 {
		return Result{}, false, nil
	}

	for _, selector := range p.site.Remove {
		selector = strings.TrimSpace(selector)
		if selector != "" {
			content.Filter(selector).Remove()
			content.Find(selector).Remove()
		}
	}

	normalizeConfiguredMediaAttrs(content, "img", firstNonEmpty(p.site.ImageAttrs, []string{"src"}))
	normalizeConfiguredMediaAttrs(content, "video", firstNonEmpty(p.site.VideoAttrs, []string{"src"}))
	normalizeConfiguredMediaAttrs(content, "source", firstNonEmpty(p.site.VideoAttrs, []string{"src"}))

	html, err := content.Html()
	if err != nil {
		return Result{}, false, fmt.Errorf("render configured site %s content: %w", p.site.Name, err)
	}
	html = strings.TrimSpace(html)
	if html == "" || strings.TrimSpace(content.Text()) == "" {
		return Result{}, false, nil
	}

	resources, err := collectResources(baseURL, html)
	if err != nil {
		return Result{}, false, err
	}

	title := strings.TrimSpace(firstMatchingSelection(doc, p.site.Title).Text())
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

func firstMatchingSelection(doc *goquery.Document, selectors []string) *goquery.Selection {
	for _, selector := range selectors {
		selector = strings.TrimSpace(selector)
		if selector == "" {
			continue
		}
		selection := doc.Find(selector).First()
		if selection.Length() > 0 {
			return selection
		}
	}
	return doc.Find("__web2md_no_match__")
}

func normalizeConfiguredMediaAttrs(content *goquery.Selection, element string, attrs []string) {
	content.Find(element).Each(func(_ int, selection *goquery.Selection) {
		for _, attr := range attrs {
			attr = strings.TrimSpace(attr)
			if attr == "" {
				continue
			}
			value, ok := selection.Attr(attr)
			value = strings.TrimSpace(value)
			if ok && value != "" {
				selection.SetAttr("src", value)
				return
			}
		}
	})
}
