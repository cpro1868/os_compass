package downloader

import (
	"errors"
	"fmt"
	"io"
	"mime"
	"net/http"
	"net/url"
	"os"
	"path"
	"path/filepath"
	"strings"
	"sync"
	"time"
)

type Resource struct {
	URL string
}

type Result struct {
	URL       string
	LocalPath string
	// Replacement is the Markdown/html path for assets stored in the page's assets directory.
	// It is always formatted as ./assets/<filename>, independent of the absolute assetDir.
	Replacement string
	Failed      bool
	Err         error
}

type Config struct {
	Concurrency int
	Strict      bool
	Events      EventSink
}

type EventSink interface {
	EventName(name string, status string)
}

type Downloader struct {
	concurrency int
	strict      bool
	events      EventSink
	eventMu     sync.Mutex
	httpClient  *http.Client
}

const (
	defaultConcurrency = 5
	defaultTimeout     = 90 * time.Second
	maxAttempts        = 3
	browserUserAgent   = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/135.0.0.0 Safari/537.36"
)

func New(config Config) *Downloader {
	concurrency := config.Concurrency
	if concurrency <= 0 {
		concurrency = defaultConcurrency
	}

	return &Downloader{
		concurrency: concurrency,
		strict:      config.Strict,
		events:      config.Events,
		httpClient:  &http.Client{Timeout: defaultTimeout},
	}
}

func (d *Downloader) DownloadAll(assetDir string, resources []Resource) ([]Result, error) {
	if err := os.MkdirAll(assetDir, 0o755); err != nil {
		return nil, fmt.Errorf("create asset dir: %w", err)
	}

	results := make([]Result, len(resources))
	jobs := make(chan downloadJob)
	session := &downloadSession{
		assetDir: assetDir,
		reserved: make(map[string]struct{}),
	}
	var wg sync.WaitGroup

	workerCount := d.concurrency
	if workerCount > len(resources) {
		workerCount = len(resources)
	}

	for i := 0; i < workerCount; i++ {
		wg.Add(1)
		go func() {
			defer wg.Done()
			for job := range jobs {
				results[job.index] = d.download(job)
			}
		}()
	}

	for i, resource := range resources {
		baseFilename := filenameFromURL(resource.URL)
		filename := session.reserveNext(baseFilename)
		jobs <- downloadJob{
			index:        i,
			resource:     resource,
			baseFilename: baseFilename,
			filename:     filename,
			session:      session,
		}
	}
	close(jobs)
	wg.Wait()

	if d.strict {
		for _, result := range results {
			if result.Failed {
				return results, fmt.Errorf("download failed for %s: %w", result.URL, result.Err)
			}
		}
	}

	return results, nil
}

type downloadJob struct {
	index        int
	resource     Resource
	baseFilename string
	filename     string
	session      *downloadSession
}

func (d *Downloader) download(job downloadJob) Result {
	result := Result{
		URL: job.resource.URL,
	}
	displayName := job.filename

	d.emit(displayName, "Downloading")

	var lastErr error
	for attempt := 1; attempt <= maxAttempts; attempt++ {
		attemptResult, attemptDisplayName, err := d.downloadAttempt(job, result)
		displayName = attemptDisplayName
		if err == nil {
			d.emit(displayName, "Complete")
			return attemptResult
		}
		lastErr = err
		if !isRetryableDownloadError(err) || attempt == maxAttempts {
			break
		}
		time.Sleep(time.Duration(attempt) * 300 * time.Millisecond)
	}

	return d.failedResult(result, displayName, lastErr)
}

func (d *Downloader) downloadAttempt(job downloadJob, result Result) (Result, string, error) {
	request, err := http.NewRequest(http.MethodGet, job.resource.URL, nil)
	if err != nil {
		return result, job.filename, err
	}
	request.Header.Set("User-Agent", browserUserAgent)
	request.Header.Set("Accept", "image/avif,image/webp,image/apng,image/svg+xml,image/*,video/*,*/*;q=0.8")
	request.Header.Set("Accept-Language", "zh-CN,zh;q=0.9,en;q=0.8")

	resp, err := d.httpClient.Do(request)
	if err != nil {
		return result, job.filename, retryableDownloadError{err: err}
	}
	defer resp.Body.Close()

	if resp.StatusCode < http.StatusOK || resp.StatusCode >= http.StatusMultipleChoices {
		err := fmt.Errorf("HTTP %s", resp.Status)
		if resp.StatusCode == http.StatusTooManyRequests || resp.StatusCode >= http.StatusInternalServerError {
			return result, job.filename, retryableDownloadError{err: err}
		}
		return result, job.filename, err
	}

	filename := filenameWithInferredExtension(job.filename, job.resource.URL, resp.Header.Get("Content-Type"))
	if filename != job.filename {
		filename = job.session.replaceReservation(job.filename, filename)
	}

	tempFile, err := os.CreateTemp(job.session.assetDir, ".web2md-download-*")
	if err != nil {
		return result, filename, err
	}
	tempPath := tempFile.Name()

	if _, err := io.Copy(tempFile, resp.Body); err != nil {
		_ = tempFile.Close()
		_ = os.Remove(tempPath)
		return result, filename, retryableDownloadError{err: err}
	}
	if err := tempFile.Close(); err != nil {
		_ = os.Remove(tempPath)
		return result, filename, retryableDownloadError{err: err}
	}

	result, err = job.session.commitTempFile(tempPath, filename, filename, result)
	if err != nil {
		_ = os.Remove(tempPath)
		return result, filename, err
	}

	return result, filename, nil
}

func (d *Downloader) failedResult(result Result, displayName string, err error) Result {
	result.Failed = true
	result.Err = err
	result.Replacement = result.URL
	d.emit(displayName, "Failed")
	return result
}

func (d *Downloader) emit(name string, status string) {
	if d.events != nil {
		d.eventMu.Lock()
		defer d.eventMu.Unlock()
		d.events.EventName(name, status)
	}
}

type retryableDownloadError struct {
	err error
}

func (e retryableDownloadError) Error() string {
	return e.err.Error()
}

func (e retryableDownloadError) Unwrap() error {
	return e.err
}

func isRetryableDownloadError(err error) bool {
	var retryable retryableDownloadError
	return errors.As(err, &retryable)
}

type downloadSession struct {
	assetDir string
	reserved map[string]struct{}
	mu       sync.Mutex
}

func (s *downloadSession) reserveNext(filename string) string {
	s.mu.Lock()
	defer s.mu.Unlock()

	return s.reserveNextLocked(filename)
}

func (s *downloadSession) replaceReservation(oldFilename string, newFilename string) string {
	s.mu.Lock()
	defer s.mu.Unlock()

	delete(s.reserved, strings.ToLower(oldFilename))
	return s.reserveNextLocked(newFilename)
}

func (s *downloadSession) reserveNextLocked(filename string) string {
	ext := filepath.Ext(filename)
	base := strings.TrimSuffix(filename, ext)
	if base == "" {
		base = "asset"
	}

	candidate := filename
	for i := 1; fileExists(filepath.Join(s.assetDir, candidate)) || isReserved(candidate, s.reserved); i++ {
		candidate = fmt.Sprintf("%s_%d%s", base, i, ext)
	}
	s.reserved[strings.ToLower(candidate)] = struct{}{}

	return candidate
}

func (s *downloadSession) commitTempFile(tempPath string, baseFilename string, filename string, result Result) (Result, error) {
	for {
		finalPath := filepath.Join(s.assetDir, filename)
		result.LocalPath = finalPath
		result.Replacement = "./assets/" + filepath.ToSlash(filename)

		err := copyTempToExclusiveFinal(tempPath, finalPath)
		if os.IsExist(err) {
			filename = s.reserveNext(baseFilename)
			continue
		}
		if err != nil {
			return result, err
		}

		_ = os.Remove(tempPath)
		return result, nil
	}
}

func copyTempToExclusiveFinal(tempPath string, finalPath string) error {
	source, err := os.Open(tempPath)
	if err != nil {
		return err
	}
	defer source.Close()

	target, err := os.OpenFile(finalPath, os.O_CREATE|os.O_EXCL|os.O_WRONLY, 0o644)
	if err != nil {
		return err
	}

	removeTarget := true
	defer func() {
		if removeTarget {
			_ = os.Remove(finalPath)
		}
	}()

	if _, err := io.Copy(target, source); err != nil {
		_ = target.Close()
		return err
	}
	if err := target.Close(); err != nil {
		return err
	}

	removeTarget = false
	return nil
}

func filenameFromURL(rawURL string) string {
	parsed, err := url.Parse(strings.TrimSpace(rawURL))
	if err != nil {
		return "asset"
	}

	filename := path.Base(parsed.Path)
	if filename == "." || filename == "/" || filename == "" {
		return "asset"
	}

	if unescaped, err := url.PathUnescape(filename); err == nil {
		filename = unescaped
	}

	filename = sanitizeFilename(filename)
	if filename == "" {
		return "asset"
	}

	return filename
}

func filenameWithInferredExtension(filename string, rawURL string, contentType string) string {
	if filepath.Ext(filename) != "" {
		return filename
	}

	ext := extensionFromURLQuery(rawURL)
	if ext == "" {
		ext = extensionFromContentType(contentType)
	}
	if ext == "" {
		return filename
	}

	return filename + ext
}

func extensionFromURLQuery(rawURL string) string {
	parsed, err := url.Parse(strings.TrimSpace(rawURL))
	if err != nil {
		return ""
	}

	for _, key := range []string{"wx_fmt", "format", "fmt"} {
		if ext := normalizeExtension(parsed.Query().Get(key)); ext != "" {
			return ext
		}
	}
	return ""
}

func extensionFromContentType(contentType string) string {
	mediaType, _, err := mime.ParseMediaType(contentType)
	if err != nil {
		return ""
	}
	switch mediaType {
	case "image/jpeg":
		return ".jpg"
	case "image/png":
		return ".png"
	case "image/gif":
		return ".gif"
	case "image/webp":
		return ".webp"
	case "image/svg+xml":
		return ".svg"
	case "video/mp4":
		return ".mp4"
	case "video/webm":
		return ".webm"
	case "video/quicktime":
		return ".mov"
	default:
		return ""
	}
}

func normalizeExtension(value string) string {
	switch strings.ToLower(strings.TrimSpace(value)) {
	case "jpg", "jpeg":
		return ".jpg"
	case "png":
		return ".png"
	case "gif":
		return ".gif"
	case "webp":
		return ".webp"
	case "svg":
		return ".svg"
	case "mp4":
		return ".mp4"
	case "webm":
		return ".webm"
	case "mov":
		return ".mov"
	default:
		return ""
	}
}

func isReserved(filename string, reserved map[string]struct{}) bool {
	_, ok := reserved[strings.ToLower(filename)]
	return ok
}

func fileExists(filename string) bool {
	_, err := os.Stat(filename)
	return err == nil
}

func sanitizeFilename(filename string) string {
	replacer := strings.NewReplacer(
		"\\", "_",
		"/", "_",
		":", "_",
		"*", "_",
		"?", "_",
		"\"", "_",
		"<", "_",
		">", "_",
		"|", "_",
	)

	return strings.TrimSpace(replacer.Replace(filename))
}
