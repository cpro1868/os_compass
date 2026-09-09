package parser

import (
	"reflect"
	"strings"
	"testing"
)

func TestNYTimesCNProfilePreservesArticleParagraphsAndSubheads(t *testing.T) {
	t.Parallel()

	body := []byte(`<!doctype html><html><head><title>fallback</title></head><body>
		<article class="article-content">
			<div class="article-header"><header><h1>卷心菜有多健康？</h1></header></div>
			<div class="article-body">
				<div class="article-paragraph"><span>第一段正文<a href="https://example.com/ref">链接</a>。</span></div>
				<div class="article-paragraph"><b>它富含维生素K。</b></div>
				<div class="article-paragraph">小标题后的正文。</div>
				<div class="article-paragraph"><span>1. <a href="https://example.com/recipe">番茄烤卷心菜</a></span></div>
				<div class="article-paragraph"><figure><img data-src="https://static01.nyt.com/images/cover.jpg" src="https://static01.nyt.com/images/cover-low.jpg"></figure></div>
			</div>
		</article>
		<div class="related"><div class="article-paragraph">不应提取的相关推荐</div></div>
	</body></html>`)

	result, err := Parse("https://cn.nytimes.com/health/20260422/cabbage-health-benefits-recipes/", body)
	if err != nil {
		t.Fatalf("parse nytimes cn: %v", err)
	}
	if !result.HasContent {
		t.Fatal("expected content")
	}
	if result.Title != "卷心菜有多健康？" {
		t.Fatalf("unexpected title: %q", result.Title)
	}
	for _, expected := range []string{
		`<p><span>第一段正文<a href="https://example.com/ref">链接</a>。</span></p>`,
		"<h2>它富含维生素K。</h2>",
		"<p>小标题后的正文。</p>",
		`<h3><span>1. <a href="https://example.com/recipe">番茄烤卷心菜</a></span></h3>`,
		`src="https://static01.nyt.com/images/cover.jpg"`,
	} {
		if !strings.Contains(result.HTML, expected) {
			t.Fatalf("expected HTML to contain %q, got %q", expected, result.HTML)
		}
	}
	if strings.Contains(result.HTML, "不应提取") {
		t.Fatalf("expected related content to be excluded, got %q", result.HTML)
	}
	expectedResources := []Resource{{
		Type:        ResourceTypeImage,
		OriginalURL: "https://static01.nyt.com/images/cover.jpg",
		ResolvedURL: "https://static01.nyt.com/images/cover.jpg",
		Attr:        "src",
	}}
	if !reflect.DeepEqual(result.Resources, expectedResources) {
		t.Fatalf("unexpected resources:\n got: %#v\nwant: %#v", result.Resources, expectedResources)
	}
}
