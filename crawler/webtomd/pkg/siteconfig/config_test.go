package siteconfig

import (
	"os"
	"path/filepath"
	"testing"
)

func TestLoadValidConfig(t *testing.T) {
	t.Parallel()

	path := writeConfig(t, `{
		"version": 1,
		"sites": [{
			"name": "custom",
			"hosts": ["example.com"],
			"title": ["h1"],
			"content": ["article"],
			"remove": [".ad"],
			"image_attrs": ["data-src", "src"],
			"video_attrs": ["src"]
		}]
	}`)

	config, err := Load(path)
	if err != nil {
		t.Fatalf("load should succeed: %v", err)
	}
	if config.Sites[0].Name != "custom" {
		t.Fatalf("unexpected site name: %q", config.Sites[0].Name)
	}
}

func TestLoadRejectsMissingContentSelector(t *testing.T) {
	t.Parallel()

	path := writeConfig(t, `{
		"version": 1,
		"sites": [{
			"name": "custom",
			"hosts": ["example.com"]
		}]
	}`)

	_, err := Load(path)
	if err == nil {
		t.Fatal("expected missing content selectors to fail")
	}
}

func writeConfig(t *testing.T, content string) string {
	t.Helper()

	path := filepath.Join(t.TempDir(), "sites.json")
	if err := os.WriteFile(path, []byte(content), 0o644); err != nil {
		t.Fatalf("write config: %v", err)
	}
	return path
}
