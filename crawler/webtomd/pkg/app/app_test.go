package app

import (
	"io"
	"net/http"
	"net/http/httptest"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func TestRunWritesMarkdownAndAssets(t *testing.T) {
	t.Parallel()

	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		switch r.URL.Path {
		case "/article":
			w.Header().Set("Content-Type", "text/html; charset=utf-8")
			_, _ = io.WriteString(w, `<!doctype html>
<html>
<head><title>Readable Story</title></head>
<body>
	<main>
		<article>
			<h1>Readable Story</h1>
			<p>The durable web page keeps the essential paragraph when menus, sidebars, and other chrome are removed. This sentence gives the extractor enough readable text to treat the section as the primary article content.</p>
			<p>A second paragraph adds useful density for readability scoring and makes the article look like a real page instead of a short teaser.</p>
			<img src="/cover.png" alt="Cover">
		</article>
	</main>
</body>
</html>`)
		case "/cover.png":
			w.Header().Set("Content-Type", "image/png")
			_, _ = io.WriteString(w, "cover image")
		default:
			http.NotFound(w, r)
		}
	}))
	defer server.Close()

	workDir := t.TempDir()
	if err := Run(Config{
		URL:     server.URL + "/article",
		Name:    "note",
		WorkDir: workDir,
	}); err != nil {
		t.Fatalf("run should succeed: %v", err)
	}

	notePath := filepath.Join(workDir, "note.md")
	markdownBytes, err := os.ReadFile(notePath)
	if err != nil {
		t.Fatalf("read note: %v", err)
	}
	markdown := string(markdownBytes)
	if !strings.Contains(markdown, `> 原文链接：`+server.URL+`/article`) {
		t.Fatalf("expected metadata quote URL in markdown: %q", markdown)
	}
	if !strings.Contains(markdown, "# Readable Story") {
		t.Fatalf("expected title heading in markdown: %q", markdown)
	}
	if !strings.Contains(markdown, "./assets/cover.png") {
		t.Fatalf("expected rewritten local asset URL in markdown: %q", markdown)
	}
	if got, err := os.ReadFile(filepath.Join(workDir, "assets", "cover.png")); err != nil || string(got) != "cover image" {
		t.Fatalf("asset file mismatch: got %q err %v", string(got), err)
	}
}

func TestRunTolerantKeepsRemoteURLForFailedAsset(t *testing.T) {
	t.Parallel()

	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		switch r.URL.Path {
		case "/article":
			w.Header().Set("Content-Type", "text/html; charset=utf-8")
			_, _ = io.WriteString(w, `<!doctype html>
<html>
<head><title>Readable Story</title></head>
<body>
	<main>
		<article>
			<h1>Readable Story</h1>
			<p>The durable web page keeps the essential paragraph when menus, sidebars, and other chrome are removed. This sentence gives the extractor enough readable text to treat the section as the primary article content.</p>
			<p>A second paragraph adds useful density for readability scoring and makes the article look like a real page instead of a short teaser.</p>
			<img src="/missing.png" alt="Missing">
		</article>
	</main>
</body>
</html>`)
		case "/missing.png":
			http.NotFound(w, r)
		default:
			http.NotFound(w, r)
		}
	}))
	defer server.Close()

	workDir := t.TempDir()
	if err := Run(Config{
		URL:     server.URL + "/article",
		Name:    "note",
		WorkDir: workDir,
		Strict:  false,
	}); err != nil {
		t.Fatalf("tolerant run should not fail: %v", err)
	}

	markdownBytes, err := os.ReadFile(filepath.Join(workDir, "note.md"))
	if err != nil {
		t.Fatalf("read note: %v", err)
	}
	markdown := string(markdownBytes)
	missingURL := server.URL + "/missing.png"
	if !strings.Contains(markdown, missingURL) {
		t.Fatalf("expected remote missing URL to be preserved in markdown: %q", markdown)
	}
}

func TestRunRejectsEmptyName(t *testing.T) {
	t.Parallel()

	err := Run(Config{
		URL:     "https://example.com/article",
		Name:    "   ",
		WorkDir: t.TempDir(),
	})
	if err == nil {
		t.Fatal("expected empty name to fail")
	}
}

func TestRunRejectsEmptyURL(t *testing.T) {
	t.Parallel()

	err := Run(Config{
		URL:     " ",
		Name:    "note",
		WorkDir: t.TempDir(),
	})
	if err == nil {
		t.Fatal("expected empty URL to fail")
	}
}

func TestRunStrictReturnsErrorAndNoMarkdown(t *testing.T) {
	t.Parallel()

	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		switch r.URL.Path {
		case "/article":
			w.Header().Set("Content-Type", "text/html; charset=utf-8")
			_, _ = io.WriteString(w, `<!doctype html><html><body><article>
<h1>Readable Story</h1>
<p>The durable web page keeps the essential paragraph when menus, sidebars, and other chrome are removed.</p>
<p>A second paragraph adds useful density for readability scoring.</p>
<img src="/missing.png" alt="Missing">
</article></body></html>`)
		case "/missing.png":
			http.NotFound(w, r)
		default:
			http.NotFound(w, r)
		}
	}))
	defer server.Close()

	workDir := t.TempDir()
	err := Run(Config{
		URL:     server.URL + "/article",
		Name:    "strict-note",
		WorkDir: workDir,
		Strict:  true,
	})
	if err == nil {
		t.Fatal("expected strict mode to return an error")
	}
	if _, statErr := os.Stat(filepath.Join(workDir, "strict-note.md")); !os.IsNotExist(statErr) {
		t.Fatalf("expected markdown not to be written on strict failure, stat err: %v", statErr)
	}
}

func TestRunDeduplicatesSameResolvedAsset(t *testing.T) {
	t.Parallel()

	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		switch r.URL.Path {
		case "/article":
			w.Header().Set("Content-Type", "text/html; charset=utf-8")
			_, _ = io.WriteString(w, `<!doctype html><html><body><article>
<h1>Readable Story</h1>
<p>The durable web page keeps the essential paragraph when menus, sidebars, and other chrome are removed.</p>
<p>A second paragraph adds useful density for readability scoring.</p>
<img src="/cover.png" alt="Cover A">
<img src="/cover.png" alt="Cover B">
</article></body></html>`)
		case "/cover.png":
			_, _ = io.WriteString(w, "cover image")
		default:
			http.NotFound(w, r)
		}
	}))
	defer server.Close()

	workDir := t.TempDir()
	if err := Run(Config{
		URL:     server.URL + "/article",
		Name:    "dup-note",
		WorkDir: workDir,
	}); err != nil {
		t.Fatalf("run should succeed: %v", err)
	}

	assetsDir := filepath.Join(workDir, "assets")
	entries, err := os.ReadDir(assetsDir)
	if err != nil {
		t.Fatalf("read assets dir: %v", err)
	}
	if len(entries) != 1 || entries[0].Name() != "cover.png" {
		t.Fatalf("expected exactly one deduplicated asset file, got %#v", entries)
	}
}

func TestRunRejectsCaptchaVerificationPage(t *testing.T) {
	t.Parallel()

	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "text/html; charset=utf-8")
		_, _ = io.WriteString(w, `<!doctype html><html><body>
<h1>环境异常</h1>
<p>当前环境异常，完成验证后即可继续访问。</p>
<a href="/verify">去验证</a>
</body></html>`)
	}))
	defer server.Close()

	workDir := t.TempDir()
	err := Run(Config{
		URL:     server.URL + "/mp/wappoc_appmsgcaptcha?poc_token=abc",
		Name:    "captcha-note",
		WorkDir: workDir,
	})
	if err == nil {
		t.Fatal("expected captcha page to return an error")
	}
	if !strings.Contains(err.Error(), "触发验证码") {
		t.Fatalf("expected captcha error message, got %v", err)
	}
	if _, statErr := os.Stat(filepath.Join(workDir, "captcha-note.md")); !os.IsNotExist(statErr) {
		t.Fatalf("expected markdown not to be written for captcha page, stat err: %v", statErr)
	}
}

func TestRunRejectsPermissionLimitedPage(t *testing.T) {
	t.Parallel()

	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "text/html; charset=utf-8")
		_, _ = io.WriteString(w, `<!doctype html><html><head><title>Sina Visitor System</title></head><body>
<p>你暂无权限查看此页面内容</p>
</body></html>`)
	}))
	defer server.Close()

	workDir := t.TempDir()
	err := Run(Config{
		URL:     server.URL + "/limited",
		Name:    "limited-note",
		WorkDir: workDir,
	})
	if err == nil {
		t.Fatal("expected permission-limited page to return an error")
	}
	if !strings.Contains(err.Error(), "权限") && !strings.Contains(err.Error(), "风控") {
		t.Fatalf("expected permission or anti-crawler error, got %v", err)
	}
	if _, statErr := os.Stat(filepath.Join(workDir, "limited-note.md")); !os.IsNotExist(statErr) {
		t.Fatalf("expected markdown not to be written for limited page, stat err: %v", statErr)
	}
}

func TestRunBypassesWechatStyleCaptchaWhenBrowserUA(t *testing.T) {
	t.Parallel()

	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		switch r.URL.Path {
		case "/article":
			if strings.Contains(strings.ToLower(r.UserAgent()), "web2md") {
				http.Redirect(w, r, "/mp/wappoc_appmsgcaptcha?poc_token=blocked", http.StatusFound)
				return
			}
			w.Header().Set("Content-Type", "text/html; charset=utf-8")
			_, _ = io.WriteString(w, `<!doctype html><html><body><article>
<h1>Wechat Readable Story</h1>
<p>This body should be extracted as readable content from a direct article response.</p>
</article></body></html>`)
		case "/mp/wappoc_appmsgcaptcha":
			w.Header().Set("Content-Type", "text/html; charset=utf-8")
			_, _ = io.WriteString(w, `<!doctype html><html><body><h1>环境异常</h1><a>去验证</a></body></html>`)
		default:
			http.NotFound(w, r)
		}
	}))
	defer server.Close()

	workDir := t.TempDir()
	err := Run(Config{
		URL:     server.URL + "/article",
		Name:    "wechat-note",
		WorkDir: workDir,
	})
	if err != nil {
		t.Fatalf("expected browser-like UA to avoid captcha path, got error: %v", err)
	}

	markdown, readErr := os.ReadFile(filepath.Join(workDir, "wechat-note.md"))
	if readErr != nil {
		t.Fatalf("read markdown: %v", readErr)
	}
	if !strings.Contains(string(markdown), "Wechat Readable Story") {
		t.Fatalf("expected article content in markdown, got: %q", string(markdown))
	}
}

func TestRunUsesSiteConfig(t *testing.T) {
	t.Parallel()

	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		switch r.URL.Path {
		case "/article":
			w.Header().Set("Content-Type", "text/html; charset=utf-8")
			_, _ = io.WriteString(w, `<!doctype html><html><body>
<h1 class="headline">Configured Title</h1>
<section class="configured-body">
	<p>Configured body was selected.</p>
	<p class="ad">remove me</p>
</section>
</body></html>`)
		default:
			http.NotFound(w, r)
		}
	}))
	defer server.Close()

	workDir := t.TempDir()
	configPath := filepath.Join(workDir, "sites.json")
	configJSON := strings.ReplaceAll(`{
		"version": 1,
		"sites": [{
			"name": "local",
			"hosts": ["HOST"],
			"title": [".headline"],
			"content": [".configured-body"],
			"remove": [".ad"]
		}]
	}`, "HOST", strings.TrimPrefix(server.URL, "http://"))
	if err := os.WriteFile(configPath, []byte(configJSON), 0o644); err != nil {
		t.Fatalf("write site config: %v", err)
	}

	err := Run(Config{
		URL:            server.URL + "/article",
		Name:           "configured-note",
		WorkDir:        workDir,
		SiteConfigPath: configPath,
	})
	if err != nil {
		t.Fatalf("run should succeed: %v", err)
	}

	markdown, err := os.ReadFile(filepath.Join(workDir, "configured-note.md"))
	if err != nil {
		t.Fatalf("read markdown: %v", err)
	}
	text := string(markdown)
	if !strings.Contains(text, "# Configured Title") {
		t.Fatalf("expected configured title in markdown: %q", text)
	}
	if !strings.Contains(text, "Configured body was selected.") {
		t.Fatalf("expected configured body in markdown: %q", text)
	}
	if strings.Contains(text, "remove me") {
		t.Fatalf("expected remove selector to drop ad: %q", text)
	}
}
