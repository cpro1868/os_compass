package parser

import (
	"net/url"
	"reflect"
	"strings"
	"testing"

	"webtomd/pkg/siteconfig"
)

func TestConfigProfileExtractsConfiguredSite(t *testing.T) {
	t.Parallel()

	config := siteconfig.Config{
		Version: 1,
		Sites: []siteconfig.Site{{
			Name:       "custom",
			Hosts:      []string{"example.com"},
			Title:      []string{".headline"},
			Content:    []string{".article-body"},
			Remove:     []string{".ad"},
			ImageAttrs: []string{"data-src", "src"},
		}},
	}
	body := []byte(`<!doctype html><html><head><title>fallback</title></head><body>
		<h1 class="headline">Custom Title</h1>
		<div class="article-body">
			<p>Custom article body should remain.</p>
			<p class="ad">Advertisement should be removed.</p>
			<img data-src="/image/cover" src="/placeholder.gif">
		</div>
	</body></html>`)

	result, err := ParseWithProfiles("https://example.com/post", body, ProfilesFromConfig(config))
	if err != nil {
		t.Fatalf("parse with profiles should succeed: %v", err)
	}
	if result.Title != "Custom Title" {
		t.Fatalf("unexpected title: %q", result.Title)
	}
	if !strings.Contains(result.HTML, "Custom article body should remain") {
		t.Fatalf("expected configured body in HTML: %q", result.HTML)
	}
	if strings.Contains(result.HTML, "Advertisement") {
		t.Fatalf("expected remove selector to drop ad: %q", result.HTML)
	}

	expectedResources := []Resource{{
		Type:        ResourceTypeImage,
		OriginalURL: "/image/cover",
		ResolvedURL: "https://example.com/image/cover",
		Attr:        "src",
	}}
	if !reflect.DeepEqual(result.Resources, expectedResources) {
		t.Fatalf("unexpected resources:\n got: %#v\nwant: %#v", result.Resources, expectedResources)
	}
}

func TestConfigProfileMatchUsesHostname(t *testing.T) {
	t.Parallel()

	profile := configProfile{site: siteconfig.Site{Hosts: []string{"example.com"}}}
	parsed, err := url.Parse("https://example.com:443/post")
	if err != nil {
		t.Fatal(err)
	}
	if !profile.Match(parsed) {
		t.Fatal("expected profile to match host with port")
	}
}
