package parser

import (
	"bytes"
	"fmt"
	"net/url"
	"regexp"
	"strings"

	"github.com/PuerkitoBio/goquery"
)

var nyTimesRecipeHeadingPattern = regexp.MustCompile(`^\s*\d+\.`)

type nyTimesCNProfile struct{}

func (nyTimesCNProfile) Match(baseURL *url.URL) bool {
	return strings.EqualFold(baseURL.Hostname(), "cn.nytimes.com")
}

func (nyTimesCNProfile) Parse(baseURL *url.URL, body []byte) (Result, bool, error) {
	doc, err := goquery.NewDocumentFromReader(bytes.NewReader(body))
	if err != nil {
		return Result{}, false, fmt.Errorf("parse nytimes cn HTML: %w", err)
	}

	bodyNode := doc.Find(".article-content .article-body").First()
	if bodyNode.Length() == 0 {
		bodyNode = doc.Find("article .article-body").First()
	}
	if bodyNode.Length() == 0 {
		return Result{}, false, nil
	}

	content := goquery.NewDocumentFromNode(bodyNode.Get(0))
	content.Find("script, style, noscript").Each(func(_ int, selection *goquery.Selection) {
		selection.Remove()
	})
	content.Find("img[data-src]").Each(func(_ int, selection *goquery.Selection) {
		dataSrc := strings.TrimSpace(attrOrEmpty(selection, "data-src"))
		if dataSrc != "" {
			selection.SetAttr("src", dataSrc)
		}
	})

	var builder strings.Builder
	bodyNode.Find(".article-paragraph").Each(func(_ int, paragraph *goquery.Selection) {
		paragraph.Find("script, style, noscript").Each(func(_ int, selection *goquery.Selection) {
			selection.Remove()
		})
		paragraph.Find("img[data-src]").Each(func(_ int, selection *goquery.Selection) {
			dataSrc := strings.TrimSpace(attrOrEmpty(selection, "data-src"))
			if dataSrc != "" {
				selection.SetAttr("src", dataSrc)
			}
		})

		innerHTML, err := paragraph.Html()
		if err != nil {
			return
		}
		innerHTML = strings.TrimSpace(innerHTML)
		text := strings.TrimSpace(paragraph.Text())
		if innerHTML == "" && text == "" {
			return
		}

		tag := "p"
		if isNYTimesSubhead(paragraph) {
			tag = "h2"
			innerHTML = strings.TrimSpace(paragraph.Text())
		} else if nyTimesRecipeHeadingPattern.MatchString(text) {
			tag = "h3"
		}
		builder.WriteString("<")
		builder.WriteString(tag)
		builder.WriteString(">")
		builder.WriteString(innerHTML)
		builder.WriteString("</")
		builder.WriteString(tag)
		builder.WriteString(">")
	})

	html := strings.TrimSpace(builder.String())
	if html == "" {
		return Result{}, false, nil
	}

	resources, err := collectResources(baseURL, html)
	if err != nil {
		return Result{}, false, err
	}

	title := strings.TrimSpace(doc.Find(".article-header h1").First().Text())
	if title == "" {
		title = strings.TrimSpace(doc.Find("meta[name='headline']").AttrOr("content", ""))
	}
	if title == "" {
		title = strings.TrimSpace(doc.Find("title").First().Text())
	}

	return Result{
		HTML:       html,
		Title:      strings.TrimSuffix(title, " - 纽约时报中文网"),
		Resources:  resources,
		HasContent: true,
	}, true, nil
}

func isNYTimesSubhead(paragraph *goquery.Selection) bool {
	text := strings.TrimSpace(paragraph.Text())
	if text == "" || len([]rune(text)) > 40 {
		return false
	}
	if paragraph.ChildrenFiltered("b,strong").Length() == 0 {
		return false
	}

	clone := paragraph.Clone()
	clone.Find("b, strong").Each(func(_ int, selection *goquery.Selection) {
		selection.ReplaceWithHtml(selection.Text())
	})
	return strings.TrimSpace(clone.Text()) == text
}

func attrOrEmpty(selection *goquery.Selection, name string) string {
	value, _ := selection.Attr(name)
	return value
}
