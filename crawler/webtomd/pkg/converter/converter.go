package converter

import (
	"fmt"
	"regexp"
	"strings"
	"time"

	md "github.com/JohannesKaufmann/html-to-markdown"
	"github.com/JohannesKaufmann/html-to-markdown/plugin"
)

type Document struct {
	OriginalURL string
	FetchDate   time.Time
	Title       string
	HTML        string
	HasContent  bool
}

func Convert(doc Document) (string, error) {
	body := fallbackBody(doc.OriginalURL)
	if doc.HasContent {
		converter := md.NewConverter("", true, nil)
		converter.Use(plugin.Table())

		markdown, err := converter.ConvertString(doc.HTML)
		if err != nil {
			return "", err
		}
		if trimmed := strings.TrimSpace(markdown); trimmed != "" {
			body = trimmed
		}
	}
	body = normalizeMarkdown(body)
	metadata := metadataQuote(doc.OriginalURL, doc.FetchDate)
	heading := titleHeading(doc.Title, body)

	var result string
	if heading != "" {
		result = fmt.Sprintf("%s\n\n%s\n\n%s\n", heading, metadata, body)
	} else {
		result = fmt.Sprintf("%s\n\n%s\n", metadata, body)
	}

	return result, nil
}

func titleHeading(title string, body string) string {
	title = strings.TrimSpace(title)
	if title == "" {
		return ""
	}
	trimmedBody := strings.TrimSpace(body)
	if strings.HasPrefix(trimmedBody, "# "+title) {
		return ""
	}
	return "# " + title
}

func normalizeMarkdown(markdown string) string {
	lines := strings.Split(markdown, "\n")
	inFence := false
	for i, line := range lines {
		trimmed := strings.TrimLeft(line, " \t")
		if strings.HasPrefix(strings.TrimSpace(line), "```") {
			inFence = !inFence
			lines[i] = line
			continue
		}
		if inFence {
			line = prettifyDenseCodeLine(line)
			trimmed = strings.TrimLeft(line, " \t")
		}
		if strings.HasPrefix(trimmed, ">/") {
			lines[i] = strings.Replace(line, ">/", "> /", 1)
			continue
		}
		lines[i] = line
	}
	return strings.Join(lines, "\n")
}

var (
	reHeading2 = regexp.MustCompile(`([^#\n])##\s`)
	reHeading3 = regexp.MustCompile(`([^#\n])###\s`)
	reNumList  = regexp.MustCompile(`([^\n])([1-9]\.\s\*\*)`)
	reBullet   = regexp.MustCompile(`([^\n])(-\s\*\*)`)
	reDenseHr  = regexp.MustCompile(`([^\n])---([^\n])`)
	reFenceMid = regexp.MustCompile("([^\n])```([^\n])")
)

func prettifyDenseCodeLine(line string) string {
	normalized := strings.ReplaceAll(line, "\u00a0", " ")
	normalized = strings.ReplaceAll(normalized, "```>", "```\n>")
	normalized = strings.ReplaceAll(normalized, "Prompt> 使用方法", "Prompt\n> 使用方法\n")
	normalized = strings.ReplaceAll(normalized, "Prompt>", "Prompt\n> ")
	normalized = strings.ReplaceAll(normalized, "使用方法---", "使用方法\n---")
	normalized = strings.ReplaceAll(normalized, "。---", "。\n---")
	normalized = strings.ReplaceAll(normalized, "）---", "）\n---")
	normalized = reHeading3.ReplaceAllString(normalized, "$1\n### ")
	normalized = reHeading2.ReplaceAllString(normalized, "$1\n## ")
	normalized = reNumList.ReplaceAllString(normalized, "$1\n$2")
	normalized = reBullet.ReplaceAllString(normalized, "$1\n$2")
	normalized = reDenseHr.ReplaceAllString(normalized, "$1\n---\n$2")
	normalized = reFenceMid.ReplaceAllString(normalized, "$1\n```\n$2")
	return normalized
}

func fallbackBody(originalURL string) string {
	return fmt.Sprintf("\u539f\u6587\u94fe\u63a5\uff1a%s", originalURL)
}

func metadataQuote(originalURL string, fetchDate time.Time) string {
	return fmt.Sprintf("> 原文链接：%s\n> 抓取时间：%s", originalURL, fetchDate.Format("2006-01-02 15:04:05"))
}
