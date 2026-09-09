package parser

import (
	"net/url"
	"os"
	"reflect"
	"strings"
	"testing"
	"time"

	"webtomd/pkg/converter"
)

func TestParseExtractsArticleMedia(t *testing.T) {
	t.Parallel()

	body, err := os.ReadFile("../../testdata/article.html")
	if err != nil {
		t.Fatalf("read fixture: %v", err)
	}

	result, err := Parse("https://example.com/posts/readable-story", body)
	if err != nil {
		t.Fatalf("parse should not fail: %v", err)
	}

	if !result.HasContent {
		t.Fatal("expected readable article content")
	}
	if !strings.Contains(result.HTML, "The durable web page keeps the essential paragraph") {
		t.Fatalf("expected article body text in cleaned HTML: %q", result.HTML)
	}
	if strings.Contains(result.HTML, "Navigation clutter") {
		t.Fatalf("expected navigation clutter to be removed: %q", result.HTML)
	}
	if strings.Contains(result.HTML, "Footer clutter") {
		t.Fatalf("expected footer clutter to be removed: %q", result.HTML)
	}
	expectedResources := []Resource{
		{
			Type:        ResourceTypeImage,
			OriginalURL: "https://example.com/media/cover.png",
			ResolvedURL: "https://example.com/media/cover.png",
			Attr:        "src",
		},
		{
			Type:        ResourceTypeVideo,
			OriginalURL: "https://cdn.example.com/videos/story.mp4",
			ResolvedURL: "https://cdn.example.com/videos/story.mp4",
			Attr:        "src",
		},
		{
			Type:        ResourceTypeVideo,
			OriginalURL: "https://example.com/media/clip.webm",
			ResolvedURL: "https://example.com/media/clip.webm",
			Attr:        "src",
		},
	}
	if !reflect.DeepEqual(result.Resources, expectedResources) {
		t.Fatalf("unexpected resources:\n got: %#v\nwant: %#v", result.Resources, expectedResources)
	}
}

func TestCollectResourcesIgnoresEmptyAndNonDownloadableSources(t *testing.T) {
	t.Parallel()

	baseURL := mustParseURL(t, "https://example.com/posts/story")
	resources, err := collectResources(baseURL, `
		<article>
			<img src="">
			<img src="   ">
			<img src="data:image/png;base64,abcd">
			<img src="blob:https://example.com/blob-id">
			<img src="javascript:alert(1)">
			<img src="mailto:editor@example.com">
			<img src="#cover">
			<img src="/media/cover.png">
			<video src="/media/movie.mp4"></video>
			<video src="blob:https://example.com/movie.webm"></video>
			<video><source src="clip.webm"></video>
		</article>
	`)
	if err != nil {
		t.Fatalf("collect resources: %v", err)
	}

	expectedResources := []Resource{
		{
			Type:        ResourceTypeImage,
			OriginalURL: "/media/cover.png",
			ResolvedURL: "https://example.com/media/cover.png",
			Attr:        "src",
		},
		{
			Type:        ResourceTypeVideo,
			OriginalURL: "/media/movie.mp4",
			ResolvedURL: "https://example.com/media/movie.mp4",
			Attr:        "src",
		},
		{
			Type:        ResourceTypeVideo,
			OriginalURL: "clip.webm",
			ResolvedURL: "https://example.com/posts/clip.webm",
			Attr:        "src",
		},
	}
	if !reflect.DeepEqual(resources, expectedResources) {
		t.Fatalf("unexpected resources:\n got: %#v\nwant: %#v", resources, expectedResources)
	}
}

func TestParseEmptyReturnsNoContent(t *testing.T) {
	t.Parallel()

	body, err := os.ReadFile("../../testdata/empty.html")
	if err != nil {
		t.Fatalf("read fixture: %v", err)
	}

	result, err := Parse("https://example.com/app", body)
	if err != nil {
		t.Fatalf("empty shell should not hard fail: %v", err)
	}
	if result.HasContent {
		t.Fatalf("expected no content, got result: %#v", result)
	}
}

func TestParseWeChatUsesJSContentAndDataSrc(t *testing.T) {
	t.Parallel()

	body := []byte(`<!doctype html><html><head><title>fallback title</title></head><body>
		<h1 id="activity-name">微信正文标题</h1>
		<div id="js_content">
			<p>这是微信文章第一段正文，应该被完整提取。</p>
			<p>这是微信文章第二段正文，不应该只剩滑动提示。</p>
			<img src="https://mmbiz.qpic.cn/placeholder/0?wx_fmt=gif" data-src="https://mmbiz.qpic.cn/mmbiz_jpg/abc123/640?wx_fmt=jpeg" />
		</div>
		<div>继续滑动看下一个</div>
	</body></html>`)

	result, err := Parse("https://mp.weixin.qq.com/s/example", body)
	if err != nil {
		t.Fatalf("parse wechat should not fail: %v", err)
	}
	if !result.HasContent {
		t.Fatal("expected wechat content to be extracted")
	}
	if result.Title != "微信正文标题" {
		t.Fatalf("unexpected title: %q", result.Title)
	}
	if !strings.Contains(result.HTML, "这是微信文章第一段正文") {
		t.Fatalf("expected full wechat body text in html, got %q", result.HTML)
	}
	expectedResources := []Resource{
		{
			Type:        ResourceTypeImage,
			OriginalURL: "https://mmbiz.qpic.cn/mmbiz_jpg/abc123/640?wx_fmt=jpeg",
			ResolvedURL: "https://mmbiz.qpic.cn/mmbiz_jpg/abc123/640?wx_fmt=jpeg",
			Attr:        "src",
		},
	}
	if !reflect.DeepEqual(result.Resources, expectedResources) {
		t.Fatalf("unexpected resources:\n got: %#v\nwant: %#v", result.Resources, expectedResources)
	}
}

func TestParseWeChatCleansStyledArticleHTMLForMarkdown(t *testing.T) {
	t.Parallel()

	body := []byte(`<!doctype html><html><head><title>fallback title</title></head><body>
		<h1 id="activity-name">微信样式标题</h1>
		<div id="js_content">
			<h3 data-first-child="" style="font-family: -apple-system, BlinkMacSystemFont, "Helvetica Neue"; color: rgb(25, 27, 31);"><span leaf="">一、</span></h3>
			<p data-pid="abc" style="margin: 1.4em 0px;"><span leaf=""><span textstyle="" style="font-size: 17px;">第一段正文。</span></span></p>
			<figure data-size="normal" style="margin: 1.4em 0px;"><section nodeleaf=""><img data-src="https://mmbiz.qpic.cn/cover/640?wx_fmt=jpeg" data-type="jpeg" style="width: 654px;"></section><figcaption style="text-align:center;"><span leaf="">图片说明</span></figcaption></figure>
			<p style="display: none;"><mp-style-type data-value="3"></mp-style-type></p>
		</div>
	</body></html>`)

	result, err := Parse("https://mp.weixin.qq.com/s/example", body)
	if err != nil {
		t.Fatalf("parse wechat should not fail: %v", err)
	}
	if !result.HasContent {
		t.Fatal("expected wechat content")
	}
	if strings.Contains(result.HTML, "style=") || strings.Contains(result.HTML, "data-pid") || strings.Contains(result.HTML, "data-first-child") {
		t.Fatalf("expected cleaned wechat html, got %q", result.HTML)
	}
	if !strings.Contains(result.HTML, `src="https://mmbiz.qpic.cn/cover/640?wx_fmt=jpeg"`) {
		t.Fatalf("expected data-src promoted to src, got %q", result.HTML)
	}

	markdown, err := converter.Convert(converter.Document{
		OriginalURL: "https://mp.weixin.qq.com/s/example",
		FetchDate:   time.Date(2026, 6, 4, 0, 0, 0, 0, time.Local),
		Title:       result.Title,
		HTML:        result.HTML,
		HasContent:  result.HasContent,
	})
	if err != nil {
		t.Fatalf("convert should not fail: %v", err)
	}
	if strings.Contains(markdown, "<h3") || strings.Contains(markdown, "<p") || strings.Contains(markdown, "style=") {
		t.Fatalf("expected markdown conversion without raw styled HTML, got %q", markdown)
	}
	if !strings.Contains(markdown, "### 一、") || !strings.Contains(markdown, "第一段正文。") {
		t.Fatalf("expected markdown headings and body text, got %q", markdown)
	}
}

func TestParseWeChatImageArticleUsesPicturePageInfo(t *testing.T) {
	t.Parallel()

	body := []byte(`<!doctype html><html><head><title>fallback title</title></head><body>
		<h1 id="activity-name">图片文章标题</h1>
		<div id="js_content">
			<img src="https://mmbiz.qpic.cn/cover/0?wx_fmt=png">
			<span class="wx_stream_article_slide_tip_text">向上滑动看下一个</span>
		</div>
		<script>
			window.item_show_type = '8';
			desc: '第一段\x0a\x0a第二段\x26quot;引用\x26quot;',
			content_noencode: '第一段\x0a\x0a第二段\x22引用\x22',
			picture_page_info_list: [
				{ cdn_url: 'https://mmbiz.qpic.cn/one/0?wx_fmt=png', width: '941' * 1 },
				{ cdn_url: 'https://mmbiz.qpic.cn/two/0?wx_fmt=jpeg', width: '941' * 1 }
			],
		</script>
	</body></html>`)

	result, err := Parse("https://mp.weixin.qq.com/s/example", body)
	if err != nil {
		t.Fatalf("parse wechat image article should not fail: %v", err)
	}
	if !result.HasContent {
		t.Fatal("expected image article content to be extracted")
	}
	if !strings.Contains(result.HTML, "<p>第一段</p>") || !strings.Contains(result.HTML, "<p>第二段&#34;引用&#34;</p>") {
		t.Fatalf("expected decoded script text in html, got %q", result.HTML)
	}
	if !strings.Contains(result.HTML, `src="https://mmbiz.qpic.cn/one/0?wx_fmt=png"`) ||
		!strings.Contains(result.HTML, `src="https://mmbiz.qpic.cn/two/0?wx_fmt=jpeg"`) {
		t.Fatalf("expected all picture-page images in html, got %q", result.HTML)
	}
	if len(result.Resources) != 2 {
		t.Fatalf("expected two image resources, got %#v", result.Resources)
	}
}

func TestParseWeChatImageArticleCleansHTMLContentNoEncode(t *testing.T) {
	t.Parallel()

	body := []byte(`<!doctype html><html><head>
		<meta property="og:title" content="脚本 HTML 标题" />
	</head><body>
		<div id="js_content">
			<span class="wx_stream_article_slide_tip_text">向上滑动看下一个</span>
		</div>
		<script>
			item_show_type: '8' * 1,
			content_noencode: '<h3 style="font-family: &quot;Helvetica Neue&quot;;" data-first-child=""><span leaf="">一、</span></h3><p data-pid="p1" style="margin: 1em;"><span leaf="">正文段落</span></p><p style="display: none;"><mp-style-type data-value="3"></mp-style-type></p><img data-src="https://mmbiz.qpic.cn/one/0?wx_fmt=png" style="width: 100px;">',
			picture_page_info_list: [
				{ cdn_url: 'https://mmbiz.qpic.cn/one/0?wx_fmt=png' }
			],
		</script>
	</body></html>`)

	result, err := Parse("https://mp.weixin.qq.com/s/example", body)
	if err != nil {
		t.Fatalf("parse wechat image article should not fail: %v", err)
	}
	if !result.HasContent {
		t.Fatal("expected image article content")
	}
	if strings.Contains(result.HTML, "style=") || strings.Contains(result.HTML, "data-first-child") || strings.Contains(result.HTML, "data-pid") {
		t.Fatalf("expected cleaned html, got %q", result.HTML)
	}
	if !strings.Contains(result.HTML, "<h3><span>一、</span></h3>") || !strings.Contains(result.HTML, "正文段落") {
		t.Fatalf("expected cleaned heading and paragraph, got %q", result.HTML)
	}
	if strings.Count(result.HTML, "https://mmbiz.qpic.cn/one/0?wx_fmt=png") != 1 {
		t.Fatalf("expected image URL only once, got %q", result.HTML)
	}
}

func TestParseWeChatImageArticleDoesNotRequireJSContent(t *testing.T) {
	t.Parallel()

	body := []byte(`<!doctype html><html><head>
		<meta property="og:title" content="图片文章标题" />
	</head><body>
		<div id="js_content_container">
			<span class="wx_stream_article_slide_tip_text">向上滑动看下一个</span>
		</div>
		<script>
			item_show_type: '8' * 1,
			content_noencode: '正文摘要',
			picture_page_info_list: [
				{ cdn_url: 'https://mmbiz.qpic.cn/one/0?wx_fmt=png' }
			],
		</script>
	</body></html>`)

	result, err := Parse("https://mp.weixin.qq.com/s/example", body)
	if err != nil {
		t.Fatalf("parse wechat image article should not fail: %v", err)
	}
	if !result.HasContent {
		t.Fatal("expected image article content")
	}
	if result.Title != "图片文章标题" {
		t.Fatalf("unexpected title: %q", result.Title)
	}
	if !strings.Contains(result.HTML, "正文摘要") || !strings.Contains(result.HTML, `src="https://mmbiz.qpic.cn/one/0?wx_fmt=png"`) {
		t.Fatalf("unexpected html: %q", result.HTML)
	}
}

func TestRewriteResourcesHandlesFragmentInput(t *testing.T) {
	t.Parallel()

	rewritten, err := RewriteResources(
		`<img src="/cover.png" alt="Cover">`,
		map[string]string{"/cover.png": "./assets/cover.png"},
	)
	if err != nil {
		t.Fatalf("rewrite should not fail: %v", err)
	}
	if !strings.Contains(rewritten, `src="./assets/cover.png"`) {
		t.Fatalf("expected rewritten src in output, got %q", rewritten)
	}
}

func mustParseURL(t *testing.T, rawURL string) *url.URL {
	t.Helper()

	parsed, err := url.Parse(rawURL)
	if err != nil {
		t.Fatalf("parse URL: %v", err)
	}

	return parsed
}
