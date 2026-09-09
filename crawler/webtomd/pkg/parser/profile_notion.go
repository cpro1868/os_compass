package parser

import (
	"bytes"
	"encoding/json"
	"fmt"
	"html"
	"io"
	"net/http"
	"net/url"
	"regexp"
	"strings"
	"time"
)

const notionLoadPageChunkURL = "https://www.notion.so/api/v3/loadPageChunk"

var notionPageIDPattern = regexp.MustCompile(`(?i)([0-9a-f]{8})-?([0-9a-f]{4})-?([0-9a-f]{4})-?([0-9a-f]{4})-?([0-9a-f]{12})`)

type notionProfile struct {
	loadPage  func(pageID string) (notionLoadPageResponse, error)
	signFiles func(files []notionFileReference) (map[notionFileReference]string, error)
}

type notionLoadPageResponse struct {
	Cursor    notionCursor    `json:"cursor"`
	RecordMap notionRecordMap `json:"recordMap"`
}

type notionLoadPageRequest struct {
	PageID          string       `json:"pageId"`
	Limit           int          `json:"limit"`
	Cursor          notionCursor `json:"cursor"`
	ChunkNumber     int          `json:"chunkNumber"`
	VerticalColumns bool         `json:"verticalColumns"`
}

type notionCursor struct {
	Stack []json.RawMessage `json:"stack"`
}

type notionRecordMap struct {
	Block map[string]notionBlockRecord `json:"block"`
}

type notionBlockRecord struct {
	Value notionBlockRecordValue `json:"value"`
}

type notionBlockRecordValue struct {
	Value notionBlock `json:"value"`
}

type notionBlock struct {
	ID         string                       `json:"id"`
	Type       string                       `json:"type"`
	Properties map[string]notionRawProperty `json:"properties"`
	Content    []string                     `json:"content"`
	Format     map[string]any               `json:"format"`
}

type notionRawProperty json.RawMessage

type notionFileReference struct {
	BlockID string
	URL     string
}

type notionSignedFileURLResponse struct {
	SignedURLs []string `json:"signedUrls"`
}

func (p *notionRawProperty) UnmarshalJSON(data []byte) error {
	if p == nil {
		return nil
	}
	*p = append((*p)[0:0], data...)
	return nil
}

func (notionProfile) Match(baseURL *url.URL) bool {
	host := strings.ToLower(baseURL.Hostname())
	if host == "www.notion.so" || host == "notion.so" {
		return extractNotionPageID(baseURL, nil) != ""
	}
	return host == "notion.site" || strings.HasSuffix(host, ".notion.site")
}

func (p notionProfile) Parse(baseURL *url.URL, body []byte) (Result, bool, error) {
	pageID := extractNotionPageID(baseURL, body)
	if pageID == "" {
		return Result{}, false, nil
	}

	loadPage := p.loadPage
	if loadPage == nil {
		loadPage = defaultLoadNotionPage
	}

	response, err := loadPage(pageID)
	if err != nil {
		return Result{}, false, fmt.Errorf("load notion page: %w", err)
	}

	signFiles := p.signFiles
	if signFiles == nil {
		signFiles = defaultSignNotionFileURLs
	}
	signedFiles, err := signFiles(collectNotionFileReferences(response.RecordMap))
	if err != nil {
		return Result{}, false, fmt.Errorf("sign notion files: %w", err)
	}

	result, err := renderNotionPage(baseURL, response.RecordMap, pageID, signedFiles)
	if err != nil {
		return Result{}, false, err
	}
	if !result.HasContent {
		return Result{}, false, nil
	}

	return result, true, nil
}

func extractNotionPageID(baseURL *url.URL, body []byte) string {
	if baseURL != nil {
		if id := normalizeNotionPageID(baseURL.Path); id != "" {
			return id
		}
		if id := normalizeNotionPageID(baseURL.RawQuery); id != "" {
			return id
		}
	}
	return normalizeNotionPageID(string(body))
}

func normalizeNotionPageID(value string) string {
	match := notionPageIDPattern.FindStringSubmatch(value)
	if len(match) != 6 {
		return ""
	}
	return strings.ToLower(fmt.Sprintf("%s-%s-%s-%s-%s", match[1], match[2], match[3], match[4], match[5]))
}

func defaultLoadNotionPage(pageID string) (notionLoadPageResponse, error) {
	return loadNotionPageChunks(pageID, fetchNotionPageChunk)
}

func loadNotionPageChunks(pageID string, fetch func(notionLoadPageRequest) (notionLoadPageResponse, error)) (notionLoadPageResponse, error) {
	merged := notionLoadPageResponse{RecordMap: notionRecordMap{Block: map[string]notionBlockRecord{}}}
	cursor := notionCursor{Stack: []json.RawMessage{}}

	for chunkNumber := 0; chunkNumber < 20; chunkNumber++ {
		response, err := fetch(notionLoadPageRequest{
			PageID:          pageID,
			Limit:           200,
			Cursor:          cursor,
			ChunkNumber:     chunkNumber,
			VerticalColumns: false,
		})
		if err != nil {
			return notionLoadPageResponse{}, err
		}
		for id, block := range response.RecordMap.Block {
			merged.RecordMap.Block[id] = block
		}
		if len(response.Cursor.Stack) == 0 {
			return merged, nil
		}
		cursor = response.Cursor
	}

	return merged, fmt.Errorf("notion page has too many chunks")
}

func fetchNotionPageChunk(payload notionLoadPageRequest) (notionLoadPageResponse, error) {
	payloadBody, err := json.Marshal(payload)
	if err != nil {
		return notionLoadPageResponse{}, fmt.Errorf("encode request: %w", err)
	}

	request, err := http.NewRequest(http.MethodPost, notionLoadPageChunkURL, bytes.NewReader(payloadBody))
	if err != nil {
		return notionLoadPageResponse{}, fmt.Errorf("create request: %w", err)
	}
	request.Header.Set("Content-Type", "application/json")
	request.Header.Set("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0 Safari/537.36")
	request.Header.Set("Notion-Client-Version", "23.13.20260421.2304")

	client := http.Client{Timeout: 45 * time.Second}
	response, err := client.Do(request)
	if err != nil {
		return notionLoadPageResponse{}, err
	}
	defer response.Body.Close()

	if response.StatusCode < 200 || response.StatusCode >= 300 {
		body, _ := io.ReadAll(io.LimitReader(response.Body, 512))
		return notionLoadPageResponse{}, fmt.Errorf("notion API returned %d: %s", response.StatusCode, strings.TrimSpace(string(body)))
	}

	var parsed notionLoadPageResponse
	if err := json.NewDecoder(response.Body).Decode(&parsed); err != nil {
		return notionLoadPageResponse{}, fmt.Errorf("decode response: %w", err)
	}
	return parsed, nil
}

func defaultSignNotionFileURLs(files []notionFileReference) (map[notionFileReference]string, error) {
	signed := make(map[notionFileReference]string, len(files))
	if len(files) == 0 {
		return signed, nil
	}

	urls := make([]map[string]any, 0, len(files))
	for _, file := range files {
		urls = append(urls, map[string]any{
			"permissionRecord": map[string]string{
				"table": "block",
				"id":    file.BlockID,
			},
			"url": file.URL,
		})
	}
	payloadBody, err := json.Marshal(map[string]any{"urls": urls})
	if err != nil {
		return nil, fmt.Errorf("encode request: %w", err)
	}

	request, err := http.NewRequest(http.MethodPost, "https://www.notion.so/api/v3/getSignedFileUrls", bytes.NewReader(payloadBody))
	if err != nil {
		return nil, fmt.Errorf("create request: %w", err)
	}
	request.Header.Set("Content-Type", "application/json")
	request.Header.Set("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0 Safari/537.36")
	request.Header.Set("Notion-Client-Version", "23.13.20260421.2304")

	client := http.Client{Timeout: 45 * time.Second}
	response, err := client.Do(request)
	if err != nil {
		return nil, err
	}
	defer response.Body.Close()

	if response.StatusCode < 200 || response.StatusCode >= 300 {
		body, _ := io.ReadAll(io.LimitReader(response.Body, 512))
		return nil, fmt.Errorf("notion API returned %d: %s", response.StatusCode, strings.TrimSpace(string(body)))
	}

	var parsed notionSignedFileURLResponse
	if err := json.NewDecoder(response.Body).Decode(&parsed); err != nil {
		return nil, fmt.Errorf("decode response: %w", err)
	}
	for index, signedURL := range parsed.SignedURLs {
		if index >= len(files) {
			break
		}
		if signedURL != "" {
			signed[files[index]] = signedURL
		}
	}
	return signed, nil
}

func collectNotionFileReferences(recordMap notionRecordMap) []notionFileReference {
	seen := map[notionFileReference]bool{}
	var files []notionFileReference
	for _, record := range recordMap.Block {
		block := record.Value.Value
		if block.Type != "image" && block.Type != "video" && block.Type != "file" {
			continue
		}
		source := notionPlainProperty(block.Properties["source"])
		if !strings.HasPrefix(source, "attachment:") {
			continue
		}
		file := notionFileReference{BlockID: block.ID, URL: source}
		if file.BlockID == "" || seen[file] {
			continue
		}
		seen[file] = true
		files = append(files, file)
	}
	return files
}

func renderNotionPage(baseURL *url.URL, recordMap notionRecordMap, pageID string, signedFiles map[notionFileReference]string) (Result, error) {
	page, ok := notionBlockByID(recordMap, pageID)
	if !ok {
		return Result{HasContent: false}, nil
	}

	title := notionPlainProperty(page.Properties["title"])
	var builder strings.Builder
	visited := map[string]bool{}
	renderNotionChildren(&builder, recordMap, page.Content, visited, signedFiles)
	htmlBody := strings.TrimSpace(builder.String())
	if htmlBody == "" {
		return Result{Title: title, HasContent: false}, nil
	}

	resources, err := collectResources(baseURL, htmlBody)
	if err != nil {
		return Result{}, err
	}

	return Result{
		HTML:       htmlBody,
		Title:      title,
		Resources:  resources,
		HasContent: true,
	}, nil
}

func notionBlockByID(recordMap notionRecordMap, id string) (notionBlock, bool) {
	record, ok := recordMap.Block[id]
	if !ok {
		return notionBlock{}, false
	}
	return record.Value.Value, true
}

func renderNotionChildren(builder *strings.Builder, recordMap notionRecordMap, ids []string, visited map[string]bool, signedFiles map[notionFileReference]string) {
	for _, id := range ids {
		renderNotionBlock(builder, recordMap, id, visited, signedFiles)
	}
}

func renderNotionBlock(builder *strings.Builder, recordMap notionRecordMap, id string, visited map[string]bool, signedFiles map[notionFileReference]string) {
	if visited[id] {
		return
	}
	visited[id] = true

	block, ok := notionBlockByID(recordMap, id)
	if !ok {
		return
	}

	titleHTML := notionHTMLProperty(block.Properties["title"])
	titleText := notionPlainProperty(block.Properties["title"])
	source := notionBlockSource(block, signedFiles)
	if source == "" && block.Format != nil {
		if displaySource, ok := block.Format["display_source"].(string); ok {
			source = strings.TrimSpace(displaySource)
		}
	}

	switch block.Type {
	case "page", "column_list", "column", "toggle":
		renderNotionChildren(builder, recordMap, block.Content, visited, signedFiles)
	case "header":
		writeTag(builder, "h2", titleHTML)
		renderNotionChildren(builder, recordMap, block.Content, visited, signedFiles)
	case "sub_header":
		writeTag(builder, "h3", titleHTML)
		renderNotionChildren(builder, recordMap, block.Content, visited, signedFiles)
	case "sub_sub_header":
		writeTag(builder, "h4", titleHTML)
		renderNotionChildren(builder, recordMap, block.Content, visited, signedFiles)
	case "text":
		writeTag(builder, "p", titleHTML)
		renderNotionChildren(builder, recordMap, block.Content, visited, signedFiles)
	case "quote", "callout":
		if titleHTML == "" && len(block.Content) == 0 {
			return
		}
		builder.WriteString("<blockquote>")
		writeTag(builder, "p", titleHTML)
		renderNotionChildren(builder, recordMap, block.Content, visited, signedFiles)
		builder.WriteString("</blockquote>")
	case "code":
		if titleText != "" {
			builder.WriteString("<pre><code>")
			builder.WriteString(html.EscapeString(titleText))
			builder.WriteString("</code></pre>")
		}
	case "bulleted_list":
		if titleHTML != "" || len(block.Content) > 0 {
			builder.WriteString("<ul><li>")
			builder.WriteString(titleHTML)
			renderNotionChildren(builder, recordMap, block.Content, visited, signedFiles)
			builder.WriteString("</li></ul>")
		}
	case "numbered_list":
		if titleHTML != "" || len(block.Content) > 0 {
			builder.WriteString("<ol><li>")
			builder.WriteString(titleHTML)
			renderNotionChildren(builder, recordMap, block.Content, visited, signedFiles)
			builder.WriteString("</li></ol>")
		}
	case "to_do":
		if titleHTML != "" {
			builder.WriteString(`<p><input type="checkbox" disabled> `)
			builder.WriteString(titleHTML)
			builder.WriteString("</p>")
		}
		renderNotionChildren(builder, recordMap, block.Content, visited, signedFiles)
	case "image":
		if source != "" {
			builder.WriteString(`<p><img src="`)
			builder.WriteString(html.EscapeString(source))
			builder.WriteString(`" alt=""></p>`)
		}
	case "video":
		if source != "" {
			builder.WriteString(`<p><video controls src="`)
			builder.WriteString(html.EscapeString(source))
			builder.WriteString(`"></video></p>`)
		}
	case "divider":
		builder.WriteString("<hr>")
	default:
		writeTag(builder, "p", titleHTML)
		renderNotionChildren(builder, recordMap, block.Content, visited, signedFiles)
	}
}

func notionBlockSource(block notionBlock, signedFiles map[notionFileReference]string) string {
	source := notionPlainProperty(block.Properties["source"])
	if source == "" {
		return ""
	}
	if signedURL, ok := signedFiles[notionFileReference{BlockID: block.ID, URL: source}]; ok {
		return signedURL
	}
	return source
}

func writeTag(builder *strings.Builder, tag string, content string) {
	if strings.TrimSpace(content) == "" {
		return
	}
	builder.WriteString("<")
	builder.WriteString(tag)
	builder.WriteString(">")
	builder.WriteString(content)
	builder.WriteString("</")
	builder.WriteString(tag)
	builder.WriteString(">")
}

func notionHTMLProperty(raw notionRawProperty) string {
	var segments []any
	if len(raw) == 0 || json.Unmarshal(json.RawMessage(raw), &segments) != nil {
		return ""
	}

	var builder strings.Builder
	for _, segment := range segments {
		segmentValues, ok := segment.([]any)
		if !ok || len(segmentValues) == 0 {
			continue
		}
		text, ok := segmentValues[0].(string)
		if !ok || text == "" {
			continue
		}
		escaped := html.EscapeString(text)
		link := notionLinkAnnotation(segmentValues)
		if link != "" {
			builder.WriteString(`<a href="`)
			builder.WriteString(html.EscapeString(link))
			builder.WriteString(`">`)
			builder.WriteString(escaped)
			builder.WriteString("</a>")
			continue
		}
		builder.WriteString(escaped)
	}
	return strings.TrimSpace(builder.String())
}

func notionPlainProperty(raw notionRawProperty) string {
	var segments []any
	if len(raw) == 0 || json.Unmarshal(json.RawMessage(raw), &segments) != nil {
		return ""
	}

	var builder strings.Builder
	for _, segment := range segments {
		segmentValues, ok := segment.([]any)
		if !ok || len(segmentValues) == 0 {
			continue
		}
		text, ok := segmentValues[0].(string)
		if ok {
			builder.WriteString(text)
		}
	}
	return strings.TrimSpace(builder.String())
}

func notionLinkAnnotation(segment []any) string {
	if len(segment) < 2 {
		return ""
	}
	annotations, ok := segment[1].([]any)
	if !ok {
		return ""
	}
	for _, annotation := range annotations {
		values, ok := annotation.([]any)
		if !ok || len(values) < 2 {
			continue
		}
		if code, ok := values[0].(string); !ok || code != "a" {
			continue
		}
		link, _ := values[1].(string)
		return strings.TrimSpace(link)
	}
	return ""
}
