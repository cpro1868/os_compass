# web2md MVP Design

## Goal

`web2md` is a cross-platform Go CLI that converts a web article into an offline Markdown note for Obsidian and Typora. It extracts the readable article body, downloads article media into a local `assets` directory, rewrites media links to local relative paths, and injects source metadata.

The final user experience should require minimal setup: users download a single executable and run it from the directory where they want the Markdown file created.

## CLI Contract

Command name: `web2md`.

Primary usage:

```bash
web2md <URL> -n <document-name>
web2md <URL> -n <document-name> --strict
```

Arguments and flags:

- `<URL>` is required. The CLI does not perform strict URL validation before fetching; unreachable or unsupported targets fail through the fetch path with a friendly error.
- `-n, --name` is required and is the Markdown filename without the `.md` suffix.
- `--strict` is optional. It changes resource download failures from warnings into a non-zero command failure.
- `-h, --help` prints command help, argument descriptions, and examples.

If `-n` is missing, the CLI exits with a non-zero status, prints a friendly error, and shows help.

## Output Layout

All output is written relative to the current working directory:

```text
./<document-name>.md
./assets/
```

If `<document-name>.md` already exists, it is overwritten. If `assets/` already exists, existing files are preserved. New media files must avoid collisions with existing files by appending numeric suffixes before the extension, for example `image.png`, `image_1.png`, `image_2.png`.

## Fetching

The MVP fetcher uses normal HTTP behavior only:

- Set a reasonable browser-like `User-Agent`.
- Use a default timeout of 30 seconds.
- Follow the Go HTTP client's default redirect behavior, up to 10 redirects.
- Do not implement browser rendering, JavaScript execution, login sessions, CAPTCHA handling, or advanced anti-bot bypasses.

Network failures, 404 responses, and other non-successful fetches exit safely with a non-zero status and an error message.

## Article Extraction

The parser uses a Readability-style extraction step to keep the article body and remove navigation, sidebars, footers, ads, and other non-content DOM. If the page fetch succeeds but no useful article body can be extracted, `web2md` still writes a Markdown file containing frontmatter and a body line:

```markdown
原文链接：<URL>
```

The command exits successfully in this case but prints a warning that the article body could not be extracted.

## Media Discovery

Only media inside the extracted article body is considered.

Supported image source:

- `img[src]`

Supported video sources:

- `video[src]`
- `video source[src]`

Video downloads are limited to direct links with common video file extensions such as `.mp4`, `.webm`, `.mov`, and `.m4v`. Streaming manifests such as `.m3u8`, iframe players, embedded third-party players, and media without a direct downloadable URL are ignored.

Relative media URLs are resolved against the original article URL before downloading.

## Resource Downloading

Resources are downloaded concurrently with a default concurrency of 5. The CLI should show multi-file download feedback inspired by `docker pull`: each active or recently completed file gets a status line that communicates pending, downloading, completed, skipped, or failed state. The implementation may keep the display simple as long as users can see that multiple downloads are progressing.

In default tolerant mode:

- Failed media downloads do not fail the command.
- The generated Markdown keeps the original remote URL for that media.
- The final summary prints the number of failed resources.

In `--strict` mode:

- Any media download failure stops the command and returns a non-zero status.
- Already downloaded files and any written output are preserved.
- No rollback or cleanup is attempted.

## Markdown Conversion

After media handling, the converter turns the extracted article HTML into Markdown. Successfully downloaded media references are rewritten to local relative paths such as:

```markdown
![alt text](./assets/image_1.png)
```

Failed resources in tolerant mode keep their original remote URL. The generated Markdown begins with YAML frontmatter:

```yaml
---
original_url: <URL>
fetch_date: <local time in YYYY-MM-DD HH:mm:ss>
---
```

## Architecture

The project should use a small Go module with focused packages:

```text
cmd/
  root.go          # Cobra command, flags, validation, help text
pkg/
  fetcher/         # HTTP client, timeout, user agent, response checks
  parser/          # Readability extraction, media discovery, DOM rewriting hooks
  downloader/      # Concurrent resource download, collision-safe filenames, progress events
  converter/       # HTML-to-Markdown conversion and frontmatter injection
main.go            # Program entrypoint
go.mod
go.sum
```

Recommended libraries:

- `github.com/spf13/cobra` for CLI behavior.
- `github.com/go-shiori/go-readability` for article extraction.
- `github.com/PuerkitoBio/goquery` for DOM traversal and rewriting.
- `github.com/JohannesKaufmann/html-to-markdown` for Markdown conversion.
- `github.com/schollz/progressbar/v3` or a small custom renderer for terminal progress.

The packages should communicate through plain structs so that parsing, downloading, and conversion can be tested independently.

## Error Handling

Use clear user-facing messages and non-zero exit codes for parameter errors, fetch failures, and strict-mode resource failures. Tolerant-mode resource failures are warnings. Extraction failure after a successful fetch is also a warning and still writes a traceable Markdown file.

Error messages should be concise and actionable, for example:

```text
错误：无法访问该网页，请检查网络连接或链接是否正确。
警告：连接成功但未提取到有效正文，已保存原始链接。
警告：文章已保存，但有 2 个资源下载失败，已在文中保留原链接。
```

## Testing Strategy

Automated tests should prefer local fixtures over live network dependencies.

Fixture coverage:

- Static article HTML with navigation, footer, article text, images, and videos.
- Existing `assets/` files to verify collision-safe naming.
- Failed resource responses to verify tolerant and strict modes.
- Extraction failure fixture that still produces frontmatter and `原文链接：<URL>`.
- CLI argument tests for missing `-n` and help output.

Live public URLs may be covered by optional smoke tests or scripts, but they should not be required for normal unit or CI-style test runs.

## Acceptance Criteria

- Running `web2md <URL> -n note` creates `note.md` and `assets/` in the current directory.
- Generated Markdown opens cleanly in Typora or Obsidian with readable article text and no obvious webpage clutter.
- Downloaded images and supported direct videos render from local `./assets/...` links while offline.
- Missing `-n` exits gracefully and displays help.
- Tolerant mode completes when individual resources fail and leaves remote URLs in Markdown.
- Strict mode returns a non-zero status on resource failure and preserves already written files.
