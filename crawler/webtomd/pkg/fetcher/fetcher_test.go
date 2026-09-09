package fetcher

import (
	"io"
	"net/http"
	"net/http/httptest"
	"os"
	"path/filepath"
	"strings"
	"testing"
	"time"
)

func TestFetchSendsUserAgentAndReturnsBody(t *testing.T) {
	t.Parallel()

	userAgentCh := make(chan string, 1)
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.URL.Path == "/redirect" {
			userAgentCh <- r.Header.Get("User-Agent")
			http.Redirect(w, r, "/final", http.StatusFound)
			return
		}

		if r.URL.Path != "/final" {
			http.NotFound(w, r)
			return
		}

		w.WriteHeader(http.StatusOK)
		_, _ = io.WriteString(w, "hello from body")
	}))
	defer server.Close()

	client := New(ClientConfig{Timeout: time.Second})
	page, err := client.Fetch(server.URL + "/redirect")
	if err != nil {
		t.Fatalf("fetch should succeed: %v", err)
	}

	if got := string(page.Body); got != "hello from body" {
		t.Fatalf("unexpected body: %q", got)
	}
	if page.URL != server.URL+"/final" {
		t.Fatalf("unexpected final URL: got %q want %q", page.URL, server.URL+"/final")
	}

	userAgent := <-userAgentCh
	if !strings.Contains(userAgent, "Mozilla/5.0") {
		t.Fatalf("expected browser-like user agent, got %q", userAgent)
	}
	if strings.Contains(strings.ToLower(userAgent), "web2md") {
		t.Fatalf("expected user agent to avoid bot marker, got %q", userAgent)
	}
}

func TestFetchRejectsNotFound(t *testing.T) {
	t.Parallel()

	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		http.NotFound(w, r)
	}))
	defer server.Close()

	client := New(ClientConfig{Timeout: time.Second})
	_, err := client.Fetch(server.URL)
	if err == nil {
		t.Fatal("expected fetch to fail for 404 response")
	}
	if !strings.Contains(err.Error(), "无法访问该网页") {
		t.Fatalf("expected friendly error message, got %q", err.Error())
	}
}

func TestFetchSendsConfiguredCookie(t *testing.T) {
	t.Parallel()

	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.Header.Get("Cookie") != "SUB=abc; SUBP=def" {
			http.Error(w, "missing cookie", http.StatusForbidden)
			return
		}
		_, _ = io.WriteString(w, "cookie ok")
	}))
	defer server.Close()

	client := New(ClientConfig{Timeout: time.Second, Cookie: "SUB=abc; SUBP=def"})
	page, err := client.Fetch(server.URL)
	if err != nil {
		t.Fatalf("fetch should succeed with configured cookie: %v", err)
	}
	if string(page.Body) != "cookie ok" {
		t.Fatalf("unexpected body: %q", string(page.Body))
	}
}

func TestCandidateURLsForWeiboArticleIncludesMobileFallback(t *testing.T) {
	t.Parallel()

	candidates := candidateURLsForURL("https://weibo.com/ttarticle/x/m/show/id/2309405303156245659656")
	want := "https://m.weibo.cn/status/5303156245659656"
	for _, candidate := range candidates {
		if candidate == want {
			return
		}
	}
	t.Fatalf("expected mobile fallback %q in candidates %#v", want, candidates)
}

func TestWeiboArticleIDFromURL(t *testing.T) {
	t.Parallel()

	tests := map[string]string{
		"https://weibo.com/ttarticle/x/m/show/id/2309405303156245659656":  "5303156245659656",
		"https://card.weibo.com/article/m/show/id/2309405303156245659656": "5303156245659656",
		"https://m.weibo.cn/status/5303156245659656":                      "5303156245659656",
		"https://example.com/status/5303156245659656":                     "",
	}
	for input, want := range tests {
		if got := weiboArticleIDFromURL(input); got != want {
			t.Fatalf("weiboArticleIDFromURL(%q) = %q, want %q", input, got, want)
		}
	}
}

func TestExtractWeiboLongText(t *testing.T) {
	t.Parallel()

	body := []byte(`{"ok":1,"data":{"longTextContent":"<p>正文内容</p>"}}`)
	if got := extractWeiboLongText(body); got != "<p>正文内容</p>" {
		t.Fatalf("unexpected long text: %q", got)
	}
}

func TestExtractStringFieldFromJSONP(t *testing.T) {
	t.Parallel()

	body := []byte(`gen_callback({"retcode":20000000,"data":{"tid":"visitor-tid"}})`)
	if got := extractStringFieldFromJSONP(body, "tid"); got != "visitor-tid" {
		t.Fatalf("unexpected tid: %q", got)
	}
}

func TestCopyBrowserSessionCopiesCookiesToTemporaryDefaultProfile(t *testing.T) {
	sourceRoot := t.TempDir()
	sourceProfile := filepath.Join(sourceRoot, "Default")
	if err := os.MkdirAll(filepath.Join(sourceProfile, "Network"), 0o755); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(filepath.Join(sourceRoot, "Local State"), []byte("local-state"), 0o644); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(filepath.Join(sourceProfile, "Network", "Cookies"), []byte("cookies"), 0o644); err != nil {
		t.Fatal(err)
	}

	targetRoot := t.TempDir()
	t.Setenv("WEB2MD_BROWSER_USER_DATA_DIR", sourceRoot)
	t.Setenv("WEB2MD_BROWSER_PROFILE_DIR", "")
	t.Setenv("LOCALAPPDATA", "")

	if err := copyBrowserSession(targetRoot, ""); err != nil {
		t.Fatalf("copyBrowserSession failed: %v", err)
	}
	if got, err := os.ReadFile(filepath.Join(targetRoot, "Default", "Network", "Cookies")); err != nil || string(got) != "cookies" {
		t.Fatalf("unexpected copied cookies: %q, %v", string(got), err)
	}
}
