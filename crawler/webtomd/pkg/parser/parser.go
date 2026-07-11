package parser

import (
	"bytes"
	"fmt"
	"net/url"
	"path"
	"strings"

	"github.com/PuerkitoBio/goquery"
	readability "github.com/go-shiori/go-readability"
)

type ResourceType string

const (
	ResourceTypeImage ResourceType = "image"
	ResourceTypeVideo ResourceType = "video"
)

type Resource struct {
	Type        ResourceType
	OriginalURL string
	ResolvedURL string
	Attr        string
}

type Result struct {
	HTML       string
	Title      string
	Resources  []Resource
	HasContent bool
}

func Parse(pageURL string, body []byte) (Result, error) {
	baseURL, err := url.Parse(pageURL)
	if err != nil {
		return Result{}, fmt.Errorf("parse page URL: %w", err)
	}

	if result, ok, err := trySiteProfiles(baseURL, body); err != nil {
		return Result{}, err
	} else if ok {
		return result, nil
	}

	article, err := readability.FromReader(bytes.NewReader(body), baseURL)
	if err != nil {
		return Result{HasContent: false}, nil
	}

	html := strings.TrimSpace(article.Content)
	if html == "" || strings.TrimSpace(article.TextContent) == "" {
		return Result{Title: article.Title, HasContent: false}, nil
	}

	resources, err := collectResources(baseURL, html)
	if err != nil {
		return Result{}, err
	}

	return Result{
		HTML:       html,
		Title:      article.Title,
		Resources:  resources,
		HasContent: true,
	}, nil
}

func collectResources(baseURL *url.URL, html string) ([]Resource, error) {
	doc, err := goquery.NewDocumentFromReader(strings.NewReader(html))
	if err != nil {
		return nil, fmt.Errorf("parse extracted HTML: %w", err)
	}

	var resources []Resource
	doc.Find("img[src], video[src], video source[src]").EachWithBreak(func(_ int, selection *goquery.Selection) bool {
		src, ok := selection.Attr("src")
		src = strings.TrimSpace(src)
		if !ok || src == "" || IsFragmentOnlyURL(src) {
			return true
		}

		resourceType := ResourceTypeImage
		nodeName := goquery.NodeName(selection)
		if nodeName == "video" || nodeName == "source" {
			if !IsDirectVideoURL(src) {
				return true
			}
			resourceType = ResourceTypeVideo
		}

		resolved, resolveErr := ResolveURL(baseURL, src)
		if resolveErr != nil {
			err = resolveErr
			return false
		}
		if !IsDownloadableURL(resolved) {
			return true
		}

		resources = append(resources, Resource{
			Type:        resourceType,
			OriginalURL: src,
			ResolvedURL: resolved,
			Attr:        "src",
		})
		return true
	})
	if err != nil {
		return nil, err
	}

	return resources, nil
}

func RewriteResources(html string, replacements map[string]string) (string, error) {
	doc, err := goquery.NewDocumentFromReader(strings.NewReader(html))
	if err != nil {
		return "", fmt.Errorf("parse HTML for resource rewrite: %w", err)
	}

	doc.Find("img[src], video[src], video source[src]").Each(func(_ int, selection *goquery.Selection) {
		src, ok := selection.Attr("src")
		if !ok {
			return
		}
		if replacement, exists := replacements[src]; exists {
			selection.SetAttr("src", replacement)
			return
		}
		if replacement, exists := replacements[strings.TrimSpace(src)]; exists {
			selection.SetAttr("src", replacement)
		}
	})

	rewritten, err := doc.Find("body").Html()
	if err != nil {
		return "", fmt.Errorf("render rewritten HTML: %w", err)
	}
	if strings.TrimSpace(rewritten) == "" {
		rewritten, err = doc.Html()
		if err != nil {
			return "", fmt.Errorf("render rewritten HTML fallback: %w", err)
		}
	}

	return rewritten, nil
}

func ResolveURL(baseURL *url.URL, rawURL string) (string, error) {
	parsed, err := url.Parse(strings.TrimSpace(rawURL))
	if err != nil {
		return "", fmt.Errorf("parse resource URL %q: %w", rawURL, err)
	}

	return baseURL.ResolveReference(parsed).String(), nil
}

func IsDownloadableURL(rawURL string) bool {
	parsed, err := url.Parse(strings.TrimSpace(rawURL))
	if err != nil {
		return false
	}

	return parsed.Scheme == "http" || parsed.Scheme == "https"
}

func IsFragmentOnlyURL(rawURL string) bool {
	parsed, err := url.Parse(strings.TrimSpace(rawURL))
	if err != nil {
		return false
	}

	return parsed.Scheme == "" && parsed.Host == "" && parsed.Path == "" && parsed.RawQuery == "" && parsed.Fragment != ""
}

func IsDirectVideoURL(rawURL string) bool {
	parsed, err := url.Parse(strings.TrimSpace(rawURL))
	if err != nil {
		return false
	}

	switch strings.ToLower(path.Ext(parsed.Path)) {
	case ".mp4", ".webm", ".mov", ".m4v":
		return true
	default:
		return false
	}
}
