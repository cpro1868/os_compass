package siteconfig

import (
	"encoding/json"
	"fmt"
	"os"
	"strings"
)

type Config struct {
	Version int    `json:"version"`
	Sites   []Site `json:"sites"`
}

type Site struct {
	Name       string   `json:"name"`
	Hosts      []string `json:"hosts"`
	Title      []string `json:"title"`
	Content    []string `json:"content"`
	Remove     []string `json:"remove"`
	ImageAttrs []string `json:"image_attrs"`
	VideoAttrs []string `json:"video_attrs"`
}

func Load(path string) (Config, error) {
	data, err := os.ReadFile(path)
	if err != nil {
		return Config{}, fmt.Errorf("read site config: %w", err)
	}

	var config Config
	if err := json.Unmarshal(data, &config); err != nil {
		return Config{}, fmt.Errorf("parse site config JSON: %w", err)
	}
	if err := config.Validate(); err != nil {
		return Config{}, err
	}
	return config, nil
}

func (c Config) Validate() error {
	if c.Version != 1 {
		return fmt.Errorf("site config version must be 1")
	}
	if len(c.Sites) == 0 {
		return fmt.Errorf("site config must contain at least one site")
	}
	for i, site := range c.Sites {
		if strings.TrimSpace(site.Name) == "" {
			return fmt.Errorf("site config site[%d] missing name", i)
		}
		if len(nonEmpty(site.Hosts)) == 0 {
			return fmt.Errorf("site config site[%s] missing hosts", site.Name)
		}
		if len(nonEmpty(site.Content)) == 0 {
			return fmt.Errorf("site config site[%s] missing content selectors", site.Name)
		}
	}
	return nil
}

func nonEmpty(values []string) []string {
	filtered := make([]string, 0, len(values))
	for _, value := range values {
		if strings.TrimSpace(value) != "" {
			filtered = append(filtered, value)
		}
	}
	return filtered
}
