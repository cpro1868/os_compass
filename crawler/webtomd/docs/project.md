# web2md 技术实现文档
## 1. 产品定位
一个零依赖、跨平台的命令行工具，用于将网页文章高质量地转化为适配 Obsidian / Typora 的本地离线 Markdown 笔记，并自动将相关媒体资源本地化。

## 2. 核心 CLI 设计
工具严格遵循标准 POSIX 命令行规范。借助 Go 的 `Cobra` 库，我们将自动获得完美的 `--help` 提示。

- **基础命令**：`web2md [URL] -n [文件名]`
- **必填参数**：
    - `[URL]`：目标网页地址。
    - `-n, --name`：输出的 Markdown 文件名（不含后缀）。**如果缺失，程序打印友好错误并退出，同时显示 Help 信息。**
        
- **选填参数**：
    - `--strict`：开启严格模式。默认不开启（宽容模式）。宽容模式下，遇到个别图片/视频下载失败时仅在终端输出 Warning，继续生成 Markdown；严格模式下，任何下载失败直接报错回滚。
    - `-h, --help`：打印详细的命令帮助、参数说明和使用示例。
        

## 3. 核心业务规则
- **存储结构**：统一在执行命令的当前目录下生成 `[name].md` 文件和 `/assets` 文件夹。
- **媒体处理**：
    - 仅抓取正文中的图片 (`<img>`) 和明确后辍的直链视频 (`<video src="*.mp4">`)。
    - **重名处理**：保存到 `/assets` 前检查同名文件。若存在 `image.png`，则自动重命名为 `image_1.png`、`image_2.png`，保留原文件后缀以确保编辑器兼容性。
    - **路径替换**：将 HTML 转 MD 时，把原绝对路径替换为相对路径 `./assets/xxx.png`。
        
- **并发反馈**：控制下载并发数（建议 3-5 个），终端需提供清晰的下载进度条和当前正在处理的文件名。
- **元数据展示**：生成的 Markdown 正文以文章标题开头，并在标题下方用引用块记录来源信息：

```Markdown
# 文章标题

> 原文链接：https://example.com/article
> 抓取时间：2026-04-21 10:00:00
```
    

---

### 🛠️ Go 技术架构与核心选型

为了最高效地实现上述 PRD，推荐以下 Go 语言生态中最成熟的开源库组合：
1. **CLI 框架**: `github.com/spf13/cobra` (提供路由、参数解析和绝佳的自动 `--help` 生成)。
2. **正文提取**: `github.com/go-shiori/go-readability` (Go 版本的 Mozilla Readability，提取干净的正文 HTML)。
3. **DOM 操作**: `github.com/PuerkitoBio/goquery` (类似 jQuery，用于遍历提取 `img/video` 标签，并修改 `src` 属性)。
4. **Markdown 转换**: `github.com/JohannesKaufmann/html-to-markdown` (极其强大的 HTML 转 MD 工具)。
5. **进度条**: `github.com/schollz/progressbar/v3` (实现并发下载时的终端 UI 反馈)。

## 4、项目架构
```Plaintext 
	web2md/
	├── cmd/
	│   └── root.go          # Cobra 命令行入口、参数解析、Help 提示
	├── pkg/
	│   ├── fetcher/         # 负责发起 HTTP 请求、绕过基础反爬
	│   ├── parser/          # 负责Readability 解析、GoQuery 提取图片和修改 DOM
	│   ├── downloader/      # 负责并发下载资源、处理重名逻辑 (_1, _2)
	│   └── converter/       # 负责 HTML 转 Markdown、整理标题和来源信息
	├── main.go              # 程序主入口
	├── go.mod
	└── go.sum
```
