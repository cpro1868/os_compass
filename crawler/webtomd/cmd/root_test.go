package cmd

import (
	"bytes"
	"strings"
	"testing"
)

func TestRootRequiresName(t *testing.T) {
	var stdout bytes.Buffer
	var stderr bytes.Buffer
	root := NewRootCommand(&stdout, &stderr)
	root.SetArgs([]string{"https://example.com/article"})

	err := root.Execute()
	if err == nil {
		t.Fatal("expected missing name to fail")
	}
	if !strings.Contains(stderr.String(), "需要提供 -n") {
		t.Fatalf("expected friendly missing-name error, got %q", stderr.String())
	}
	if !strings.Contains(stderr.String(), "Usage:") {
		t.Fatalf("expected help output, got %q", stderr.String())
	}
}

func TestRootHelpMentionsStrict(t *testing.T) {
	var stdout bytes.Buffer
	var stderr bytes.Buffer
	root := NewRootCommand(&stdout, &stderr)
	root.SetArgs([]string{"--help"})

	err := root.Execute()
	if err != nil {
		t.Fatalf("help should not fail: %v", err)
	}
	if !strings.Contains(stdout.String(), "--strict") {
		t.Fatalf("expected help to mention strict mode, got %q", stdout.String())
	}
	if !strings.Contains(stdout.String(), "--site-config") {
		t.Fatalf("expected help to mention site config, got %q", stdout.String())
	}
	if !strings.Contains(stdout.String(), "--cookie") {
		t.Fatalf("expected help to mention cookie, got %q", stdout.String())
	}
	if !strings.Contains(stdout.String(), "--browser-profile") {
		t.Fatalf("expected help to mention browser profile, got %q", stdout.String())
	}
}

func TestRootRunsWithName(t *testing.T) {
	var stdout bytes.Buffer
	var stderr bytes.Buffer
	called := false
	root := NewRootCommandWithRunner(&stdout, &stderr, func(opts Options, url string) error {
		called = true
		if opts.Name != "article-note" {
			t.Fatalf("unexpected name: %q", opts.Name)
		}
		if !opts.Strict {
			t.Fatal("expected strict option true")
		}
		if opts.SiteConfigPath != "sites.json" {
			t.Fatalf("unexpected site config path: %q", opts.SiteConfigPath)
		}
		if opts.Cookie != "SUB=abc" {
			t.Fatalf("unexpected cookie: %q", opts.Cookie)
		}
		if opts.BrowserProfile != `C:\Users\me\AppData\Local\Google\Chrome\User Data\Default` {
			t.Fatalf("unexpected browser profile: %q", opts.BrowserProfile)
		}
		if url != "https://example.com/article" {
			t.Fatalf("unexpected url: %q", url)
		}
		return nil
	})
	root.SetArgs([]string{"https://example.com/article", "-n", "article-note", "--strict", "--site-config", "sites.json", "--cookie", "SUB=abc", "--browser-profile", `C:\Users\me\AppData\Local\Google\Chrome\User Data\Default`})

	err := root.Execute()
	if err != nil {
		t.Fatalf("expected run to succeed: %v", err)
	}
	if !called {
		t.Fatal("expected runner to be called")
	}
}
