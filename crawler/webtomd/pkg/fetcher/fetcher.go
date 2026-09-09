package fetcher

import (
	"compress/gzip"
	"context"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"net/http/cookiejar"
	"net/url"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"time"

	"github.com/chromedp/chromedp"
)

type ClientConfig struct {
	Timeout           time.Duration
	Cookie            string
	BrowserProfileDir string
	Proxy             string
}

type Client struct {
	httpClient        *http.Client
	cookie            string
	browserProfileDir string
	proxy             string
}

type Page struct {
	URL  string
	Body []byte
}

const defaultTimeout = 90 * time.Second

const browserLikeUserAgent = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/135.0.0.0 Safari/537.36"
const mobileBrowserLikeUserAgent = "Mozilla/5.0 (Linux; Android 12; Pixel 5) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/135.0.0.0 Mobile Safari/537.36"

func New(config ClientConfig) *Client {
	timeout := config.Timeout
	if timeout <= 0 {
		timeout = defaultTimeout
	}
	jar, _ := cookiejar.New(nil)

	httpClient := &http.Client{Timeout: timeout, Jar: jar}

	proxy := strings.TrimSpace(config.Proxy)
	if proxy == "" {
		proxy = os.Getenv("HTTP_PROXY")
	}
	if proxy == "" {
		proxy = os.Getenv("http_proxy")
	}
	if proxy != "" {
		if proxyURL, err := url.Parse(proxy); err == nil {
			httpClient.Transport = &http.Transport{
				Proxy: http.ProxyURL(proxyURL),
			}
		}
	}

	return &Client{
		httpClient:        httpClient,
		cookie:            strings.TrimSpace(config.Cookie),
		browserProfileDir: strings.TrimSpace(config.BrowserProfileDir),
		proxy:             proxy,
	}
}

func (c *Client) Fetch(rawURL string) (Page, error) {
	var lastErr error
	for _, candidateURL := range candidateURLsForURL(rawURL) {
		for _, userAgent := range userAgentsForURL(candidateURL) {
			page, err := c.fetchOnce(candidateURL, userAgent)
			if err == nil {
				return page, nil
			}
			lastErr = err
			time.Sleep(300 * time.Millisecond)
		}
	}
	if page, err := FetchWeiboArticleAPI(rawURL); err == nil {
		return page, nil
	}
	if supportsBrowserFallback(rawURL) {
		page, err := FetchWithBrowser(rawURL, c.browserProfileDir)
		if err == nil {
			return page, nil
		}
	}
	return Page{}, lastErr
}

func (c *Client) fetchOnce(rawURL string, userAgent string) (Page, error) {
	req, err := http.NewRequest(http.MethodGet, rawURL, nil)
	if err != nil {
		return Page{}, fmt.Errorf("无法访问该网页: %w", err)
	}
	req.Header.Set("User-Agent", userAgent)
	req.Header.Set("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8")
	req.Header.Set("Accept-Language", "zh-CN,zh;q=0.9,en;q=0.8")
	req.Header.Set("Accept-Encoding", "gzip")
	req.Header.Set("Cache-Control", "no-cache")
	req.Header.Set("Pragma", "no-cache")
	req.Header.Set("Sec-Fetch-Dest", "document")
	req.Header.Set("Sec-Fetch-Mode", "navigate")
	req.Header.Set("Sec-Fetch-Site", "none")
	req.Header.Set("Sec-Fetch-User", "?1")
	req.Header.Set("Upgrade-Insecure-Requests", "1")
	if c.cookie != "" {
		req.Header.Set("Cookie", c.cookie)
	}
	if referer := buildReferer(rawURL); referer != "" {
		req.Header.Set("Referer", referer)
	}

	resp, err := c.httpClient.Do(req)
	if err != nil {
		return Page{}, fmt.Errorf("无法访问该网页: %w", err)
	}
	defer resp.Body.Close()

	reader := io.Reader(resp.Body)
	if resp.Header.Get("Content-Encoding") == "gzip" {
		gzipReader, gzipErr := gzip.NewReader(resp.Body)
		if gzipErr != nil {
			return Page{}, fmt.Errorf("无法访问该网页: %w", gzipErr)
		}
		defer gzipReader.Close()
		reader = gzipReader
	}

	body, err := io.ReadAll(reader)
	if err != nil {
		return Page{}, fmt.Errorf("无法访问该网页: %w", err)
	}
	if resp.StatusCode < http.StatusOK || resp.StatusCode >= http.StatusMultipleChoices {
		return Page{}, fmt.Errorf("无法访问该网页: HTTP %s", resp.Status)
	}

	return Page{
		URL:  resp.Request.URL.String(),
		Body: body,
	}, nil
}

func userAgentsForURL(rawURL string) []string {
	parsed, err := url.Parse(rawURL)
	if err != nil {
		return []string{browserLikeUserAgent}
	}
	host := parsed.Hostname()
	if host == "weibo.com" || host == "www.weibo.com" || host == "m.weibo.cn" || host == "card.weibo.com" {
		return []string{mobileBrowserLikeUserAgent, browserLikeUserAgent}
	}
	return []string{browserLikeUserAgent}
}

func candidateURLsForURL(rawURL string) []string {
	candidates := []string{rawURL}
	parsed, err := url.Parse(rawURL)
	if err != nil {
		return candidates
	}
	host := parsed.Hostname()
	if host != "weibo.com" && host != "www.weibo.com" && host != "card.weibo.com" {
		return candidates
	}
	id := weiboArticleIDFromPath(parsed.Path)
	if id == "" {
		return candidates
	}
	candidates = append(candidates,
		"https://m.weibo.cn/status/"+id,
		"https://card.weibo.com/article/m/show/id/230940"+id,
	)
	return candidates
}

func FetchWeiboArticleAPI(rawURL string) (Page, error) {
	id := weiboArticleIDFromURL(rawURL)
	if id == "" {
		return Page{}, fmt.Errorf("not a supported weibo article URL")
	}

	jar, _ := cookiejar.New(nil)
	client := &http.Client{Timeout: defaultTimeout, Jar: jar}
	_ = bootstrapWeiboVisitor(client)

	for _, endpoint := range []string{
		"https://m.weibo.cn/statuses/extend?id=" + url.QueryEscape(id),
		"https://weibo.com/ajax/statuses/longtext?id=" + url.QueryEscape(id),
	} {
		body, finalURL, err := fetchWeiboAPIEndpoint(client, endpoint)
		if err != nil {
			continue
		}
		content := strings.TrimSpace(extractWeiboLongText(body))
		if content == "" || detectSinaVisitorBody([]byte(content)) {
			continue
		}
		html := `<html><head><title>微博长文</title></head><body><article class="article"><div class="WB_editor_iframe_new">` + content + `</div></article></body></html>`
		return Page{URL: finalURL, Body: []byte(html)}, nil
	}

	return Page{}, fmt.Errorf("weibo article API did not return extractable content")
}

func bootstrapWeiboVisitor(client *http.Client) error {
	form := url.Values{}
	form.Set("cb", "gen_callback")
	form.Set("fp", `{"os":"1","browser":"Chrome","fonts":"undefined","screenInfo":"1920*1080*24","plugins":""}`)

	req, err := http.NewRequest(http.MethodPost, "https://passport.weibo.com/visitor/genvisitor2", strings.NewReader(form.Encode()))
	if err != nil {
		return err
	}
	req.Header.Set("User-Agent", browserLikeUserAgent)
	req.Header.Set("Accept", "application/javascript,application/json,text/plain,*/*")
	req.Header.Set("Accept-Language", "zh-CN,zh;q=0.9,en;q=0.8")
	req.Header.Set("Content-Type", "application/x-www-form-urlencoded")
	req.Header.Set("Referer", "https://weibo.com/")

	resp, err := client.Do(req)
	if err != nil {
		return err
	}
	defer resp.Body.Close()

	body, err := io.ReadAll(resp.Body)
	if err != nil {
		return err
	}
	tid := strings.TrimSpace(extractStringFieldFromJSONP(body, "tid"))
	if tid == "" {
		return fmt.Errorf("weibo visitor tid not returned")
	}

	visitorURL := "https://passport.weibo.com/visitor/visitor?a=incarnate&t=" + url.QueryEscape(tid) + "&w=2&c=095&gc=&cb=cross_domain&from=weibo&_rand=" + url.QueryEscape(fmt.Sprintf("%f", float64(time.Now().UnixMilli())/1000))
	req, err = http.NewRequest(http.MethodGet, visitorURL, nil)
	if err != nil {
		return err
	}
	req.Header.Set("User-Agent", browserLikeUserAgent)
	req.Header.Set("Accept", "application/javascript,application/json,text/plain,*/*")
	req.Header.Set("Accept-Language", "zh-CN,zh;q=0.9,en;q=0.8")
	req.Header.Set("Referer", "https://weibo.com/")

	resp, err = client.Do(req)
	if err != nil {
		return err
	}
	defer resp.Body.Close()
	_, _ = io.Copy(io.Discard, resp.Body)
	return nil
}

func fetchWeiboAPIEndpoint(client *http.Client, endpoint string) ([]byte, string, error) {
	req, err := http.NewRequest(http.MethodGet, endpoint, nil)
	if err != nil {
		return nil, "", err
	}
	req.Header.Set("User-Agent", mobileBrowserLikeUserAgent)
	req.Header.Set("Accept", "application/json,text/plain,*/*")
	req.Header.Set("Accept-Language", "zh-CN,zh;q=0.9,en;q=0.8")
	req.Header.Set("Referer", "https://m.weibo.cn/")

	resp, err := client.Do(req)
	if err != nil {
		return nil, "", err
	}
	defer resp.Body.Close()

	body, err := io.ReadAll(resp.Body)
	if err != nil {
		return nil, "", err
	}
	if resp.StatusCode < http.StatusOK || resp.StatusCode >= http.StatusMultipleChoices {
		return nil, "", fmt.Errorf("weibo API HTTP %s", resp.Status)
	}
	return body, resp.Request.URL.String(), nil
}

func weiboArticleIDFromURL(rawURL string) string {
	parsed, err := url.Parse(rawURL)
	if err != nil {
		return ""
	}
	host := strings.ToLower(parsed.Hostname())
	if host != "weibo.com" && host != "www.weibo.com" && host != "card.weibo.com" && host != "m.weibo.cn" {
		return ""
	}
	if id := strings.TrimSpace(parsed.Query().Get("id")); id != "" {
		return strings.TrimPrefix(id, "230940")
	}

	parts := strings.Split(strings.Trim(parsed.Path, "/"), "/")
	if len(parts) > 0 {
		last := strings.TrimPrefix(parts[len(parts)-1], "230940")
		if allDigits(last) {
			return last
		}
	}
	return weiboArticleIDFromPath(parsed.Path)
}

func weiboArticleIDFromPath(path string) string {
	const marker = "/id/"
	index := strings.LastIndex(path, marker)
	if index < 0 {
		return ""
	}
	id := strings.TrimSpace(path[index+len(marker):])
	id = strings.TrimPrefix(id, "230940")
	if id == "" {
		return ""
	}
	return id
}

func extractWeiboLongText(body []byte) string {
	var data any
	if err := json.Unmarshal(body, &data); err != nil {
		return ""
	}
	for _, key := range []string{"longTextContent", "content", "text_raw", "text"} {
		if value := strings.TrimSpace(findStringField(data, key)); value != "" {
			return value
		}
	}
	return ""
}

func extractStringFieldFromJSONP(body []byte, field string) string {
	payload := strings.TrimSpace(string(body))
	if start := strings.Index(payload, "("); start >= 0 {
		if end := strings.LastIndex(payload, ")"); end > start {
			payload = payload[start+1 : end]
		}
	}

	var data any
	if err := json.Unmarshal([]byte(payload), &data); err != nil {
		return ""
	}
	return findStringField(data, field)
}

func findStringField(value any, field string) string {
	switch typed := value.(type) {
	case map[string]any:
		if raw, ok := typed[field]; ok {
			if text, ok := raw.(string); ok {
				return text
			}
		}
		for _, child := range typed {
			if text := findStringField(child, field); text != "" {
				return text
			}
		}
	case []any:
		for _, child := range typed {
			if text := findStringField(child, field); text != "" {
				return text
			}
		}
	}
	return ""
}

func allDigits(value string) bool {
	if value == "" {
		return false
	}
	for _, r := range value {
		if r < '0' || r > '9' {
			return false
		}
	}
	return true
}

func detectSinaVisitorBody(body []byte) bool {
	return strings.Contains(strings.ToLower(string(body)), "sina visitor system")
}

func buildReferer(rawURL string) string {
	parsed, err := url.Parse(rawURL)
	if err != nil || parsed.Scheme == "" || parsed.Host == "" {
		return ""
	}
	return parsed.Scheme + "://" + parsed.Host + "/"
}

func supportsBrowserFallback(rawURL string) bool {
	parsed, err := url.Parse(rawURL)
	if err != nil {
		return false
	}
	switch parsed.Hostname() {
	case "weibo.com", "www.weibo.com", "m.weibo.cn", "card.weibo.com", "mp.weixin.qq.com":
		return true
	default:
		return false
	}
}

func FetchWithBrowser(rawURL string, browserProfileDir string) (Page, error) {
	chromePath, err := findChromeExecutable()
	if err != nil {
		return Page{}, err
	}
	userDataDir, err := os.MkdirTemp("", "web2md-browser-*")
	if err != nil {
		return Page{}, fmt.Errorf("create browser profile: %w", err)
	}
	defer os.RemoveAll(userDataDir)
	_ = copyBrowserSession(userDataDir, browserProfileDir)

	ctx, cancel := context.WithTimeout(context.Background(), defaultTimeout)
	defer cancel()

	options := append(chromedp.DefaultExecAllocatorOptions[:],
		chromedp.ExecPath(chromePath),
		chromedp.UserDataDir(userDataDir),
		chromedp.Flag("headless", "new"),
		chromedp.Flag("disable-gpu", true),
		chromedp.Flag("disable-dev-shm-usage", true),
		chromedp.Flag("no-first-run", true),
		chromedp.Flag("no-default-browser-check", true),
		chromedp.Flag("profile-directory", "Default"),
		chromedp.Flag("window-size", "1280,2000"),
		chromedp.UserAgent(browserLikeUserAgent),
	)
	allocatorCtx, cancelAllocator := chromedp.NewExecAllocator(ctx, options...)
	defer cancelAllocator()

	browserCtx, cancelBrowser := chromedp.NewContext(allocatorCtx)
	defer cancelBrowser()

	var html string
	var finalURL string
	if err := chromedp.Run(browserCtx,
		chromedp.Navigate(rawURL),
		chromedp.WaitReady("body", chromedp.ByQuery),
		chromedp.Sleep(12*time.Second),
		chromedp.Location(&finalURL),
		chromedp.OuterHTML("html", &html),
	); err != nil {
		return Page{}, fmt.Errorf("browser fallback failed: %w", err)
	}
	if strings.TrimSpace(html) == "" {
		return Page{}, fmt.Errorf("browser fallback returned empty DOM")
	}
	if strings.TrimSpace(finalURL) == "" {
		finalURL = rawURL
	}
	return Page{URL: finalURL, Body: []byte(html)}, nil
}

func findChromeExecutable() (string, error) {
	for _, candidate := range []string{
		os.Getenv("WEB2MD_CHROME"),
		`C:\Program Files\Google\Chrome\Application\chrome.exe`,
		`C:\Program Files (x86)\Google\Chrome\Application\chrome.exe`,
		`C:\Program Files\Microsoft\Edge\Application\msedge.exe`,
		`C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe`,
		"chrome.exe",
		"msedge.exe",
	} {
		candidate = strings.TrimSpace(candidate)
		if candidate == "" {
			continue
		}
		if filepath.IsAbs(candidate) {
			if _, err := os.Stat(candidate); err == nil {
				return candidate, nil
			}
			continue
		}
		if resolved, err := exec.LookPath(candidate); err == nil {
			return resolved, nil
		}
	}
	return "", fmt.Errorf("browser fallback requires Chrome or Edge; set WEB2MD_CHROME to the browser executable path")
}

func copyBrowserSession(targetUserDataDir string, explicitProfileDir string) error {
	sourceUserDataDir, sourceProfileDir, err := findBrowserProfile(explicitProfileDir)
	if err != nil {
		return err
	}

	if err := copyFileIfExists(filepath.Join(sourceUserDataDir, "Local State"), filepath.Join(targetUserDataDir, "Local State")); err != nil {
		return err
	}
	targetProfileDir := filepath.Join(targetUserDataDir, "Default")
	if err := os.MkdirAll(filepath.Join(targetProfileDir, "Network"), 0o755); err != nil {
		return err
	}
	for _, pair := range []struct {
		source string
		target string
	}{
		{filepath.Join(sourceProfileDir, "Preferences"), filepath.Join(targetProfileDir, "Preferences")},
		{filepath.Join(sourceProfileDir, "Cookies"), filepath.Join(targetProfileDir, "Cookies")},
		{filepath.Join(sourceProfileDir, "Network", "Cookies"), filepath.Join(targetProfileDir, "Network", "Cookies")},
	} {
		if err := copyFileIfExists(pair.source, pair.target); err != nil {
			return err
		}
	}
	return nil
}

func findBrowserProfile(explicitProfileDir string) (string, string, error) {
	if explicit := strings.TrimSpace(firstNonEmpty(explicitProfileDir, os.Getenv("WEB2MD_BROWSER_PROFILE_DIR"))); explicit != "" {
		if hasBrowserCookies(explicit) {
			return filepath.Dir(explicit), explicit, nil
		}
		return "", "", fmt.Errorf("WEB2MD_BROWSER_PROFILE_DIR does not contain browser cookies")
	}

	for _, root := range browserUserDataDirCandidates() {
		if root == "" {
			continue
		}
		for _, profileName := range []string{"Default", "Profile 1", "Profile 2", "Profile 3", "Profile 4", "Profile 5"} {
			profileDir := filepath.Join(root, profileName)
			if hasBrowserCookies(profileDir) {
				return root, profileDir, nil
			}
		}
	}
	return "", "", fmt.Errorf("browser profile with cookies not found")
}

func browserUserDataDirCandidates() []string {
	localAppData := strings.TrimSpace(os.Getenv("LOCALAPPDATA"))
	return []string{
		strings.TrimSpace(os.Getenv("WEB2MD_BROWSER_USER_DATA_DIR")),
		filepath.Join(localAppData, "Google", "Chrome", "User Data"),
		filepath.Join(localAppData, "Microsoft", "Edge", "User Data"),
	}
}

func hasBrowserCookies(profileDir string) bool {
	for _, candidate := range []string{
		filepath.Join(profileDir, "Network", "Cookies"),
		filepath.Join(profileDir, "Cookies"),
	} {
		if info, err := os.Stat(candidate); err == nil && !info.IsDir() {
			return true
		}
	}
	return false
}

func firstNonEmpty(values ...string) string {
	for _, value := range values {
		if strings.TrimSpace(value) != "" {
			return value
		}
	}
	return ""
}

func copyFileIfExists(source string, target string) error {
	info, err := os.Stat(source)
	if err != nil {
		if os.IsNotExist(err) {
			return nil
		}
		return err
	}
	if info.IsDir() {
		return nil
	}
	input, err := os.Open(source)
	if err != nil {
		return nil
	}
	defer input.Close()

	if err := os.MkdirAll(filepath.Dir(target), 0o755); err != nil {
		return err
	}
	output, err := os.Create(target)
	if err != nil {
		return err
	}
	defer output.Close()

	_, err = io.Copy(output, input)
	return err
}
