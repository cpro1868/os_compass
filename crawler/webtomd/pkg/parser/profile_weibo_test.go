package parser

import (
	"strings"
	"testing"
)

func TestWeiboArticleProfileExtractsStaticArticle(t *testing.T) {
	t.Parallel()

	body := []byte(`<!doctype html><html><head>
		<title>微博长文标题</title>
	</head><body>
		<article class="article">
			<h1 class="title">微博长文标题</h1>
			<div class="author">作者名</div>
			<div class="WB_editor_iframe_new">
				<p>第一段微博长文正文。</p>
				<p>第二段正文。</p>
				<img src="https://wx1.sinaimg.cn/large/demo.jpg">
			</div>
		</article>
	</body></html>`)

	result, err := Parse("https://weibo.com/ttarticle/x/m/show/id/2309405303156245659656", body)
	if err != nil {
		t.Fatalf("parse weibo article should not fail: %v", err)
	}
	if !result.HasContent {
		t.Fatal("expected content")
	}
	if result.Title != "微博长文标题" {
		t.Fatalf("unexpected title: %q", result.Title)
	}
	if !strings.Contains(result.HTML, "第一段微博长文正文") || !strings.Contains(result.HTML, "第二段正文") {
		t.Fatalf("expected body content, got %q", result.HTML)
	}
	if len(result.Resources) != 1 || result.Resources[0].ResolvedURL != "https://wx1.sinaimg.cn/large/demo.jpg" {
		t.Fatalf("unexpected resources: %#v", result.Resources)
	}
}

func TestWeiboArticleProfileRejectsArticleShellWithoutContent(t *testing.T) {
	t.Parallel()

	body := []byte(`<!doctype html><html><head><title>微博</title></head><body>
		<div id="app"></div>
		<script>window.__CONFIG__ = {}</script>
	</body></html>`)

	_, err := Parse("https://weibo.com/ttarticle/x/m/show/id/2309405303156245659656", body)
	if err == nil {
		t.Fatal("expected empty weibo article shell to fail")
	}
	if !strings.Contains(err.Error(), "微博") || !strings.Contains(err.Error(), "--browser-profile") {
		t.Fatalf("expected weibo cookie guidance error, got %v", err)
	}
}
