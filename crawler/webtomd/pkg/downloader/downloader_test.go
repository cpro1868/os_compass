package downloader

import (
	"io"
	"net/http"
	"net/http/httptest"
	"os"
	"path/filepath"
	"reflect"
	"sync/atomic"
	"testing"
	"time"
)

func TestDownloadAllAvoidsExistingFilenameCollision(t *testing.T) {
	t.Parallel()

	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.URL.Path != "/cover.png" {
			http.NotFound(w, r)
			return
		}

		w.WriteHeader(http.StatusOK)
		_, _ = io.WriteString(w, "new cover")
	}))
	defer server.Close()

	assetDir := filepath.Join(t.TempDir(), "assets")
	if err := os.MkdirAll(assetDir, 0o755); err != nil {
		t.Fatalf("create asset dir: %v", err)
	}
	if err := os.WriteFile(filepath.Join(assetDir, "cover.png"), []byte("existing cover"), 0o644); err != nil {
		t.Fatalf("write existing asset: %v", err)
	}

	client := New(Config{Concurrency: 1})
	results, err := client.DownloadAll(assetDir, []Resource{{URL: server.URL + "/cover.png"}})
	if err != nil {
		t.Fatalf("download should succeed: %v", err)
	}
	if len(results) != 1 {
		t.Fatalf("expected one result, got %d", len(results))
	}

	result := results[0]
	if result.Failed {
		t.Fatalf("download failed unexpectedly: %v", result.Err)
	}
	if result.Replacement != "./assets/cover_1.png" {
		t.Fatalf("unexpected replacement: got %q want %q", result.Replacement, "./assets/cover_1.png")
	}
	if result.LocalPath != filepath.Join(assetDir, "cover_1.png") {
		t.Fatalf("unexpected local path: got %q", result.LocalPath)
	}
	if got, err := os.ReadFile(result.LocalPath); err != nil || string(got) != "new cover" {
		t.Fatalf("downloaded file mismatch: got %q err %v", string(got), err)
	}
}

func TestDownloadAllAddsExtensionFromWechatURLFormat(t *testing.T) {
	t.Parallel()

	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "image/jpeg")
		w.WriteHeader(http.StatusOK)
		_, _ = io.WriteString(w, "jpeg")
	}))
	defer server.Close()

	client := New(Config{Concurrency: 1})
	results, err := client.DownloadAll(filepath.Join(t.TempDir(), "assets"), []Resource{{URL: server.URL + "/mmbiz/640?wx_fmt=jpeg"}})
	if err != nil {
		t.Fatalf("download should succeed: %v", err)
	}
	if len(results) != 1 {
		t.Fatalf("expected one result, got %d", len(results))
	}
	if results[0].Replacement != "./assets/640.jpg" {
		t.Fatalf("unexpected replacement: got %q want %q", results[0].Replacement, "./assets/640.jpg")
	}
	if filepath.Ext(results[0].LocalPath) != ".jpg" {
		t.Fatalf("expected local file to have .jpg extension, got %q", results[0].LocalPath)
	}
}

func TestDownloadAllAddsExtensionFromContentTypeWhenURLHasNoExtension(t *testing.T) {
	t.Parallel()

	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "image/png")
		w.WriteHeader(http.StatusOK)
		_, _ = io.WriteString(w, "png")
	}))
	defer server.Close()

	client := New(Config{Concurrency: 1})
	results, err := client.DownloadAll(filepath.Join(t.TempDir(), "assets"), []Resource{{URL: server.URL + "/asset/640"}})
	if err != nil {
		t.Fatalf("download should succeed: %v", err)
	}
	if results[0].Replacement != "./assets/640.png" {
		t.Fatalf("unexpected replacement: got %q want %q", results[0].Replacement, "./assets/640.png")
	}
}

func TestDownloadAllUsesBrowserLikeUserAgent(t *testing.T) {
	t.Parallel()

	var userAgent string
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		userAgent = r.UserAgent()
		w.Header().Set("Content-Type", "image/jpeg")
		w.WriteHeader(http.StatusOK)
		_, _ = io.WriteString(w, "jpeg")
	}))
	defer server.Close()

	client := New(Config{Concurrency: 1})
	results, err := client.DownloadAll(filepath.Join(t.TempDir(), "assets"), []Resource{{URL: server.URL + "/cover.jpg"}})
	if err != nil {
		t.Fatalf("download should succeed: %v", err)
	}
	if len(results) != 1 || results[0].Failed {
		t.Fatalf("download failed unexpectedly: %#v", results)
	}
	if userAgent == "" || userAgent == "Go-http-client/1.1" {
		t.Fatalf("expected browser-like user agent, got %q", userAgent)
	}
}

func TestDownloadAllRetriesTransientServerFailure(t *testing.T) {
	t.Parallel()

	var attempts atomic.Int32
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if attempts.Add(1) == 1 {
			http.Error(w, "temporary", http.StatusServiceUnavailable)
			return
		}
		w.Header().Set("Content-Type", "image/jpeg")
		w.WriteHeader(http.StatusOK)
		_, _ = io.WriteString(w, "jpeg")
	}))
	defer server.Close()

	client := New(Config{Concurrency: 1})
	results, err := client.DownloadAll(filepath.Join(t.TempDir(), "assets"), []Resource{{URL: server.URL + "/cover.jpg"}})
	if err != nil {
		t.Fatalf("download should succeed after retry: %v", err)
	}
	if len(results) != 1 || results[0].Failed {
		t.Fatalf("download failed unexpectedly: %#v", results)
	}
	if got := attempts.Load(); got != 2 {
		t.Fatalf("expected two attempts, got %d", got)
	}
}

func TestDownloadAllTolerantModeKeepsOriginalURLOnFailure(t *testing.T) {
	t.Parallel()

	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		http.NotFound(w, r)
	}))
	defer server.Close()

	resourceURL := server.URL + "/missing.png"
	client := New(Config{Concurrency: 1, Strict: false})
	results, err := client.DownloadAll(filepath.Join(t.TempDir(), "assets"), []Resource{{URL: resourceURL}})
	if err != nil {
		t.Fatalf("tolerant mode should not return top-level error: %v", err)
	}
	if len(results) != 1 {
		t.Fatalf("expected one result, got %d", len(results))
	}
	if !results[0].Failed {
		t.Fatal("expected failed result")
	}
	if results[0].Err == nil {
		t.Fatal("expected per-resource error")
	}
	if results[0].Replacement != resourceURL {
		t.Fatalf("unexpected replacement: got %q want %q", results[0].Replacement, resourceURL)
	}
}

func TestDownloadAllStrictModeReturnsErrorOnFailure(t *testing.T) {
	t.Parallel()

	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		http.NotFound(w, r)
	}))
	defer server.Close()

	resourceURL := server.URL + "/missing.png"
	client := New(Config{Concurrency: 1, Strict: true})
	results, err := client.DownloadAll(filepath.Join(t.TempDir(), "assets"), []Resource{{URL: resourceURL}})
	if err == nil {
		t.Fatal("strict mode should return a top-level error")
	}
	if len(results) != 1 {
		t.Fatalf("expected partial result to be preserved, got %d results", len(results))
	}
	if !results[0].Failed {
		t.Fatal("expected failed result")
	}
	if results[0].Replacement != resourceURL {
		t.Fatalf("unexpected replacement: got %q want %q", results[0].Replacement, resourceURL)
	}
}

func TestDownloadAllEmitsReservedFilenameForFailureEvents(t *testing.T) {
	t.Parallel()

	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		http.NotFound(w, r)
	}))
	defer server.Close()

	sink := &recordingSink{}
	client := New(Config{Concurrency: 1, Events: sink})
	results, err := client.DownloadAll(filepath.Join(t.TempDir(), "assets"), []Resource{{URL: server.URL + "/missing.png"}})
	if err != nil {
		t.Fatalf("tolerant mode should not return top-level error: %v", err)
	}
	if len(results) != 1 {
		t.Fatalf("expected one result, got %d", len(results))
	}
	if !results[0].Failed {
		t.Fatal("expected failed result")
	}

	expected := []recordedEvent{
		{name: "missing.png", status: "Downloading"},
		{name: "missing.png", status: "Failed"},
	}
	if !reflect.DeepEqual(sink.events, expected) {
		t.Fatalf("unexpected events: got %#v want %#v", sink.events, expected)
	}
}

func TestDownloadAllSerializesConcurrentEvents(t *testing.T) {
	t.Parallel()

	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.WriteHeader(http.StatusOK)
		_, _ = io.WriteString(w, "asset")
	}))
	defer server.Close()

	sink := &reentryDetectingSink{}
	resources := []Resource{
		{URL: server.URL + "/one.png"},
		{URL: server.URL + "/two.png"},
		{URL: server.URL + "/three.png"},
		{URL: server.URL + "/four.png"},
	}
	client := New(Config{Concurrency: 4, Events: sink})

	results, err := client.DownloadAll(filepath.Join(t.TempDir(), "assets"), resources)
	if err != nil {
		t.Fatalf("download should succeed: %v", err)
	}
	if len(results) != len(resources) {
		t.Fatalf("expected %d results, got %d", len(resources), len(results))
	}
	if sink.concurrent.Load() {
		t.Fatal("event sink was called concurrently")
	}
	if got := sink.count.Load(); got != int64(len(resources)*2) {
		t.Fatalf("expected Downloading and Complete events for each resource, got %d events", got)
	}
}

func TestDownloadAllRetriesWhenFinalNameAppearsAfterReservation(t *testing.T) {
	t.Parallel()

	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.WriteHeader(http.StatusOK)
		_, _ = io.WriteString(w, "downloaded")
	}))
	defer server.Close()

	assetDir := filepath.Join(t.TempDir(), "assets")
	sink := eventFunc(func(name string, status string) {
		if status == "Downloading" {
			if err := os.WriteFile(filepath.Join(assetDir, "race.png"), []byte("late existing"), 0o644); err != nil {
				t.Errorf("write late collision file: %v", err)
			}
		}
	})

	client := New(Config{Concurrency: 1, Events: sink})
	results, err := client.DownloadAll(assetDir, []Resource{{URL: server.URL + "/race.png"}})
	if err != nil {
		t.Fatalf("download should succeed: %v", err)
	}
	if len(results) != 1 {
		t.Fatalf("expected one result, got %d", len(results))
	}
	if results[0].Replacement != "./assets/race_1.png" {
		t.Fatalf("unexpected replacement: got %q want %q", results[0].Replacement, "./assets/race_1.png")
	}
	if got, err := os.ReadFile(filepath.Join(assetDir, "race.png")); err != nil || string(got) != "late existing" {
		t.Fatalf("late existing file changed: got %q err %v", string(got), err)
	}
	if got, err := os.ReadFile(filepath.Join(assetDir, "race_1.png")); err != nil || string(got) != "downloaded" {
		t.Fatalf("downloaded file mismatch: got %q err %v", string(got), err)
	}
}

func TestDownloadAllRemovesTempFileWhenCopyFails(t *testing.T) {
	t.Parallel()

	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Length", "20")
		_, _ = io.WriteString(w, "short")
	}))
	defer server.Close()

	assetDir := filepath.Join(t.TempDir(), "assets")
	client := New(Config{Concurrency: 1})
	results, err := client.DownloadAll(assetDir, []Resource{{URL: server.URL + "/broken.png"}})
	if err != nil {
		t.Fatalf("tolerant mode should not return top-level error: %v", err)
	}
	if len(results) != 1 {
		t.Fatalf("expected one result, got %d", len(results))
	}
	if !results[0].Failed {
		t.Fatal("expected failed result")
	}
	if results[0].Err == nil {
		t.Fatal("expected per-resource copy error")
	}

	entries, readErr := os.ReadDir(assetDir)
	if readErr != nil {
		t.Fatalf("read asset dir: %v", readErr)
	}
	if len(entries) != 0 {
		t.Fatalf("expected no partial files after copy failure, found %d", len(entries))
	}
}

type reentryDetectingSink struct {
	active     atomic.Bool
	concurrent atomic.Bool
	count      atomic.Int64
}

func (s *reentryDetectingSink) EventName(name string, status string) {
	if !s.active.CompareAndSwap(false, true) {
		s.concurrent.Store(true)
		return
	}
	defer s.active.Store(false)

	s.count.Add(1)
	time.Sleep(5 * time.Millisecond)
}

type eventFunc func(name string, status string)

func (f eventFunc) EventName(name string, status string) {
	f(name, status)
}

type recordedEvent struct {
	name   string
	status string
}

type recordingSink struct {
	events []recordedEvent
}

func (s *recordingSink) EventName(name string, status string) {
	s.events = append(s.events, recordedEvent{name: name, status: status})
}
