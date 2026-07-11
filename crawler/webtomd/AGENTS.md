# Repository Guidelines

## Project Structure & Module Organization

This project is a Go CLI (`web2md`) that fetches an article page and outputs Markdown plus local assets.

Current layout:

- `main.go`: CLI entrypoint.
- `cmd/`: Cobra command wiring and CLI tests.
- `pkg/`: core packages (`app`, `fetcher`, `parser`, `downloader`, `converter`, `progress`).
- `testdata/`: fixed HTML fixtures for parser and integration-style tests.
- `docs/`: PRD, design notes, implementation plan, and progress log.
- `scripts/`: helper scripts (for example, smoke tests).

Keep module boundaries clear: orchestration in `pkg/app`, implementation details in leaf packages.

## Build, Test, and Development Commands

Primary commands:

- `go test ./...`: run all tests.
- `go build -o web2md.exe .`: build Windows binary.
- `go run . <URL> -n <name>`: run locally.
- `go run . <URL> -n <name> --site-config examples/sites.example.json`: run with selector-based site rules.
- `powershell -ExecutionPolicy Bypass -File scripts/smoke-public-url.ps1`: optional public URL smoke check.
- `powershell -ExecutionPolicy Bypass -File scripts/smoke-sites.ps1`: optional real-site smoke checks written under `test-output/`.

On Windows in this environment, prefer `rtk powershell -Command "<command>"` for compact output.

## Coding Style & Naming Conventions

Use idiomatic Go:

- Run `gofmt` on changed Go files.
- Package names are lowercase and short.
- Exported identifiers use `PascalCase`; unexported use `camelCase`.
- Wrap errors with context at boundaries (`fmt.Errorf("...: %w", err)`).
- Keep CLI/user-facing Chinese error messages stable where tests assert them.

## Testing Guidelines

Use `go test` with package-local `*_test.go` files. Favor table-driven tests for parsing and URL edge cases. Keep fixtures deterministic under `testdata/`. For bug fixes, add a regression test in the closest affected package.

Real website smoke output belongs under `test-output/`; do not leave generated Markdown or `assets/` in the repository root.

## Commit & Pull Request Guidelines

Use concise, imperative commits scoped to one change, for example:

- `feat: connect cli to conversion pipeline`
- `fix: deduplicate resource downloads by resolved url`
- `test: add strict mode regression for missing assets`

PRs should include: summary, scope, commands run (`go test ./...`, build result), and sample output when CLI behavior changes.
