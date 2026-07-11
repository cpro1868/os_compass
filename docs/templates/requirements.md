# Video-Agent-Skill 需求规格说明书

版本：v1.0
日期：2026-06-05
关联文档：`PRD v1.1`、`TDD v1.0`、`docs/验收测试计划.md`、`docs/实施前检查清单.md`

## 1. 文档目的

本文档在《Agent 视频内容摘要引擎 PRD》及《系统技术架构说明书》的基础上，将项目需求固化为可验收的软件需求规格。后续的代码开发、任务拆分、测试验收以及 Agent 框架集成均以本文档为需求基线。

## 2. 项目背景

现有的大语言模型 Agent 系统主要以文本交互为核心，当用户或工作流传入视频流媒体链接（如 B站、YouTube）时，Agent 无法直接读取和理解视频内容，导致信息链路断裂。

新系统需要建设一个“无头（Headless）”的 CLI 组件，通过智能降级策略（优先提取字幕，兜底本地语音识别），为 Agent 提供视频内容的解析、转录与摘要提炼服务。同时，必须解决第三方音视频底层库输出日志污染 Agent 数据解析的痛点。

## 3. 目标与范围

### 3.1 项目目标

* 构建一个高内聚、零 GUI 的 Python 命令行视频解析工程。
* 实现**智能降级提取机制**：首选提取零成本平台字幕；次选下载低码率音频并使用本地大模型转写。
* 实现**绝对的 I/O 隔离**：屏蔽底层库所有的运行日志与进度条，确保标准输出（stdout）仅为纯净 JSON。
* 彻底解决长音频语音识别导致的 GPU 显存溢出（OOM）问题。
* 支持通过 `uv` 极速运行与 `Docker` 容器化标准交付。

### 3.2 MVP 范围

首批实现并验证以下核心链路以证明架构可行性：

* **提取探针**：基于 `yt-dlp` 优先保障 B站 和 YouTube 的元数据与媒体下载，同时允许尝试解析其他 `yt-dlp` 支持的平台。
* **语音计算**：集成 `SenseVoiceSmall` 模型完成本地离线中英文 ASR 转写。
* **摘要生成**：对接本地 Ollama (Qwen2.5) 或 OpenAI 兼容 API 生成结构化数据。

### 3.3 最终范围

全面支持主流视频平台的链接解析；在极端网络与鉴权环境下，能够精准熔断并向 Agent 返回规范的错误说明，而非抛出代码级异常。

## 4. 用户角色

* **Agent 开发者**：将其作为 Tool/Skill 集成到 Dify、FastGPT、Claude Code 等编排框架中。
* **极客/终端用户**：通过 `pipx` 或 `uv` 在本地终端直接调用，快速获取长视频摘要。
* **系统维护者**：配置代理路由表、部署 Docker 镜像、调整底层大模型参数。

## 5. 功能需求

### 5.1 CLI 交互与参数

系统必须通过命令行参数接收外部指令，正式包入口为 `src/video_agent_skill/cli.py`，发布后的控制台命令固定为 `video-agent`。可保留根目录 `main.py` 作为开发兼容入口。
必须支持的参数：

* `-u, --url` [必填]：目标视频流媒体的 URL。
* `-l, --lang` [可选]：优先嗅探的字幕语言（如 `zh`, `en`），默认 `zh`。
* `--proxy` [可选]：强制单次任务使用的代理地址。
* `--keep-temp` [可选]：保留临时文件（用于调试），默认关闭。

### 5.2 配置加载与智能路由

系统必须从 `config.yaml` 中读取静态配置。

`config.yaml` 必须包含但不限于以下字段：

* `system.temp_dir`：自定义缓存目录。
* `system.auto_cleanup`：布尔值，清理沙盒。
* `network.rules`：列表形式的域名代理映射（如匹配 `youtube.com` 走特定的 socks5 代理，匹配 `bilibili.com` 走直连）。
* `ai.asr.device`：指定计算设备 (`auto`/`cuda`/`mps`/`cpu`)。
* `ai.asr.source_dir`：本地 SenseVoice 源码目录，可用于指定包含 `model.py` 的检出路径，例如 `G:/Projects/Sources/SenseVoice`。
* `ai.llm.api_base` / `model_name`：大模型接口与模型名称。
* `ai.llm.system_prompt` / `user_prompt_template`：最终视频信息提取 prompt，可为空使用内置默认值。仓库与安装包必须提供默认 prompt 文件 `default-system.txt` 与 `default-video-summary.txt`，用于将已转写文本规整提取为最终 JSON。CLI 必须提供 `--prompt-info` 查看安装包内 prompt 资源，并提供 `--init-prompts` 将默认 prompt 复制到本地目录供用户修改。`user_prompt_template` 支持 `{output_language}`、`{language_instruction}`、`{transcript}`。

### 5.3 媒体嗅探与降级提取

系统必须封装 `yt-dlp`，禁止使用 `subprocess` 调用其二进制文件，必须调用 Python API。

**降级策略规则**：

1. 注入 `skip_download=True` 获取 `info_dict`。
2. 若存在符合 `--lang` 要求的 UP主精校字幕或机器字幕，修改配置为静默下载字幕 (`.vtt`)。
3. 若无字幕，修改配置为 `format=bestaudio`，静默下载最低码率的音频至临时目录。

### 5.4 音频切片与本地 ASR

系统必须对下载的音频进行二次处理。

* **防 OOM 机制**：必须使用 `pydub` 等音频库，根据静音片段（VAD）或最高不超过 30 秒的时长限制，将音频进行切片处理。
* **算力推理**：必须使用 `modelscope` 加载 `SenseVoiceSmall` 模型，并且显式启用 `fp16` 半精度推理以降低显存占用。
* **设备策略**：当 `ai.asr.device=auto` 时，按 `cuda -> mps -> cpu` 探测并选择首个可用设备；当显式指定设备时，只使用该设备，不可用则返回标准错误 JSON。
* **源码路径策略**：SenseVoice 源码目录读取优先级为 CLI 参数 `--sensevoice-source-dir` > 环境变量 `VIDEO_AGENT_SENSEVOICE_SOURCE_DIR` > `config.yaml` 中的 `ai.asr.source_dir` > 空值。若本地源码不可用，系统必须回退到 FunASR/模型仓库默认实现或返回标准 ASR 错误，禁止抛出原生导入异常。
* **文本清洗**：必须使用正则表达式过滤识别结果中附带的情感或语种标签（如 `<|zh|>`）。

### 5.5 大模型摘要提取

系统必须提供调用大模型的摘要组件。

* 必须启用 `response_format={ "type": "json_object" }` 强制 JSON 输出。
* Prompt 必须包含指令，要求提炼：一段 `< 200 字` 的 summary，以及 `3-5 条` key_points。
* LLM 配置读取优先级为：CLI 参数 > 环境变量 > `config.yaml` > 内置默认值。最终信息提取 prompt 也遵循相同优先级，并支持通过 UTF-8 prompt 文件传入。默认不在本地服务不可用时自动切换到云端 API，避免未经确认的数据外发或费用产生。

### 5.6 数据 I/O 隔离与契约输出

系统在被唤起时，必须接管全局标准流。

**隔离规则**：

* 启动时执行 `sys.stdout = sys.stderr`。
* `yt-dlp`、`FFmpeg` 和 `ModelScope` 产生的所有调试、下载进度信息只允许流向 `stderr`。

**契约输出**：

* 业务处理完成后，必须绕过重定向（如向 `sys.__stdout__` 写入），输出**且仅输出**一段标准 JSON：

```json
{
  "status": "success/error",
  "meta": {"url": "...", "strategy_used": "...", "language": "..."},
  "content": {"summary": "...", "key_points": [], "tags": []},
  "error": {"code": "...", "message": "..."} // 仅 status=error 时存在
}

```

### 5.7 异常熔断机制

在任务失败时，系统必须优雅退出（Exit Code 1），并在 stdout 输出符合上述契约的错误 JSON，绝对禁止将原生 Python Traceback 抛给 Agent。

必须捕获并分类：

* **网络或防盗链异常**（如 B站需要大会员认证）。
* **CUDA 显存溢出异常**。
* **大模型 API 请求超时异常**。

### 5.8 临时文件沙盒管理

系统运行中必须生成唯一的 UUID 临时目录。
必须注册 `atexit` 清理钩子：无论程序正常结束还是遇到异常崩溃，在主进程退出前，必须递归销毁当前 UUID 工作目录下的所有 `.vtt`、`.wav` 及音频切片。

### 5.9 安装发布与分发

系统必须支持以下交付方式：

* **源码运行**：通过 `uv run video-agent -u "<URL>"` 或兼容入口 `uv run main.py -u "<URL>"` 进行开发调试。
* **pipx 安装**：通过 `pipx install .` 暴露 `video-agent` 命令。
* **Wheel 分发**：通过 `uv build --wheel` 生成 `.whl`，并支持 `pip install dist/video_agent_skill-*.whl` 安装。
* **Docker 运行**：通过 Docker 镜像封装 Python、FFmpeg、模型依赖和 CLI 入口。

Wheel 发布前必须满足：

* `pyproject.toml` 定义项目名、版本号、依赖、Python 版本和 `video-agent=video_agent_skill.cli:app` 控制台入口。
* 版本号遵循语义化版本 `MAJOR.MINOR.PATCH`。
* 发布前完成契约测试，确认 `stdout` 仅输出标准 JSON。
* `dist/` 构建产物默认不提交到源码仓库，只作为正式 Release 附件或内部制品分发。

### 5.10 工作进展记录

项目必须维护 `docs/工作进展记录.md` 作为持续跟进文件。每次完成设计、实现、测试、发布准备或关键决策后，必须追加记录。

记录内容至少包括：

* 日期与执行人。
* 工作范围与主要变更。
* 变更文件列表。
* 验证情况。
* 遗留问题和下一步。

记录不得包含密钥、Token、私有 Cookie、受限视频链接或其他敏感信息。

## 6. 非功能需求

### 6.1 隐私与安全性

除了通过 LLM API 进行文本摘要（如果配置了云端大模型）外，所有的原始音视频媒体流禁止上传到任何第三方服务器。ASR 识别必须在用户本地机器上 100% 离线完成。

### 6.2 性能基准

* **字幕通道**：网络畅通时，对于带有字幕的视频，从发起请求到返回最终 JSON 耗时不得超过 5 秒。
* **音频通道**：在具备 4GB 以上显存的独立显卡上，10 分钟长度的无字幕视频，ASR 转写耗时不得超过 1.5 分钟。

### 6.3 可测试性

* `core/extractor.py` 的字幕和音频下载必须支持单独被实例化和测试。
* 全局异常拦截装饰器必须支持传入 mock 函数进行单元测试。

### 6.4 编码规范

* 所有函数、类和方法必须包含严格的类型注解 (Type Hints)。
* 使用 `uv` 维护依赖，并生成跨平台的 `uv.lock`。

## 7. 验收标准

### 7.1 MVP 验收

* 能成功使用 `uv run` 启动 CLI。
* 读取包含国内外平台混合规则的 `config.yaml` 并在请求中正确代理。
* 输入带中文字幕的 YouTube 链接，能秒级触发字幕通道并返回标准 JSON。
* 输入无字幕的视频链接，能自动触发下载音频、VAD 切片、ASR 识别的全流程。
* B站真实样例优先验收平台可达性与 ASR fallback；当 yt-dlp 仅暴露 `danmaku` 时，不视为可用 transcript 字幕。
* Agent 调用测试：通过 LangChain 或 Dify 的 Tool 节点执行该脚本，Agent 能够完美解析 stdout 的 JSON 并且没有被底层打印进度条干扰。
* 进程退出后，检查系统的 `/tmp` 目录，无本任务产生的媒体垃圾文件遗留。

### 7.2 验收测试基线

MVP 发版前必须执行 `docs/验收测试计划.md` 中定义的 P0 用例。测试结论应记录以下信息：

* 命令行完整输入、退出码、stdout JSON 和 stderr 日志摘要。
* 字幕通道与 ASR 通道各至少一个端到端样例；MVP 阶段字幕通道可使用 YouTube 稳定样例，B站覆盖 ASR fallback。
* stdout 文件可被 `json.loads` 直接解析的断言结果。
* 临时目录在正常退出和异常退出后的清理结果。
* 错误码是否与错误码基线保持一致。

## 8. 需求追踪

| 编号 | 需求项 | 来源/目的 | 验收方式 |
| --- | --- | --- | --- |
| REQ-001 | 智能提取降级策略 | 降低算力成本与响应时间 | 传入含字幕/无字幕链接，对比耗时及执行日志 |
| REQ-002 | I/O 全局拦截与 JSON 契约 | 确保 Agent 稳定解析 | 在命令行重定向 stdout 至文件，验证文件内是否为纯 JSON |
| REQ-003 | VAD 音频切片 | 解决 GPU 显存限制 | 使用显存低于 4GB 的设备跑 1 小时音频测试 OOM |
| REQ-004 | atexit 临时文件销毁 | 防止长期运行吃满磁盘 | 中断/正常运行多次后观察临时目录大小 |
| REQ-005 | 全局异常转化为 JSON | 避免死循环重试 | 模拟断网/受限视频，断言 stdout 输出了标准错误 JSON |
| REQ-006 | Wheel 安装发布 | 支持内网、离线和 Agent 平台分发 | 构建 `.whl` 后使用 `pip install dist/*.whl` 安装并运行 `video-agent` |
| REQ-007 | 工作进展记录 | 支持跨会话持续跟进 | 每次任务结束后检查 `docs/工作进展记录.md` 是否追加记录 |
