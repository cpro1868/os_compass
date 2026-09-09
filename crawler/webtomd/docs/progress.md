# web2md Progress

## 2026-04-17

### Completed

- Reviewed `docs/prd.md` and `docs/project.md`.
- Confirmed MVP decisions with the product owner:
  - CLI binary name is `web2md`.
  - Final user install should be a single executable with minimal system requirements.
  - URL validation stays lightweight; fetch failures produce friendly errors.
  - Existing Markdown output is overwritten.
  - Existing files in `assets/` are preserved and new downloads avoid name collisions.
  - Default tolerant mode keeps original remote URLs for failed resources.
  - `--strict` returns a non-zero status on resource failure and preserves existing output.
  - Empty article extraction writes frontmatter plus `原文链接：<URL>`.
  - Direct videos are discovered from `video[src]` and `video source[src]`.
  - MVP fetch behavior uses User-Agent, timeout, and redirects only; no browser rendering or login handling.
  - Download concurrency defaults to 5.
  - Tests should use local fixtures first, with public URL smoke tests optional.
- Created design spec: `docs/superpowers/specs/2026-04-17-web2md-design.md`.
- Created implementation plan: `docs/superpowers/plans/2026-04-17-web2md-mvp.md`.

### Current State

- Task 1 bootstrap files now exist: `go.mod`, `main.go`, `cmd/root.go`, and `cmd/root_test.go`.
- The workspace is not currently a Git repository, so commit steps were skipped and the design/plan docs remain uncommitted.
- Task 1 currently only covers the CLI shell and tests; later MVP implementation tasks are still pending.
- Verification remains blocked by a missing Go toolchain in this environment, so `go test` and `go version` cannot run successfully here.

### Task 2

- Implemented `pkg/fetcher` with timeout-aware HTTP GET support, a `web2md` User-Agent, non-2xx handling, and final URL/body capture.
- Added `pkg/fetcher/fetcher_test.go` covering User-Agent/body retrieval and 404 rejection.
- Verification attempt: `rtk powershell -Command "go test ./pkg/fetcher -v"`.
- Verification status: blocked because `go` is not recognized in this environment, so the package could not be compiled or tested here.
- Review fixes applied: the fetcher and tests now use exact UTF-8 `无法访问该网页`, the success test follows a redirect and checks the final URL, and the User-Agent capture uses a buffered channel to avoid a race.

### Task 3

- Added parser fixtures in `testdata/article.html` and `testdata/empty.html`.
- Added parser tests for readability extraction, clutter removal, media discovery, URL resolution, and empty shell handling.
- Implemented `pkg/parser` with go-readability extraction, goquery traversal, resource models, URL resolution, and direct-video filtering for `.mp4`, `.webm`, `.mov`, and `.m4v`.
- Updated `go.mod` manually with `github.com/PuerkitoBio/goquery` and `github.com/go-shiori/go-readability`.
- Did not run `go get`; `go.sum` cannot be generated in this environment because Go is unavailable in PATH.
- Verification attempt before implementation: `rtk powershell -Command "go test ./pkg/parser -v"`.
- Verification status: blocked because `go` is not recognized in this environment, so the red test could not be compiled or run.
- Verification attempt after implementation: `rtk powershell -Command "go test ./pkg/parser -v"`.
- Verification status: blocked because `go` is not recognized in this environment, so the parser package could not be compiled or tested here.

### Task 3 Review Fixes

- Strengthened parser resource assertions to compare the full expected resource slice.
- Added parser coverage for ignored empty, whitespace-only, `data:`, `blob:`, `javascript:`, `mailto:`, and fragment-only sources.
- Updated resource collection to skip empty or whitespace-only `src` values, skip fragment-only sources, and keep only resolved `http` or `https` resources.
- Verification attempt before review-fix implementation: `rtk powershell -Command "go test ./pkg/parser -v"`.
- Verification status: blocked because `go` is not recognized in this environment, so the red test could not be compiled or run.
- Verification attempt after review-fix implementation: `rtk powershell -Command "go test ./pkg/parser -v"`.
- Verification status: blocked because `go` is not recognized in this environment, so the parser package could not be compiled or tested here.

### Task 4

- Added `pkg/downloader/downloader_test.go` first, covering existing filename collision handling, tolerant failure replacement behavior, and strict failure error behavior.
- Verification attempt before implementation: `rtk powershell -Command "go test ./pkg/downloader -v"`.
- Verification status: blocked because `go` is not recognized in this environment, so the red test could not be compiled or run.
- Implemented `pkg/downloader` with resource/result/config/event types, default concurrency 5, a 30-second HTTP timeout, asset directory creation, worker-pool downloads, collision-safe filenames, `./assets/<filename>` replacements, event emission, tolerant mode, and strict mode partial-result preservation.
- Formatting attempt: `rtk powershell -Command "gofmt -w pkg\downloader\downloader.go pkg\downloader\downloader_test.go"`.
- Formatting status: blocked because `gofmt` is not recognized in this environment.
- Verification attempt after implementation: `rtk powershell -Command "go test ./pkg/downloader -v"`.
- Verification status: blocked because `go` is not recognized in this environment, so the downloader package could not be compiled or tested here.

### Task 4 Review Fixes

- Added downloader tests for serialized event emission with `Concurrency > 1`, late final-name collision retry without overwriting an externally created file, and cleanup of staged files after copy failure.
- Verification attempt before review-fix implementation: `rtk powershell -Command "go test ./pkg/downloader -v"`.
- Verification status: blocked because `go` is not recognized in this environment, so the red tests could not be compiled or run.
- Updated downloader event emission to serialize `EventSink` callbacks with a mutex.
- Updated downloader writes to stage responses in a temporary file, create final files with `os.O_CREATE|os.O_EXCL|os.O_WRONLY`, retry on `os.IsExist`, and remove temporary/final residue on failure paths.
- Added a concise comment documenting that `Result.Replacement` is always `./assets/<filename>` regardless of the absolute `assetDir`.
- Formatting attempt: `rtk powershell -Command "gofmt -w pkg\downloader\downloader.go pkg\downloader\downloader_test.go"`.
- Formatting status: blocked because `gofmt` is not recognized in this environment.
- Verification attempt after review-fix implementation: `rtk powershell -Command "go test ./pkg/downloader -v"`.
- Verification status: blocked because `go` is not recognized in this environment, so the downloader package could not be compiled or tested here.

### Task 5

- Added `pkg/converter/converter_test.go` first, covering frontmatter output, HTML heading conversion, local asset link preservation, and the no-content original-link fallback.
- Verification attempt before implementation: `rtk powershell -Command "go test ./pkg/converter -v"`.
- Verification status: blocked because `go` is not recognized in this environment, so the red tests could not be compiled or run.
- Implemented `pkg/converter` with `Document` and `Convert`, YAML frontmatter, `YYYY-MM-DD HH:mm:ss` fetch dates, html-to-markdown conversion for content, original-link fallback for empty content, and a trailing newline.
- Updated `go.mod` manually with `github.com/JohannesKaufmann/html-to-markdown v1.6.0`; did not run `go get`, and `go.sum` cannot be generated in this environment because Go is unavailable in PATH.
- Formatting attempt: `rtk powershell -Command "gofmt -w pkg\converter\converter.go pkg\converter\converter_test.go"`.
- Formatting status: blocked because `gofmt` is not recognized in this environment.
- Verification attempt after implementation: `rtk powershell -Command "go test ./pkg/converter -v"`.
- Verification status: blocked because `go` is not recognized in this environment, so the converter package could not be compiled or tested here.
- Replaced the fallback text literals in converter code/tests with Go Unicode escapes so the runtime output remains the required original-link fallback without depending on source-file console decoding.
- Final formatting attempt: `rtk powershell -Command "gofmt -w pkg\converter\converter.go pkg\converter\converter_test.go"`.
- Final formatting status: blocked because `gofmt` is not recognized in this environment.
- Final verification attempt: `rtk powershell -Command "go test ./pkg/converter -v"`.
- Final verification status: blocked because `go` is not recognized in this environment, so the converter package could not be compiled or tested here.

### Task 5 Review Fixes

- Added converter regression tests for YAML-quoted original URLs, image conversion, `HasContent: false` overriding non-empty HTML, and fallback behavior when content conversion trims to empty.
- Verification attempt before review-fix implementation: `rtk powershell -Command "go test ./pkg/converter -v"`.
- Verification status: blocked because `go` is not recognized in this environment, so the red tests could not be compiled or run.
- Updated converter behavior so `HasContent` is authoritative: false always writes the original-link fallback, and true converts HTML but falls back if the converted markdown is empty after trimming.
- Updated frontmatter generation to always double-quote `original_url` and escape backslashes, quotes, newlines, carriage returns, and tabs deterministically.
- Formatting attempt: `rtk powershell -Command "gofmt -w pkg\converter\converter.go pkg\converter\converter_test.go"`.
- Formatting status: blocked because `gofmt` is not recognized in this environment.
- Verification attempt after review-fix implementation: `rtk powershell -Command "go test ./pkg/converter -v"`.
- Verification status: blocked because `go` is not recognized in this environment, so the converter package could not be compiled or tested here.
- Final formatting attempt after tightening the empty-conversion regression: `rtk powershell -Command "gofmt -w pkg\converter\converter.go pkg\converter\converter_test.go"`.
- Final formatting status: blocked because `gofmt` is not recognized in this environment.
- Final verification attempt after tightening the empty-conversion regression: `rtk powershell -Command "go test ./pkg/converter -v"`.
- Final verification status: blocked because `go` is not recognized in this environment, so the converter package could not be compiled or tested here.

### Task 6

- Added `pkg/progress/progress_test.go` first, covering Docker pull style progress lines for `cover.png: Downloading` and `clip.webm: Complete`.
- Verification attempt before implementation: `rtk powershell -Command "go test ./pkg/progress ./pkg/downloader -v"`.
- Verification status: blocked because `go` is not recognized in this environment, so the red test could not be compiled or run.
- Implemented `pkg/progress` with `Event`, a writer-backed mutex-protected `Renderer`, `NewRenderer`, `Event`, and `EventName`.
- Confirmed downloader already exposes compatible `EventSink` with `EventName(name string, status string)`, so `pkg/downloader/downloader.go` was not changed.
- Formatting attempt: `rtk powershell -Command "gofmt -w pkg\progress\progress.go pkg\progress\progress_test.go"`.
- Formatting status: blocked because `gofmt` is not recognized in this environment.
- Final verification attempt: `rtk powershell -Command "go test ./pkg/progress ./pkg/downloader -v"`.
- Final verification status: blocked because `go` is not recognized in this environment, so the progress and downloader packages could not be compiled or tested here.

### Task 6 Review Fixes

- Added progress coverage for `NewRenderer(nil)` so a nil writer is expected to be safe.
- Added downloader coverage for failure progress events using the reserved local filename instead of the raw source URL.
- Verification attempt before review-fix implementation: `rtk powershell -Command "go test ./pkg/progress ./pkg/downloader -v"`.
- Verification status: blocked because `go` is not recognized in this environment, so the red tests could not be compiled or run.
- Updated `NewRenderer` to default nil writers to `io.Discard` and documented renderer writes as best-effort.
- Updated downloader event emission to use the reserved filename for `Downloading`, `Complete`, and `Failed` events.
- Formatting attempt: `rtk powershell -Command "gofmt -w pkg\progress\progress.go pkg\progress\progress_test.go pkg\downloader\downloader.go pkg\downloader\downloader_test.go"`.
- Formatting status: blocked because `gofmt` is not recognized in this environment.
- Final verification attempt: `rtk powershell -Command "go test ./pkg/progress ./pkg/downloader -v"`.
- Final verification status: blocked because `go` is not recognized in this environment, so the progress and downloader packages could not be compiled or tested here.

### Task 7

- Added `pkg/app/app_test.go` first, covering a full successful run that writes `note.md`, rewrites a downloaded image to `./assets/cover.png`, and stores the asset file.
- Added tolerant-mode app coverage for a missing image where the downloader fails but `Run` still writes markdown preserving the remote resource URL.
- Red-test verification attempt before implementation: `rtk powershell -Command "go test ./pkg/app -v"`.
- Red-test verification status: blocked because `go` is not recognized in this environment, so the new app tests could not be compiled or run before implementation.
- Implemented `pkg/app` with `Config` and `Run`, including work directory resolution, fetch, parse, asset download to `<WorkDir>/assets` with concurrency 5, replacement mapping for original and resolved resource URLs, markdown conversion, and `<WorkDir>/<Name>.md` writing.
- Added `parser.RewriteResources` to update `src` attributes on `img[src]`, `video[src]`, and `video source[src]` using a replacement map.
- Formatting attempt: `rtk powershell -Command "gofmt -w pkg\app\app.go pkg\app\app_test.go pkg\parser\parser.go"`.
- Formatting status: blocked because `gofmt` is not recognized in this environment.
- Final verification attempt: `rtk powershell -Command "go test ./pkg/app -v"`.
- Final verification status: blocked because `go` is not recognized in this environment, so the app package could not be compiled or tested here.
- Parser verification attempt after adding `RewriteResources`: `rtk powershell -Command "go test ./pkg/parser -v"`.
- Parser verification status: blocked because `go` is not recognized in this environment, so the parser package could not be compiled or tested here.

### Task 7 Review Fixes

- Added `Run` config validation for empty URL/name and unsafe name values containing path traversal/path-separator semantics.
- Updated `Run` resource handling to deduplicate downloads by resolved URL before calling downloader, then map all parser resources to the deduplicated result replacements.
- Added app-level regression tests for empty `Name`, empty `URL`, strict-mode missing resource error with no markdown output, and duplicate resource deduplication behavior.
- Updated `parser.RewriteResources` to fall back to full document HTML when `body` rendering is empty, improving fragment-input robustness.
- Added parser coverage for `RewriteResources` on fragment input without requiring an explicit `<body>` wrapper.
- Verification remains blocked in this environment because `go`/`gofmt` are unavailable in PATH, so these review-fix changes could not be compiled or executed locally.

### Task 8

- Updated `cmd/root.go` to call application orchestration through a runner seam:
  - Added `Runner` type and `NewRootCommandWithRunner`.
  - `NewRootCommand` now delegates to `app.Run` with `URL`, `Name`, and `Strict`.
- Updated `cmd/root_test.go`:
  - Replaced planned-output assertion with `TestRootRunsWithName`, verifying runner invocation and option/url handoff.
  - Kept existing missing-name and help tests.
- Added optional smoke script: `scripts/smoke-public-url.ps1`.
- Verification attempt: `rtk powershell -Command "go test ./cmd -v"`.
- Verification status: blocked because `go` is not recognized in this environment.

### Task 9

- Added `README.md` with CLI usage, strict-mode behavior, and development commands.
- Updated `AGENTS.md` to match actual Go project structure and canonical Go commands:
  - `go test ./...`
  - `go build -o web2md.exe .`
  - `go run . <URL> -n <name>`
- Final verification attempts:
  - `rtk powershell -Command "go test ./..."`
  - `rtk powershell -Command "go build -o web2md.exe ."`
- Verification status: blocked because `go` is not recognized in this environment, so full compile/test completion is pending local Go toolchain availability.

## 2026-04-20

### Environment Recovery and Final Verification

- Go toolchain became available; resumed final validation.
- Initial `go test ./...` failed because `go.sum` was missing.
- Ran `go mod tidy` successfully and generated `go.sum`.
- During validation, updated parser expectation in `pkg/parser/parser_test.go`:
  - `TestParseExtractsArticleMedia` now expects `OriginalURL` values as absolute URLs, matching readability-normalized output.
- Final verification commands (passed):
  - `go test ./...`
  - `go build -o web2md.exe .`
- Verification artifacts now present in workspace:
  - `go.sum`
  - `web2md.exe`

### Captcha/Verification Handling Fix

- Root cause identified from real URL reproduction:
  - The target WeChat URL redirects to `/mp/wappoc_appmsgcaptcha...`.
  - Previous behavior treated the verification page as normal article content, producing markdown body like `环境异常`.
- Added a failing regression test:
  - `pkg/app/app_test.go`: `TestRunRejectsCaptchaVerificationPage`.
- Implemented fix in orchestration layer:
  - `pkg/app/app.go`: detect verification/captcha pages by URL marker and verification-page content markers.
  - On detection, `Run` now returns a clear error and aborts before markdown write.
- Verification (passed):
  - `go test ./pkg/app -run TestRunRejectsCaptchaVerificationPage -v`
  - `go test ./...`
  - `go build -o web2md.exe .`
- Manual CLI reproduction with provided full WeChat URL now returns:
  - `Error: 目标站点触发验证码或环境校验，当前版本不支持自动通过，请在浏览器完成验证后重试`

### WeChat Fetch Compatibility Update

- User requirement: must support fetching WeChat public-account article links directly.
- Root-cause evidence:
  - Fetcher used bot-like UA (`web2md fetcher`), which increased risk of WeChat captcha/verification redirection.
- TDD updates:
  - Updated `pkg/fetcher/fetcher_test.go` to require browser-like UA and reject bot marker.
  - Added `pkg/app/app_test.go` case `TestRunBypassesWechatStyleCaptchaWhenBrowserUA` simulating WeChat-style captcha redirect for bot UA.
  - Confirmed both tests failed before implementation.
- Implementation:
  - `pkg/fetcher/fetcher.go` now sets browser-like headers:
    - `User-Agent` (modern browser UA)
    - `Accept`
    - `Accept-Language`
    - `Referer` derived from target origin
  - Added `buildReferer` helper for stable referer construction.
- Verification (passed):
  - `go test ./pkg/fetcher ./pkg/app -run "TestFetchSendsUserAgentAndReturnsBody|TestRunBypassesWechatStyleCaptchaWhenBrowserUA|TestRunRejectsCaptchaVerificationPage" -v`
  - `go test ./...`
  - `go build -o web2md.exe .`
- Real-link validation:
  - `web2md.exe "https://mp.weixin.qq.com/s/Y_uRMYBmdLWUPnz_ac7jWA" -n wechat-real`
  - Successfully generated `wechat-real.md` with extracted article content and local `assets/` references.

### WeChat Body Completeness Fix

- User-reported issue: exported markdown only contained a small fragment instead of full article body.
- Root cause:
  - Generic readability extraction can truncate WeChat pages and keep only a partial section.
  - WeChat article images often use `data-src`; relying only on `img[src]` can lose real resource URLs.
- TDD updates:
  - Added parser regression test `TestParseWeChatUsesJSContentAndDataSrc` in `pkg/parser/parser_test.go`.
  - Confirmed test failed before implementation.
- Implementation in `pkg/parser/parser.go`:
  - For `mp.weixin.qq.com`, parser now prefers `#js_content` as authoritative article body.
  - Normalizes `img[data-src]` by copying to `src` before resource collection and downstream rewrite/markdown conversion.
  - Uses `#activity-name` as title (fallback to `<title>`).
  - Keeps non-WeChat pages on existing readability path.
- Stability improvement:
  - Increased default fetch timeout from `30s` to `90s` in `pkg/fetcher/fetcher.go` to reduce WeChat timeout failures.
- Verification (passed):
  - `go test ./pkg/parser -v`
  - `go test ./...`
  - `go build -o web2md.exe .`
- Real link run: `web2md.exe "https://mp.weixin.qq.com/s/Y_uRMYBmdLWUPnz_ac7jWA" -n wechat-real-v3`
  - Output now contains full正文 content (not only short sliding-hint fragment).

### Markdown Title and Quote Formatting Fix

- User-reported issues:
  - Markdown body missed a top-level title heading.
  - Quote lines had malformed prefix formatting (e.g. `>/ 作者...`).
- TDD updates:
  - Updated converter tests to require:
    - title heading generation from parsed title
    - normalized quote prefix spacing (`> / ...`)
  - Updated app integration test to assert heading output.
- Implementation:
  - `pkg/converter/converter.go`
    - Added `Document.Title`.
    - Prepends `# <Title>` before body when title exists and is not already the first heading.
    - Added markdown normalization for quote prefix spacing (`>/` -> `> /`).
  - `pkg/app/app.go`
    - Passes `parsed.Title` to converter.
- Verification (passed):
  - `go test ./pkg/app ./pkg/converter -v`
  - `go test ./...`
  - `go build -o web2md.exe .`
- Real link run:
    - `web2md.exe "https://mp.weixin.qq.com/s/Y_uRMYBmdLWUPnz_ac7jWA" -n wechat-real-v4`
    - Output confirmed:
      - heading appears as `# 分享一个我用了2年的深度研究Prompt，半小时帮你搞懂任何陌生领域。`
      - quote lines normalized as `> / 作者...` and `> / 投稿或爆料...`.

### Metadata Presentation Adjustment

- User requested UI-level markdown layout change:
  - remove top YAML frontmatter block (`original_url` / `fetch_date`)
  - place metadata under title using quote style.
- Implementation:
  - `pkg/converter/converter.go`
    - removed frontmatter output.
    - added quote metadata block:
      - `> 原文链接：...`
      - `> 抓取时间：...`
    - preserved title-first layout: `# 标题` then metadata quote then body.
  - `pkg/app/app.go`
    - unchanged flow, continues passing parsed title into converter.
- Tests updated and passing:
  - `pkg/converter/converter_test.go` expectations switched from frontmatter to quote metadata.
  - `pkg/app/app_test.go` assertions switched accordingly.
  - verification passed:
    - `go test ./pkg/app ./pkg/converter -v`
    - `go test ./...`
    - `go build -o web2md.exe .`
- Real-link sample output (`wechat-real-v5.md`) now starts with:
  - `# ...`
  - `> 原文链接：...`
  - `> 抓取时间：...`

### Quote/Code Block Readability Adjustment

- User-reported issue: quote/code blocks were visually mixed and lacked line breaks.
- Root cause:
  - Some WeChat rich-text sections flatten into dense single-line markdown inside fenced code.
- Implementation (`pkg/converter/converter.go`):
  - Extended `normalizeMarkdown` with fenced-code-aware formatting:
    - only applies dense-line cleanup while inside code fences
    - inserts line breaks before common markdown structures (`##`, `###`, numbered list items, bullet markers)
    - separates dense `---` delimiters and inline fence markers
  - Kept blockquote prefix normalization (`>/` -> `> /`).
- Added regression test:
  - `TestNormalizeMarkdownPrettifiesDenseCodeBlock` in `pkg/converter/converter_test.go`.
- Verification passed:
  - `go test ./pkg/converter -v`
  - `go test ./...`
  - `go build -o web2md.exe .`
  - Real-link run: `web2md.exe "https://mp.weixin.qq.com/s/Y_uRMYBmdLWUPnz_ac7jWA" -n wechat-real-v7`.

## 2026-04-21

### Chinese Documentation Update

- Added user-facing Chinese documentation:
  - `README_CN.md`
    - overview, quick start, binary install, source build, usage, WeChat notes, output rules, development verification.
  - `docs/deploy_CN.md`
    - environment requirements, local build, cross-platform build, PATH installation, usage examples, distribution advice, troubleshooting.
- Updated `README.md` with links to Chinese docs.
- Updated existing Chinese project docs to match current output format:
  - `docs/prd.md`
  - `docs/project.md`
- Replaced outdated YAML Frontmatter wording with the current title + quoted metadata format:
  - `# 标题`
  - `> 原文链接：...`
  - `> 抓取时间：...`

### Asset Extension Inference Fix

- User-reported issue: files under `assets/` had no file extensions, especially for WeChat image URLs like `/.../640?wx_fmt=jpeg`.
- Root cause:
  - Downloader used only the URL path basename for filenames.
  - Many WeChat image URLs have no path extension; actual format is stored in query parameters or response `Content-Type`.
- Added regression tests in `pkg/downloader/downloader_test.go`:
  - `TestDownloadAllAddsExtensionFromWechatURLFormat`
  - `TestDownloadAllAddsExtensionFromContentTypeWhenURLHasNoExtension`
- Implementation in `pkg/downloader/downloader.go`:
  - infers extension for extensionless filenames from `wx_fmt`, `format`, or `fmt` query values.
  - falls back to `Content-Type` mapping for common images/videos.
  - keeps collision handling and reserved filename behavior intact.
- Documentation updated:
  - `README_CN.md`
  - `docs/deploy_CN.md`
- Verification passed:
  - `go test ./pkg/downloader -v`
  - `go test ./...`
  - `go build -o web2md.exe .`
  - Real WeChat sample in a clean output directory produced assets such as `640.png`, `640_1.png`, etc.

### Test Output Directory and Site Profile Extensibility

- User requested automatic test output to be isolated from project Markdown files and asked for a scalable way to support more sites.
- Implemented parser extension structure:
  - added `pkg/parser/profiles.go` with `SiteProfile` and profile dispatch.
  - moved WeChat-specific parsing into `pkg/parser/profile_wechat.go`.
  - `parser.Parse` now tries site profiles first, then falls back to Readability.
- Added/updated smoke scripts:
  - `scripts/smoke-public-url.ps1` now writes output under `test-output/smoke/public` by default.
  - `scripts/smoke-sites.ps1` runs multiple real-site checks and writes under `test-output/sites/<site>/`.
- Added `.gitignore` entries for local generated output and caches:
  - `test-output/`, `tmp-extension-check/`, caches, generated assets, sample markdown, and binary artifacts.
- Added extension documentation:
  - `docs/site_profiles_CN.md`
  - updated `README_CN.md`, `docs/deploy_CN.md`, and `AGENTS.md`.

### Configurable Site Rules

- User requested a CLI parameter that references an extension definition file.
- Added `--site-config <path>` to CLI.
- Added `pkg/siteconfig`:
  - loads JSON config files.
  - validates `version`, `sites`, `hosts`, and `content` selectors.
- Added config-driven parser profiles:
  - `pkg/parser/profile_config.go`
  - custom config profiles run before built-in profiles and before Readability fallback.
- Added example rules:
  - `examples/sites.example.json`
- Added tests:
  - config load/validation tests in `pkg/siteconfig`.
  - config profile parser tests in `pkg/parser`.
  - app integration test for `SiteConfigPath`.
  - CLI option handoff and help-output tests for `--site-config`.
- Documentation updated:
  - `README_CN.md`
  - `docs/site_profiles_CN.md`
  - `docs/deploy_CN.md`
  - `AGENTS.md`
- Verification passed:
  - `go test ./cmd -v`
  - `go test ./...`
  - `go build -o web2md.exe .`
  - manual binary run with `--site-config examples/sites.example.json` wrote output under `test-output/manual-site-config/`.

### Site Config Reference Documentation

- Added dedicated documentation for `examples/sites.example.json`:
  - English: `docs/site_config.md`
  - Chinese: `docs/site_config_CN.md`
- Both documents cover:
  - `--site-config` usage
  - resolution order
  - file shape
  - every supported field
  - selector rules
  - lazy-loaded image handling
  - common patterns
  - debugging tips
- Updated links in:
  - `README.md`
  - `README_CN.md`
  - `docs/site_profiles_CN.md`

### Notion Public Page Support

- Investigated sample URL `https://iyouport.notion.site/S07E05-24c34ca2d46d808985a0f63a22dde6c7`.
- Root cause: the fetched HTML is a Notion app shell and does not contain article body content, so selector-based extraction cannot work for this case.
- Implemented built-in Notion profile:
  - extracts the 32-character Notion page ID from URL or HTML metadata.
  - calls Notion public `loadPageChunk` data endpoint for public pages.
  - renders common block types into HTML for the existing Markdown converter.
  - signs `attachment:` image/video/file URLs through Notion's signed file endpoint before resource collection.
- Added parser tests for:
  - page ID extraction.
  - raw Notion property decoding.
  - block rendering, links, quotes, code, and signed attachment resources.
  - avoiding false positives on plain Notion marketing pages.
- Documentation updated:
  - `README_CN.md`
  - `docs/site_profiles_CN.md`
- Manual verification:
  - rebuilt `web2md.exe`.
  - sample Notion URL exported `test-output/notion/notion-s07e05.md`.
  - title, body text, quotes, code blocks, and local image assets were present.

### NYTimes Image Download Reliability

- Investigated sample URL `https://cn.nytimes.com/health/20260422/cabbage-health-benefits-recipes/`.
- Root cause: the parser found the article images, but one image download could time out in tolerant mode; failed resources intentionally kept their original URL in Markdown.
- Downloader improvements:
  - resource requests now use a browser-like User-Agent and image/video Accept headers.
  - default resource download timeout increased to 90 seconds.
  - transient network/read failures, HTTP 429, and HTTP 5xx are retried up to three attempts.
- Added downloader regression tests for:
  - browser-like User-Agent on asset requests.
  - retrying transient server failure.
- Manual verification:
  - rebuilt `web2md.exe`.
  - the NYTimes sample passed with `--strict`.
  - all five article images were saved under `test-output/nytimes-fixed/assets/` and Markdown links were rewritten to `./assets/...jpg`.

### Completeness Fixes for Notion and NYTimes CN

- User reported incomplete Notion output and missing NYTimes CN paragraphs/subheadings.
- Notion root cause:
  - `loadPageChunk` returns long pages in multiple cursor-based chunks.
  - the previous implementation loaded only chunk 0.
- Notion fix:
  - added cursor-following chunk loading and record-map merging.
  - added regression test for multi-chunk loading.
  - real sample output grew from 197 lines to 870 lines and reaches the final closing section.
- NYTimes CN root cause:
  - generic Readability reduced the sample article from 39 `.article-paragraph` nodes to 28 paragraphs.
  - bold-only subhead paragraphs were not preserved as Markdown headings.
- NYTimes CN fix:
  - added built-in `cn.nytimes.com` profile using `.article-content .article-body`.
  - preserves all `.article-paragraph` nodes, converts short bold-only paragraphs to `h2`, recipe-number paragraphs to `h3`, and keeps `data-src` images downloadable.
  - added regression test for paragraphs, subheads, recipe headings, and image extraction.
- Verification:
  - real Notion sample passed with `--strict` and exported 870 lines.
  - real NYTimes CN sample passed with `--strict`, preserved health subheads and five recipe headings, and downloaded five images.
  - `go test ./...`
  - `go build -o web2md.exe .`

### Markdown Table Conversion

- User reported that some pages with HTML tables, including WeChat article `https://mp.weixin.qq.com/s/CJr4Oo_GDBD8ejJ57l6khQ`, exported table cells as flattened plain text instead of Markdown tables.
- Root cause:
  - the WeChat parser kept the article HTML correctly.
  - `github.com/JohannesKaufmann/html-to-markdown` supports table conversion via `plugin.Table()`, but the converter was not enabling that plugin.
- Fix:
  - enabled `plugin.Table()` in `pkg/converter/converter.go`.
  - added converter regression test for HTML table to Markdown table conversion.
- Real verification:
  - rebuilt `web2md.exe`.
  - reran the WeChat sample.
  - output now contains Markdown table rows such as `| 维度 | 传统 RAG | LLM Wiki |` instead of flattened text.
- Verification:
  - `go test ./...`
  - `go build -o web2md.exe .`

### WeChat Image Articles and Weibo Anti-Crawler Handling

- User reported:
  - WeChat image article `https://mp.weixin.qq.com/s/Jq_1sn9GhkHljPM5bhdg2g` exported only the cover and "向上滑动看下一个".
  - Weibo long article `https://weibo.com/ttarticle/x/m/show/id/2309405303156245659656` failed to fetch.
  - Some pages return "你暂无权限查看此页面内容".
- WeChat root cause:
  - this sample is `item_show_type=8`, a swipe/image article.
  - the traditional `#js_content` article body is not the full content; useful text and images are stored in script fields such as `content_noencode` and `picture_page_info_list`.
- WeChat fix:
  - `weChatProfile` now detects image articles before requiring `#js_content`.
  - it extracts text from script content and image URLs from `picture_page_info_list`.
  - it still uses the existing `#js_content` path for normal articles.
- Weibo/permission handling:
  - added a static Weibo article profile for pages that do return article HTML.
  - fetcher now uses a cookie jar, fuller browser-like headers, gzip handling, short retries, and mobile UA/candidate URL fallback for Weibo article URLs.
  - added `--cookie` so pages that are visible in a user's browser session can reuse copied site cookies for the current command.
  - app-level blocking detection now recognizes Sina Visitor System and "暂无权限查看" style pages and returns a clear error instead of writing misleading Markdown.
- Real verification:
  - the WeChat sample now exports the description text and 7 local image links.
  - the Weibo sample currently returns Sina Visitor/permission gating without public article HTML, so the CLI reports a clear anti-crawler/permission error and does not write output.

### Weibo Empty Article Fallback Guard

- User reported Weibo article `https://weibo.com/ttarticle/x/m/show/id/2309405303156245659656` could export as only a simple original-link fallback.
- Root cause:
  - when a Weibo article-like URL returned an app shell without extractable article HTML, the Weibo profile did not claim the page.
  - parsing then fell through to the generic empty-content fallback.
- Fix:
  - Weibo article-like URLs (`ttarticle`, `/article/`, `/status/`) now return an explicit error if no article body is present.
  - the error points to `--cookie` for browser-session pages or reports permission/anti-crawler restriction.
  - added regression coverage so empty Weibo shells no longer generate fallback-only Markdown.

### Weibo Browser Session Fallback

- User reported that Weibo pages visible in a normal browser still failed in the CLI and that requiring manual Cookie copying is unreasonable.
- Changes:
  - added Weibo long-text API fallback with automatic Sina Visitor bootstrap before browser rendering.
  - added Chrome/Edge DevTools rendering fallback for Weibo and WeChat blocked pages.
  - browser fallback now copies `Local State` and Cookie files from a Chrome/Edge Profile into a temporary Profile before rendering, so the original browser data is not modified.
  - added `--browser-profile` and environment support (`WEB2MD_BROWSER_PROFILE_DIR`, `WEB2MD_BROWSER_USER_DATA_DIR`) for explicit Profile selection.
  - kept `--cookie` as a last-resort manual option.
- Verification:
  - `go test ./...`
  - `go build -o web2md.exe .`
  - WeChat sample `https://mp.weixin.qq.com/s/iVHL-4Eh7IXgdB_7BfJPsQ` exports normally in this environment.
  - Weibo sample still returns a clear anti-crawler error on this machine because no reusable Weibo browser session is available and `passport.weibo.com` TLS fails locally; with a valid local browser Profile, the fallback now has a no-manual-cookie path.

### WeChat Script HTML Markdown Regression

- User reported that `https://mp.weixin.qq.com/s/w2mCh8LRe14HL3KFbyBbrQ` exported large raw HTML blocks, and that adding `--browser-profile` did not produce Markdown.
- Root cause:
  - the page was routed through the WeChat image/script article path.
  - `content_noencode` contained full styled HTML, but the parser treated it as plain text, so the Markdown converter later emitted raw HTML.
- Fix:
  - WeChat script articles now detect HTML fragments in `content_noencode`.
  - HTML script content is parsed and cleaned before conversion, removing inline styles, `data-*` attributes, hidden style markers, and preserving only safe link/media/table attributes.
  - normal text-only image articles still use the old paragraph path.
- Verification:
  - added parser regression coverage for styled WeChat HTML in `content_noencode`.
  - `go test ./...`
  - `go build -o web2md.exe .`
  - real-link runs with and without `--browser-profile` now output headings and paragraphs as Markdown, without raw `<h3 ...>`/`style=` blocks.
