package converter

import (
	"strings"
	"testing"
	"time"
)

func TestConvertAddsFrontmatterAndMarkdown(t *testing.T) {
	doc := Document{
		OriginalURL: `https://example.com/articles/demo?title="quoted"#section: intro`,
		FetchDate:   time.Date(2026, 4, 19, 15, 42, 7, 0, time.UTC),
		Title:       "Demo Article",
		HTML: `<article>
<p>Read the <a href="./assets/local-image.png">local asset</a>.</p>
<img src="./assets/cover.png" alt="Cover">
</article>`,
		HasContent: true,
	}

	markdown, err := Convert(doc)
	if err != nil {
		t.Fatalf("Convert returned error: %v", err)
	}

	if !strings.HasPrefix(markdown, "# Demo Article\n\n> 原文链接：https://example.com/articles/demo?title=\"quoted\"#section: intro\n> 抓取时间：2026-04-19 15:42:07\n\n") {
		t.Fatalf("markdown metadata block missing or malformed:\n%s", markdown)
	}
	if !strings.Contains(markdown, "# Demo Article") {
		t.Fatalf("markdown missing converted heading:\n%s", markdown)
	}
	if !strings.Contains(markdown, "[local asset](./assets/local-image.png)") {
		t.Fatalf("markdown missing local asset link:\n%s", markdown)
	}
	if !strings.Contains(markdown, "![Cover](./assets/cover.png)") {
		t.Fatalf("markdown missing converted image link:\n%s", markdown)
	}
	if !strings.HasSuffix(markdown, "\n") {
		t.Fatalf("markdown should end with a trailing newline")
	}
}

func TestConvertNormalizesBlockquotePrefixSpacing(t *testing.T) {
	doc := Document{
		OriginalURL: "https://example.com/articles/quote",
		FetchDate:   time.Date(2026, 4, 19, 18, 0, 0, 0, time.UTC),
		Title:       "Quote Article",
		HTML:        `<blockquote>/ 作者：卡兹克</blockquote>`,
		HasContent:  true,
	}

	markdown, err := Convert(doc)
	if err != nil {
		t.Fatalf("Convert returned error: %v", err)
	}

	if !strings.Contains(markdown, "\n> / 作者：卡兹克\n") {
		t.Fatalf("blockquote prefix should be normalized with a space:\n%s", markdown)
	}
}

func TestConvertPreservesHTMLTableAsMarkdownTable(t *testing.T) {
	doc := Document{
		OriginalURL: "https://example.com/articles/table",
		FetchDate:   time.Date(2026, 4, 28, 15, 0, 0, 0, time.UTC),
		Title:       "Table Article",
		HTML: `<table>
<tr><th>阶段</th><th>作用</th><th>关键要求</th></tr>
<tr><td>文件解析</td><td>解析 Word、PDF、扫描件、表格</td><td>保留页码、标题、表格位置</td></tr>
</table>`,
		HasContent: true,
	}

	markdown, err := Convert(doc)
	if err != nil {
		t.Fatalf("Convert returned error: %v", err)
	}

	for _, want := range []string{
		"| 阶段 | 作用 | 关键要求 |",
		"| --- | --- | --- |",
		"| 文件解析 | 解析 Word、PDF、扫描件、表格 | 保留页码、标题、表格位置 |",
	} {
		if !strings.Contains(markdown, want) {
			t.Fatalf("markdown missing table row %q:\n%s", want, markdown)
		}
	}
}

func TestNormalizeMarkdownPrettifiesDenseCodeBlock(t *testing.T) {
	input := "```\n# 横纵分析法 Deep Research Prompt> 使用方法---## Prompt 正文---### 一、纵向分析1. **起源追溯**：A2. **诞生节点**：B- **场景A**：C\n```"
	got := normalizeMarkdown(input)

	for _, want := range []string{
		"Prompt\n> 使用方法",
		"\n## Prompt 正文",
		"\n### 一、纵向分析",
		"\n1. **起源追溯**：A",
		"\n2. **诞生节点**：B",
		"\n- **场景A**：C",
	} {
		if !strings.Contains(got, want) {
			t.Fatalf("expected normalized code block to contain %q, got:\n%s", want, got)
		}
	}
}

func TestConvertEmptyArticleWritesOriginalLink(t *testing.T) {
	doc := Document{
		OriginalURL: "https://example.com/articles/empty",
		FetchDate:   time.Date(2026, 4, 19, 16, 5, 30, 0, time.UTC),
		HasContent:  false,
	}

	markdown, err := Convert(doc)
	if err != nil {
		t.Fatalf("Convert returned error: %v", err)
	}

	if !strings.Contains(markdown, "\u539f\u6587\u94fe\u63a5\uff1ahttps://example.com/articles/empty") {
		t.Fatalf("markdown missing original link fallback:\n%s", markdown)
	}
	if !strings.HasPrefix(markdown, "> 原文链接：https://example.com/articles/empty\n> 抓取时间：2026-04-19 16:05:30\n\n") {
		t.Fatalf("markdown metadata quote missing or malformed:\n%s", markdown)
	}
	if !strings.HasSuffix(markdown, "\n") {
		t.Fatalf("markdown should end with a trailing newline")
	}
}

func TestConvertUsesFallbackWhenHasContentFalseWithHTML(t *testing.T) {
	doc := Document{
		OriginalURL: "https://example.com/articles/rejected",
		FetchDate:   time.Date(2026, 4, 19, 16, 10, 0, 0, time.UTC),
		HTML:        "<h1>This should not be converted</h1>",
		HasContent:  false,
	}

	markdown, err := Convert(doc)
	if err != nil {
		t.Fatalf("Convert returned error: %v", err)
	}

	if !strings.Contains(markdown, "\u539f\u6587\u94fe\u63a5\uff1ahttps://example.com/articles/rejected") {
		t.Fatalf("markdown missing original link fallback:\n%s", markdown)
	}
	if strings.Contains(markdown, "This should not be converted") {
		t.Fatalf("markdown converted HTML even though HasContent is false:\n%s", markdown)
	}
}

func TestConvertUsesFallbackWhenConvertedMarkdownIsEmpty(t *testing.T) {
	doc := Document{
		OriginalURL: "https://example.com/articles/empty-conversion",
		FetchDate:   time.Date(2026, 4, 19, 16, 11, 0, 0, time.UTC),
		HTML:        "  \n\t  ",
		HasContent:  true,
	}

	markdown, err := Convert(doc)
	if err != nil {
		t.Fatalf("Convert returned error: %v", err)
	}

	if !strings.Contains(markdown, "\u539f\u6587\u94fe\u63a5\uff1ahttps://example.com/articles/empty-conversion") {
		t.Fatalf("markdown missing original link fallback for empty conversion:\n%s", markdown)
	}
}
