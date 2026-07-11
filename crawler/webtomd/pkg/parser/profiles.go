package parser

import (
	"net/url"
	"strings"

	"webtomd/pkg/siteconfig"
)

type SiteProfile interface {
	Match(baseURL *url.URL) bool
	Parse(baseURL *url.URL, body []byte) (Result, bool, error)
}

var builtinProfiles = []SiteProfile{
	weChatProfile{},
	notionProfile{},
	nyTimesCNProfile{},
	weiboArticleProfile{},
}

func trySiteProfiles(baseURL *url.URL, body []byte) (Result, bool, error) {
	return tryProfiles(baseURL, body, builtinProfiles)
}

func tryProfiles(baseURL *url.URL, body []byte, profiles []SiteProfile) (Result, bool, error) {
	for _, profile := range profiles {
		if !profile.Match(baseURL) {
			continue
		}
		result, ok, err := profile.Parse(baseURL, body)
		if err != nil || ok {
			return result, ok, err
		}
	}
	return Result{}, false, nil
}

func ProfilesFromConfig(config siteconfig.Config) []SiteProfile {
	profiles := make([]SiteProfile, 0, len(config.Sites))
	for _, site := range config.Sites {
		profiles = append(profiles, configProfile{site: site})
	}
	return profiles
}

func ParseWithProfiles(pageURL string, body []byte, profiles []SiteProfile) (Result, error) {
	baseURL, err := url.Parse(pageURL)
	if err != nil {
		return Result{}, err
	}

	if result, ok, err := tryProfiles(baseURL, body, append(profiles, builtinProfiles...)); err != nil {
		return Result{}, err
	} else if ok {
		return result, nil
	}

	return Parse(pageURL, body)
}

func firstNonEmpty(values []string, fallback []string) []string {
	filtered := make([]string, 0, len(values))
	for _, value := range values {
		value = strings.TrimSpace(value)
		if value != "" {
			filtered = append(filtered, value)
		}
	}
	if len(filtered) == 0 {
		return fallback
	}
	return filtered
}
