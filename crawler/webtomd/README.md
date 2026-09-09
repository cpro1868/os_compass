# web2md

`web2md` converts a web article into an offline Markdown note and downloads media assets to a local `assets/` directory.

中文文档：see [README_CN.md](README_CN.md). Installation and deployment details: [docs/deploy_CN.md](docs/deploy_CN.md).

Site configuration reference: [docs/site_config.md](docs/site_config.md). 中文站点配置说明：[docs/site_config_CN.md](docs/site_config_CN.md).

## Usage

```bash
web2md <URL> -n <document-name>
web2md <URL> -n <document-name> --strict
web2md <URL> -n <document-name> --browser-profile "%LOCALAPPDATA%\\Google\\Chrome\\User Data\\Default"
```

Output is written to the current directory:

- `<document-name>.md`
- `assets/`

Default mode keeps original remote URLs when media download fails. `--strict` exits non-zero when any media download fails and preserves existing files.

For pages that are visible in your browser but blocked by direct HTTP fetching, use `--browser-profile` to render with a copied Chrome/Edge profile session. The original browser profile is not modified.

## Development

```bash
go test ./...
go build -o web2md.exe .
go run . <URL> -n <name>
```
