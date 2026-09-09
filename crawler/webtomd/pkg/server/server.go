package server

import (
	"encoding/json"
	"fmt"
	"log"
	"net/http"
	"strings"
	"time"

	"github.com/spf13/cobra"

	"webtomd/pkg/app"
)

type Config struct {
	Port    int
	Host    string
	WorkDir string
	Proxy   string
}

type ConvertRequest struct {
	URL            string `json:"url"`
	Name           string `json:"name,omitempty"`
	Strict         bool   `json:"strict,omitempty"`
	SiteConfigPath string `json:"site_config_path,omitempty"`
	Cookie         string `json:"cookie,omitempty"`
	BrowserProfile string `json:"browser_profile,omitempty"`
	Proxy          string `json:"proxy,omitempty"`
}

type ConvertResponse struct {
	Title    string `json:"title"`
	Markdown string `json:"markdown"`
	Source   string `json:"source"`
	FetchedAt string `json:"fetched_at"`
}

type HealthResponse struct {
	Status string `json:"status"`
}

func RunServer(cfg Config) error {
	addr := fmt.Sprintf("%s:%d", cfg.Host, cfg.Port)
	
	mux := http.NewServeMux()
	mux.HandleFunc("GET /health", handleHealth)
	mux.HandleFunc("POST /convert", func(w http.ResponseWriter, r *http.Request) {
		handleConvert(w, r, cfg)
	})
	mux.HandleFunc("GET /", func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "application/json")
		json.NewEncoder(w).Encode(map[string]interface{}{
			"service": "web2md",
			"version": "1.0.0",
			"endpoints": []string{
				"GET  /health",
				"POST /convert",
			},
		})
	})

	server := &http.Server{
		Addr:         addr,
		Handler:      mux,
		ReadTimeout:  60 * time.Second,
		WriteTimeout: 120 * time.Second,
		IdleTimeout:  120 * time.Second,
	}

	log.Printf("web2md HTTP 服务启动: http://%s", addr)
	log.Printf("健康检查: GET http://%s/health", addr)
	log.Printf("转换接口: POST http://%s/convert", addr)

	return server.ListenAndServe()
}

func handleHealth(w http.ResponseWriter, r *http.Request) {
	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(HealthResponse{
		Status: "ok",
	})
}

func handleConvert(w http.ResponseWriter, r *http.Request, cfg Config) {
	w.Header().Set("Content-Type", "application/json")
	w.Header().Set("Access-Control-Allow-Origin", "*")
	w.Header().Set("Access-Control-Allow-Methods", "POST, OPTIONS")
	w.Header().Set("Access-Control-Allow-Headers", "Content-Type")

	if r.Method == "OPTIONS" {
		w.WriteHeader(http.StatusNoContent)
		return
	}

	if r.Method != "POST" {
		http.Error(w, `{"error": "Method not allowed, use POST"}`, http.StatusMethodNotAllowed)
		return
	}

	var req ConvertRequest
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		http.Error(w, fmt.Sprintf(`{"error": "Invalid JSON: %s"}`, err.Error()), http.StatusBadRequest)
		return
	}

	req.URL = strings.TrimSpace(req.URL)
	if req.URL == "" {
		http.Error(w, `{"error": "url is required"}`, http.StatusBadRequest)
		return
	}

	proxy := req.Proxy
	if proxy == "" {
		proxy = cfg.Proxy
	}

	config := app.Config{
		URL:            req.URL,
		Name:           req.Name,
		WorkDir:        cfg.WorkDir,
		Strict:         req.Strict,
		SiteConfigPath: req.SiteConfigPath,
		Cookie:         req.Cookie,
		BrowserProfile: req.BrowserProfile,
		Proxy:          proxy,
	}

	markdown, err := app.ConvertToMarkdown(config)
	if err != nil {
		http.Error(w, fmt.Sprintf(`{"error": %s}`, jsonEscape(err.Error())), http.StatusInternalServerError)
		return
	}

	lines := strings.SplitN(markdown, "\n", 2)
	title := ""
	if len(lines) > 0 && strings.HasPrefix(lines[0], "# ") {
		title = strings.TrimPrefix(lines[0], "# ")
	}

	resp := ConvertResponse{
		Title:     title,
		Markdown:  markdown,
		Source:    "http",
		FetchedAt: time.Now().Format(time.RFC3339),
	}

	json.NewEncoder(w).Encode(resp)
}

func jsonEscape(s string) string {
	b, _ := json.Marshal(s)
	return string(b)
}

func NewServeCommand() *cobra.Command {
	cfg := &Config{
		Port: 8080,
		Host: "127.0.0.1",
	}

	cmd := &cobra.Command{
		Use:   "serve",
		Short: "启动 web2md HTTP 服务",
		Long: `启动 web2md HTTP 服务，提供网页转 Markdown 的 API 接口。

端点:
  GET  /health   - 健康检查
  POST /convert  - 转换 URL 为 Markdown

示例:
  web2md serve --port 8080
  web2md serve --port 9000 --host 0.0.0.0`,
		RunE: func(cmd *cobra.Command, args []string) error {
			return RunServer(*cfg)
		},
	}

	cmd.Flags().IntVarP(&cfg.Port, "port", "p", 8080, "HTTP 服务端口")
	cmd.Flags().StringVar(&cfg.Host, "host", "127.0.0.1", "HTTP 服务监听地址")
	cmd.Flags().StringVar(&cfg.Proxy, "proxy", "", "HTTP 代理地址（如 http://127.0.0.1:8964）")

	return cmd
}
