package parser

import (
	"encoding/json"
	"net/url"
	"strings"
	"testing"
)

func TestExtractNotionPageID(t *testing.T) {
	t.Parallel()

	baseURL := mustParseURL(t, "https://iyouport.notion.site/S07E05-24c34ca2d46d808985a0f63a22dde6c7")
	body := []byte(`__notion_html_async.push("requiredRedirectMetadata",{"pageId":"ignored"})`)

	pageID := extractNotionPageID(baseURL, body)
	if pageID != "24c34ca2-d46d-8089-85a0-f63a22dde6c7" {
		t.Fatalf("unexpected page id: %q", pageID)
	}
}

func TestNotionProfileRendersBlockMap(t *testing.T) {
	t.Parallel()

	baseURL := mustParseURL(t, "https://example.notion.site/Demo-24c34ca2d46d808985a0f63a22dde6c7")
	profile := notionProfile{
		loadPage: func(pageID string) (notionLoadPageResponse, error) {
			if pageID != "24c34ca2-d46d-8089-85a0-f63a22dde6c7" {
				t.Fatalf("unexpected loaded page id: %q", pageID)
			}
			return notionLoadPageResponse{RecordMap: notionRecordMap{Block: map[string]notionBlockRecord{
				"24c34ca2-d46d-8089-85a0-f63a22dde6c7": notionBlockFixture(notionBlock{
					ID:      "24c34ca2-d46d-8089-85a0-f63a22dde6c7",
					Type:    "page",
					Content: []string{"header", "text", "quote", "code", "image"},
					Properties: map[string]notionRawProperty{
						"title": rawNotionProperty(`[["Demo Notion Page"]]`),
					},
				}),
				"header": notionBlockFixture(notionBlock{
					ID:   "header",
					Type: "header",
					Properties: map[string]notionRawProperty{
						"title": rawNotionProperty(`[["Section One"]]`),
					},
				}),
				"text": notionBlockFixture(notionBlock{
					ID:   "text",
					Type: "text",
					Properties: map[string]notionRawProperty{
						"title": rawNotionProperty(`[["Body with "],["a link",[["a","https://example.com"]]]]`),
					},
				}),
				"quote": notionBlockFixture(notionBlock{
					ID:   "quote",
					Type: "quote",
					Properties: map[string]notionRawProperty{
						"title": rawNotionProperty(`[["Quoted text"]]`),
					},
				}),
				"code": notionBlockFixture(notionBlock{
					ID:   "code",
					Type: "code",
					Properties: map[string]notionRawProperty{
						"title":    rawNotionProperty(`[["fmt.Println(\"ok\")"]]`),
						"language": rawNotionProperty(`[["go"]]`),
					},
				}),
				"image": notionBlockFixture(notionBlock{
					ID:   "image",
					Type: "image",
					Properties: map[string]notionRawProperty{
						"source": rawNotionProperty(`[["attachment:cover:image.png"]]`),
					},
				}),
			}}}, nil
		},
		signFiles: func(files []notionFileReference) (map[notionFileReference]string, error) {
			if len(files) != 1 || files[0].BlockID != "image" || files[0].URL != "attachment:cover:image.png" {
				t.Fatalf("unexpected files to sign: %#v", files)
			}
			return map[notionFileReference]string{
				files[0]: "https://file.notion.so/f/example/image.png?signature=ok",
			}, nil
		},
	}

	result, ok, err := profile.Parse(baseURL, []byte(`<div id="notion-app"></div>`))
	if err != nil {
		t.Fatalf("parse notion profile: %v", err)
	}
	if !ok || !result.HasContent {
		t.Fatalf("expected notion content, ok=%v result=%#v", ok, result)
	}
	if result.Title != "Demo Notion Page" {
		t.Fatalf("unexpected title: %q", result.Title)
	}
	for _, expected := range []string{
		"<h2>Section One</h2>",
		`<p>Body with <a href="https://example.com">a link</a></p>`,
		"<blockquote><p>Quoted text</p></blockquote>",
		"<pre><code>fmt.Println(&#34;ok&#34;)</code></pre>",
		`<img src="https://file.notion.so/f/example/image.png?signature=ok"`,
	} {
		if !strings.Contains(result.HTML, expected) {
			t.Fatalf("expected HTML to contain %q, got %q", expected, result.HTML)
		}
	}
	if len(result.Resources) != 1 || result.Resources[0].ResolvedURL != "https://file.notion.so/f/example/image.png?signature=ok" {
		t.Fatalf("unexpected resources: %#v", result.Resources)
	}
}

func TestNotionLoadPageResponseDecodesRawProperties(t *testing.T) {
	t.Parallel()

	body := []byte(`{
		"recordMap": {
			"block": {
				"page-id": {
					"value": {
						"value": {
							"id": "page-id",
							"type": "page",
							"properties": {
								"title": [["Decoded Title"]]
							}
						}
					}
				}
			}
		}
	}`)

	var response notionLoadPageResponse
	if err := json.Unmarshal(body, &response); err != nil {
		t.Fatalf("decode response: %v", err)
	}
	page := response.RecordMap.Block["page-id"].Value.Value
	if got := notionPlainProperty(page.Properties["title"]); got != "Decoded Title" {
		t.Fatalf("unexpected decoded title: %q", got)
	}
}

func TestLoadNotionPageChunksFollowsCursor(t *testing.T) {
	t.Parallel()

	var chunks []int
	response, err := loadNotionPageChunks("page-id", func(request notionLoadPageRequest) (notionLoadPageResponse, error) {
		chunks = append(chunks, request.ChunkNumber)
		if request.ChunkNumber == 0 {
			if len(request.Cursor.Stack) != 0 {
				t.Fatalf("expected empty initial cursor, got %#v", request.Cursor.Stack)
			}
			return notionLoadPageResponse{
				Cursor: notionCursor{Stack: []json.RawMessage{json.RawMessage(`{"id":"next"}`)}},
				RecordMap: notionRecordMap{Block: map[string]notionBlockRecord{
					"page-id": notionBlockFixture(notionBlock{ID: "page-id", Type: "page", Content: []string{"one"}}),
					"one":     notionBlockFixture(notionBlock{ID: "one", Type: "text"}),
				}},
			}, nil
		}
		if len(request.Cursor.Stack) != 1 {
			t.Fatalf("expected cursor from first chunk, got %#v", request.Cursor.Stack)
		}
		return notionLoadPageResponse{
			Cursor: notionCursor{},
			RecordMap: notionRecordMap{Block: map[string]notionBlockRecord{
				"two": notionBlockFixture(notionBlock{ID: "two", Type: "text"}),
			}},
		}, nil
	})
	if err != nil {
		t.Fatalf("load chunks: %v", err)
	}
	if len(chunks) != 2 || chunks[0] != 0 || chunks[1] != 1 {
		t.Fatalf("unexpected chunks loaded: %#v", chunks)
	}
	if len(response.RecordMap.Block) != 3 {
		t.Fatalf("expected merged blocks from both chunks, got %#v", response.RecordMap.Block)
	}
}

func TestNotionProfileDoesNotMatchPlainNotionMarketingPage(t *testing.T) {
	t.Parallel()

	baseURL, err := url.Parse("https://www.notion.so/product")
	if err != nil {
		t.Fatalf("parse URL: %v", err)
	}
	if (notionProfile{}).Match(baseURL) {
		t.Fatal("expected marketing page not to match notion profile")
	}
}

func notionBlockFixture(block notionBlock) notionBlockRecord {
	return notionBlockRecord{Value: notionBlockRecordValue{Value: block}}
}

func rawNotionProperty(value string) notionRawProperty {
	return notionRawProperty(value)
}
