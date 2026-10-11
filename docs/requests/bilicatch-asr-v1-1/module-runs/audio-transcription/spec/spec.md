# `audio-transcription` 工程规格

## 交付单元标识

`audio-transcription`

## 背景与目标

BiliCatch 当前可下载音视频，但不能把内容转为可检索、可编辑文本。本规格交付完整本地 ASR 闭环：用户从本地文件或完成下载任务创建转写，管理模型，观察进度，编辑结果并导出五种格式；下载完成后可自动触发。音频和文本不上传。

目标不是模拟 ASR 页面，而是以固定 `whisper.cpp` sidecar 和 GGML 模型完成真实推理，并将外部进程、模型网络、B 站字幕和文件系统隔离在可测试 adapter 后。

## 范围内

- `/transcribe` 路由、侧栏、页面、模型管理弹窗、结果编辑器和中英文文案。
- 受支持本地媒体、已完成下载任务和下载后自动转写来源。
- Tiny/Base/Small/Medium/Large-v3 模型下载、进度、取消、SHA-256、校验、删除和默认切换。
- FFmpeg 媒体探测、16kHz 单声道 PCM WAV 预处理及长音频分段。
- whisper sidecar 推理、自动/指定语言、时间戳、标点选项和 CPU/GPU 模式。
- 独立转写任务队列、持久化、状态机、并发、下载优先级、取消和重试。
- 平台字幕优先 adapter 与无字幕/失败时本地 ASR 回退。
- 规范化分段结果、缓存、文本编辑、复制、TXT/SRT/VTT/JSON/Markdown 导出。
- 任务页联合展示、设置 schema 扩展、系统通知、externalBin 和第三方说明。
- E011-E017、IPC/事件合同、自动化验证和 Windows 真实 smoke。

## 范围外

- 实时录音、说话人分离、字幕翻译、时间轴编辑和云端 ASR。
- 批量选择多个本地文件创建任务。
- 通用任务引擎重构或合并下载/转写后端实体。
- 自动搬迁旧模型目录。
- 在当前 Windows 环境完成 macOS/Linux 的签名、真实 GPU 或安装后 smoke。

## 触发与开始条件

- 用户访问 `/transcribe` 并选择本地文件。
- 用户从任务页完成下载项点击“转写”，进入 `/transcribe?taskId=<id>`。
- 用户在下载中心为有音频输出的任务启用自动转写，任务完成后后端触发。
- 用户对失败/取消/完成任务选择重试或重新转写。

开始任务前必须满足：源文件存在且受支持、保存目录有效、参数枚举有效、所选模型安装且摘要有效。任何一项失败都不得创建成功任务记录。

## 需求拆分摘要

本请求只有 `audio-transcription` 一个交付单元。它同时包含页面、模型、队列、结果和必要集成，因为这些能力共同构成一个用户可完成的转写闭环。详细来源追踪见 `requirements/modules/audio-transcription.md`。

## 用户流程

### 手动转写

1. 页面初始化设置、模型、转写任务和 route query 来源。
2. 用户从完成任务或本地文件选择一个源，后端返回媒体元信息。
3. 用户确认模型、语言、格式、时间戳、标点、自动复制和目录。
4. 点击开始；前端仅在本地表单有效时调用 `create_transcribe_task`。
5. 后端再次校验文件、目录、模型和枚举，持久化 Pending 任务并返回实体。
6. scheduler 在并发与下载优先级允许时运行 pipeline。
7. 页面通过快照+事件展示阶段；离开页面不取消任务。
8. 完成后加载规范化结果，允许文本编辑、复制、导出、打开目录或重新转写。

### 模型首次使用

1. 模型未安装时，开始按钮禁用并显示 E011 语义。
2. 打开模型管理，选择模型下载。
3. adapter 写入 `.part`，持续发进度；取消/失败清理临时文件。
4. 下载完成后核对字节数和 SHA-256，成功后原子 rename 并更新模型 revision。
5. 页面收到事件后允许使用；校验失败显示 E017，可删除/重试。

### 自动转写

1. 用户在下载任务草稿上显式设置 `autoTranscribe`，或未显式覆盖时使用全局默认。
2. 只有 `audio-only` 与 `video-audio` 完成且 `outputPath` 存在时触发。
3. 后端以 `download:<downloadTaskId>:<outputRevision>` 为幂等键创建转写任务。
4. 模型缺失时创建不进入推理：记录 E011 失败任务并通知用户；不得静默下载大模型。
5. 重复完成事件不能创建重复转写任务。

## 页面与模块设计

页面严格承接 `design/page-design.md`：应用壳不变，主页面为标题、来源、参数/运行、进度/结果四段；桌面两列、窄屏单列；模型管理为扁平列表模态框；任务/设置/下载中心使用现有布局的增量，不创建嵌套卡片。

代码严格承接 `design/architecture-design.md`：独立转写 feature 和 Rust domain；commands 薄层；manager/scheduler/runner/state machine 分责；外部能力通过 ports/adapters；任务页通过展示 adapter 联合呈现。

## 功能完整行为分解

### 路由、导航与恢复

- `AppRouteName` 增加 `transcribe`，路由 `/transcribe`，meta 使用 `nav.transcribe`。
- 侧栏增加 `FileAudio` 图标项，顺序为下载、任务、转写、设置。
- `taskId` query 仅接受非空字符串；无效/非完成/无输出的 ID 显示可恢复错误并保留来源选择器。
- 页面卸载只退订监听，不取消后端任务；重入先订阅再查询快照并按 sequence/revision 合并缓冲事件。

### 来源选择与元信息

- 支持扩展名（大小写不敏感）：`.mp3 .m4a .aac .flac .wav .ogg .mp4 .mkv .flv .mov .webm`。
- 系统对话框按音频/视频扩展名过滤；取消选择不改变当前来源。
- 拖拽只接受一个文件。0 个忽略；多于 1 个显示输入错误；目录或不支持文件拒绝。
- `inspect_transcription_source` 返回 `fileName/path/durationSeconds/sizeBytes/mediaKind/format`。
- 后端通过文件 metadata 和 FFmpeg probe 验证；扩展名允许但解码失败返回 E013。
- 完成任务下拉只包含 `status=completed` 且有 `outputPath` 的音频/含音频任务；文件已丢失时条目禁用。

### 参数表单

| 字段 | 类型 | 默认 | 校验/联动 |
| --- | --- | --- | --- |
| modelId | tiny/base/small/medium/large-v3 | Base | 必须安装且 valid |
| language | auto/zh/zh-TW/en/ja/ko | auto | 直接映射 sidecar 支持代码 |
| outputFormat | txt/srt/vtt/json/markdown | srt | 必填 enum |
| timestamps | boolean | true | srt/vtt/json 强制 true 且禁用 |
| punctuation | boolean | true | 只控制本地后处理，不使用网络 |
| autoCopy | boolean | false | 完成后前端 best-effort |
| outputDirectory | string path | 下载目录下 Transcribe | trim 后不可空，后端验证/创建 |
| gpuMode | auto/enabled/disabled | 来自设置 | enabled 且 sidecar 不支持时创建失败 E014 details=`GPU_UNAVAILABLE` |

- 用户切换格式时不丢失其 TXT/Markdown 的时间戳偏好；强制格式只在有效值中覆盖展示/请求。
- 开始按钮在来源未就绪、表单无效、模型不可用、创建 pending 时禁用。
- create loading 期间所有影响请求的字段禁用；失败后恢复并保留值。

### 模型目录与下载

模型 manifest 字段：`id/displayName/fileName/sizeBytes/sha256/officialUrl/recommendedFor/capabilities`。manifest 编译进应用代码或受版本控制的静态 JSON，不能运行时接受远程摘要覆盖。

- `list_asr_models` 扫描设置中的模型目录，以 manifest 校验存在、大小和 SHA-256；可按文件 metadata 缓存校验结果，但显式“校验”必须重新计算。
- 官方源使用 manifest URL；自定义源 trim 后必须为 HTTPS 且只替换 base URL，最终文件名固定为 manifest `fileName`。
- 下载使用流式 HTTP，限制重定向目标为 HTTPS，写 `<file>.part`，每个 modelId 同时最多一个下载。
- 下载事件字段：`sequence/modelId/state/downloadedBytes/totalBytes/progress/error`；state 为 queued/downloading/verifying/completed/failed/cancelled。
- 取消下载幂等；失败/取消删除 `.part`。
- 校验顺序为预期字节数再 SHA-256；不匹配返回 E017 并删除无效最终/临时文件。
- 删除默认模型允许，但删除后默认设置回退 Base；若 Base 也未安装仍保持默认 ID，页面显示未安装。
- 删除 `inUse=true` 的模型先要求 UI 确认，后端仍拒绝并返回可识别 details=`MODEL_IN_USE`，防竞态。
- 模型目录切换后立即重新扫描，不移动或删除旧目录。

### 转写任务合同与状态机

`TranscribeTaskStatus`：

- `pending`
- `preprocessing`
- `loadingModel`
- `transcribing`
- `processing`
- `saving`
- `completed`
- `failed`
- `cancelled`

允许转移：

```text
pending -> preprocessing | cancelled | failed
preprocessing -> loadingModel | cancelled | failed
loadingModel -> transcribing | cancelled | failed
transcribing -> processing | cancelled | failed
processing -> saving | cancelled | failed
saving -> completed | failed
failed -> pending (retry creates new run revision on same task)
cancelled -> pending (retry)
completed -> no mutation; re-transcribe creates a new task
```

- `cancel` 对 pending 到 processing 可用；saving 已进行原子最终写入时不再暴露 cancel，避免半完成结果。
- `retry` 只对 failed/cancelled；清除运行进度/error，增加 revision，保留来源和参数，并重新校验模型/文件。
- `delete` 只对终态；删除任务记录与内部结果索引，不删除源文件或用户已导出的文件。
- 每个变更增加 task revision；列表事件包含全局 sequence。前端忽略低 sequence 和低 revision。

### 调度与资源优先级

- `maxConcurrentTranscriptions` 范围 1-3，默认 1。
- scheduler 只从 Pending 获取租约；设置降低时不终止运行任务。
- `DownloadActivityProbe` 返回当前 downloading/processing 数量。存在下载活动时最多运行 1 个转写；无下载活动时可用设置上限。
- scheduler 不持有下载 manager 锁执行转写，也不让下载 runner等待转写锁。
- 同一模型可被多个转写任务读取，但 active usage 计数阻止删除。

### FFmpeg 预处理与长音频

- 输入通过 argument vector 传递，不拼 shell 字符串。
- 输出为 PCM signed 16-bit little-endian、16kHz、mono WAV。
- 时长不超过 30 分钟可生成一个 WAV；更长时按 30 分钟目标切段，并在目标点前后 5 秒内选择可获得的静音边界。没有静音时按目标点切分。
- 每段记录 `offsetMs`，合并结果时加偏移并保证非递减、end >= start。
- 临时根目录为设置临时目录下 `bilicatch-transcribe/<taskId>-<runRevision>`。
- cleanup guard 在 completed/failed/cancelled/panic unwind 可控路径清理；启动恢复时清理没有 active task 的陈旧目录。
- FFmpeg 不可用或无法解码映射 E013，details 区分 `FFMPEG_UNAVAILABLE` / `DECODE_FAILED`。

### whisper sidecar

- 固定 `whisper.cpp` v1.8.6 的 `whisper-cli`，目标三元组命名与现有 FFmpeg locator 规则一致。
- sidecar 接收 model path、WAV path、language、output-json/full-json、timestamps 和 GPU 参数；参数必须由 typed builder 生成。
- Windows 子进程不得创建可见控制台；取消必须终止子进程树。
- adapter 解析 sidecar 产物为 `TranscriptSegment[]` 和 detected language；原始日志只进入诊断 details，不进入用户正文。
- 非零退出、输出缺失/无效返回 E014；成功但没有非空语音 segment 返回 E015。
- stderr 进度解析是 best-effort；无法解析不导致任务失败，阶段仍可观察。

### 平台字幕优先

- 仅 `DownloadTask` 来源且有 BVID/CID 时尝试。
- `preferPlatformSubtitles=false`、无身份、本地文件直接跳过。
- adapter 返回 Available(segments/language)、Unavailable 或 Failed。
- Unavailable/Failed 均回退本地 ASR；Failed 记录诊断，不把任务置失败。
- 下载到的字幕转为同一 `TranscriptSegment`，仍执行导出/保存/缓存。

### 结果、编辑与缓存

`TranscriptResult` 持久字段：`id/taskId/revision/sourceFingerprint/modelId/requestedLanguage/detectedLanguage/segments/plainText/createdAt/updatedAt`。

- segment：`index/startMs/endMs/text`；文本必须是有效 UTF-8。
- `plainText` 由非空 trim 后 segment 文本用换行连接生成，不作为独立可漂移规则源。
- 编辑只接收完整 segment 文本数组和 `expectedRevision`；数量和 index 必须匹配，时间不可由前端提交修改。
- 每段文本去除首尾空白，内部空格/换行保留；空白段允许保存但纯文本跳过。
- revision 不匹配返回 `E_INTERNAL` details=`RESULT_REVISION_CONFLICT` 和最新结果查询路径。
- 缓存键包括规范化绝对路径、size、mtime、modelId、language、punctuation；输出格式和 timestamps 不影响识别缓存，只影响导出。该规则比 PRD 的最小键更强，避免同一识别结果因格式重复推理。
- `force=true`（重新转写）绕过缓存并创建新任务；普通缓存命中仍创建 completed 任务，标记 `cacheReused=true` 并发事件。

### 标点与语言

- Whisper 原始标点保留是默认。
- `punctuation=false` 使用确定性的本地规则移除 Unicode 中英文句末/分隔标点，但不删除字母内撇号、数字小数点或路径内容；规则由纯函数测试固定。
- 自动语言以 sidecar detected language 返回；手动语言将 requested/detected 均以请求代码呈现。
- 简中/繁中不额外进行文字转换；`zh` 和 `zh-TW` 是识别提示/结果标签，不能承诺字形强制转换。

### 导出与命名

- 基名去掉源扩展名并复用现有非法文件名清理规则，限制最终文件名不超过 200 字符。
- 语言标签用 detected language，无法检测时用 requested language，仍为 auto 时用 `und`。
- 名称：`<base>.<language>.<ext>`；存在则顺序查找 `(<n>)`，不覆盖。
- 写入同目录临时文件并原子 rename；失败清理临时文件，返回 E016。
- TXT：纯文本，无时间戳。
- SRT：1-based 序号，`HH:MM:SS,mmm --> HH:MM:SS,mmm`。
- VTT：首行 `WEBVTT`，时间为 `HH:MM:SS.mmm`。
- JSON：UTF-8 pretty JSON，包含 language 和 segments 的 startMs/endMs/text。
- Markdown：有时间戳时每段为 `- [HH:MM:SS.mmm] 文本`，否则按段落输出。
- `export_transcript` 接收 taskId/resultRevision/format/timestamps/outputDirectory；强制格式忽略 false timestamps。

### 复制与打开目录

- 复制由前端 clipboard adapter 执行；纯文本来自结果，带时间戳使用 SRT formatter 的内存输出。
- 自动复制仅在页面仍挂载且任务在当前会话完成时尝试；失败显示局部错误，不修改任务。
- 打开目录通过后端验证已保存 `outputPath` 的父目录后调用 opener，不能接受前端任意路径。

### 任务页联合展示

前端定义 discriminated union `TaskListItem`，`kind: download | transcription`，公共字段为 ID、name、kind、statusGroup、statusLabel、progress、primaryMetric、secondaryMetric、error、actions、updatedAt。

- 下载 adapter 保持现有行为。
- 转写 adapter 将阶段映射到 active，速度显示 `x realtime`，指标显示处理时长。
- 筛选：all/download/transcription/active/completed/failed。
- 完成下载且含音频的 actions 增加 transcribe；完成转写 actions 为 viewResult/openDirectory/delete/retranscribe。
- action dispatcher 按 kind 调用下载或转写 service；不能使用 nullable 字段猜类型。
- “暂停全部”仍只控制下载；“清空已完成”由 UI 明确作用于当前筛选，后端分别调用两个领域清理命令并汇总部分失败。

### 下载中心联动

- `DownloadTaskDraft` 增加 `autoTranscribe: boolean` 和可选 `transcriptionPreset`（模型/语言/格式/时间戳/标点）。
- 页面只在 audio-only/video-audio 显示选项；video-only 强制 false。
- 默认值来自设置；用户更改后写入草稿，后续设置变化不追溯修改已创建下载任务。
- 下载完成 hook 必须幂等且不能把下载成功改成失败；创建转写失败以独立转写失败任务/通知呈现。

### 设置扩展与迁移

Settings schema 从 1 升到 2。读取 schema 1 时保留原字段并补默认转写字段，然后以 schema 2 保存；读取 schema 2 严格校验。

新增字段及值：

- `defaultAsrModel`: tiny/base/small/medium/large-v3，默认 base。
- `defaultTranscriptionLanguage`: auto/zh/zh-TW/en/ja/ko，默认 auto。
- `defaultTranscriptFormat`: txt/srt/vtt/json/markdown，默认 srt。
- `transcriptionTimestamps`: boolean，默认 true。
- `transcriptionPunctuation`: boolean，默认 true。
- `preferPlatformSubtitles`: boolean，默认 true。
- `autoTranscribeDownloads`: boolean，默认 false。
- `maxConcurrentTranscriptions`: integer 1-3，默认 1。
- `asrModelDirectory`: trim 后非空路径，默认软件安装目录下 `models/whisper`；UI 显示平台实际路径，不硬编码 `~`。
- `asrGpuMode`: auto/enabled/disabled，默认 auto。
- `asrModelSource`: trim 后空字符串代表官方源，非空必须 HTTPS 且不得包含 query/fragment，默认空。

目录通过系统 picker 提交；download source 在 blur/Enter 保存。字段保存继续使用现有逐字段状态。

### 错误与通知

E011-E017 进入 Rust `AppErrorCode` 和前端 `AppErrorCode` union，i18n 同时提供中文/英文用户文案。`details` 仅用于稳定诊断分类，不展示原始本机敏感路径或完整 sidecar 命令。

- Completed：通知“转写完成：{fileName}”。
- Failed：通知“转写失败：{fileName}，可重试”。
- 用户主动取消不发失败通知。
- 通知点击定位 `/transcribe?taskId=...`；若平台通知不支持 action payload，至少唤起窗口并由应用内最近任务入口恢复。

## 设计约束

### 职责边界

- 页面编排、业务状态、IPC、领域状态机和外部 I/O 分层。
- 下载与转写实体分离；任务页展示 union 是唯一聚合层。
- manager 不直接实现 HTTP/进程/文件格式解析。

### 命名与领域语言

- 使用 `transcription` 表示领域，`transcribe` 表示命令动作，`transcript` 表示结果资产，`asrModel` 表示模型。
- 后端 serde 字段和前端合同使用相同 camelCase 语义，不为本地偏好重命名。

### 规则所有权

- 状态转移：Rust `state_machine`。
- 模型有效性：`ModelRepository`。
- 文件格式生成：export 模块。
- 临时清理：pipeline cleanup guard。
- 页面显示映射：前端 task adapter。
- 设置范围：Rust settings validation，前端只做即时反馈。

### 副作用

- 网络、磁盘、进程、通知、剪贴板、文件对话框和打开目录必须位于对应 adapter。
- 自动转写由后端下载完成事件触发，不依赖页面保持打开。
- 缓存写入在结果成功持久化后执行，不能在部分结果时暴露命中。

### 复杂度护栏

- 不引入通用任务框架、DI 容器或 exporter 类层级。
- pipeline 顶层保持可顺序阅读；低层参数和解析抽离。
- 前端 store 不同时承担格式生成和 DOM 虚拟化。
- 单测使用 ports 替身，不以 sleep 和真实网络证明状态机。

## 项目脚手架决定

这是现有应用功能，不新建项目/包。复用现有 Vue feature、Pinia、injection、Tauri commands/services/infrastructure 和 Vitest/Cargo 测试结构。允许新增上述目录，不更换框架、状态库、UI 库或构建工具。

## 变化轴与模式决定

- 外部引擎/字幕/模型存储：Port/Adapter，因供应商和失败注入真实变化。
- 任务阶段：State Machine，因取消/重试/终态竞态真实存在。
- commands 到协作者：Manager Facade，保持小入口。
- 下载/转写统一 UI：Adapter + discriminated union。
- 五种导出：直接 enum 分派纯函数；拒绝 factory 层。
- 当前只有 whisper 一个引擎 adapter，但 `Transcriber` trait 是 PRD 明确替换边界且用于测试，不是未来假设抽象。

## 代码上下文与影响假设

- 结构依据 `artifacts/code-context.md`；无代码图，使用手工导入/装配追踪。
- 现有任务 store/row 以 DownloadTask 为中心，需要最小重构为展示 union，但下载 service 和后端 manager 不重写。
- 设置当前只接受 schema v1，需要显式迁移而非直接拒绝旧文件。
- FFmpeg runner/locator 可复用思路；转写预处理不能强行复用带音频封面/编码职责的 `MediaProcessorPort`。
- Tauri 启动必须在 whisper sidecar 缺失时仍成功；创建任务时返回稳定错误。

## API 与数据合同

权威合同源为 Rust `models/transcription.rs`、`models/settings.rs` 及 PRD 字段表；前端没有后端生成 TypeScript，使用镜像类型与 JSON 序列化契约测试。

### 命令

| 命令 | 请求 | 成功响应 | 失败 |
| --- | --- | --- | --- |
| `inspect_transcription_source` | `{ source }` | `MediaSourceInfo` | E013/Internal |
| `list_transcribe_tasks` | 无 | `TranscribeTaskListSnapshot` | Internal |
| `create_transcribe_task` | `CreateTranscribeTaskRequest` | `CreateTranscribeTaskResult { task, reused }` | E011/E013/E016/Internal |
| `control_transcribe_task` | `{ taskId, action }` | `TranscribeTask` | E011-E016/Internal |
| `clear_completed_transcribe_tasks` | 无 | IDs/result | Internal |
| `list_asr_models` | 无 | `AsrModelListSnapshot` | E017/Internal |
| `download_asr_model` | `{ modelId }` | `AsrModel` queued/downloading | E012/E017 |
| `cancel_asr_model_download` | `{ modelId }` | `AsrModel` | E012/Internal |
| `verify_asr_model` | `{ modelId }` | `AsrModel` | E017 |
| `delete_asr_model` | `{ modelId }` | `AsrModelListSnapshot` | E017/Internal |
| `get_transcript_result` | `{ taskId }` | `TranscriptResult` | E014/Internal |
| `save_transcript_edits` | `{ taskId, expectedRevision, segments[] }` | `TranscriptResult` | Internal conflict/validation |
| `export_transcript` | `{ taskId, resultRevision, format, timestamps, outputDirectory }` | `{ outputPath }` | E016 |
| `open_transcript_output` | `{ taskId }` | void | E016/Internal |

请求字段均为 required，除非类型明确 `Option`；nullable 只用于尚无 total/eta/result/output/error 的运行状态。成功不能由空响应表示。

### 事件

- `transcribe://progress`: `{ sequence, task }`，其中 task 含完整当前任务，PRD 进度字段均在 task.progress 中。
- `transcribe://removed`: `{ sequence, taskId }`。
- `transcribe://model_download_progress`: `{ sequence, model }`，model 含下载状态与进度。

订阅先于查询；前端缓冲 hydration 期间事件并按 sequence/revision 重放。

### TypeScript 编译上下文

- governed by `/tsconfig.json`。
- `strict=true`、`noUnusedLocals/Parameters=true`、`moduleResolution=bundler`、target ES2020。
- 无 `baseUrl`/`paths`；使用相对导入。
- 相关声明只包括 `src/vite-env.d.ts`、Vue/Tauri 包类型和直接导入的本地 contracts；禁止实现时扫描/猜测全仓 `.d.ts`。

## 上下文与依赖来源

- 产品：PRD snapshot、requirement module、page/architecture design。
- 前端：现有 router/navigation/main、download/task/settings feature 模式。
- Rust：现有 tasks/settings/audio/download runtime 和 lib setup。
- 外部：固定 whisper.cpp v1.8.6、版本化模型 manifest、B 站字幕 fixture。
- 工具：Node/Vitest/vue-tsc/Vite；Cargo test/check；Windows sidecar smoke。

## 边界情况

- 源文件选中后被删除或修改：创建时重新校验；运行时失败 E013；缓存 fingerprint 失效。
- 应用在任务运行时退出：启动时把非终态任务恢复为 Pending 并增加 run revision，先清理陈旧 workspace；不假装从 sidecar 中点续跑。
- 磁盘不足：模型下载 E012、临时预处理 E013 details、导出 E016；清理 partial。
- 保存目录只读：创建可允许排队，但保存阶段 E016；页面可修改目录并重新导出，不需重新推理。
- 模型在排队期间被删除：开始前重新 acquire/verify，失败 E011/E017。
- 模型下载源非法：设置提交失败，不覆盖旧值。
- 事件丢失：重新进入/显式刷新以查询快照恢复。
- sidecar 输出 malformed：E014，保留去敏诊断。
- 无语音：E015，不生成空导出。
- 平台字幕存在但格式坏：记录诊断并回退 ASR。
- 用户编辑后收到旧完成事件：revision 合并不覆盖更高 result revision。
- 超长单词/路径：UI 省略或换行，tooltip/可访问文本保留完整值。

## 验收标准

### AC-01 路由与页面

- `/transcribe` 可从侧栏进入；刷新和 query handoff 正常。
- 360/800/1280px 等代表宽度无不可理解重叠或横向溢出；键盘可完成主流程。

### AC-02 来源

- 所有列出的扩展名可进入 inspect；无效、多文件、目录、丢失任务输出被拒绝并显示可恢复错误。
- 已完成任务 handoff 自动填充正确文件。

### AC-03 模型

- 至少 Tiny/Base 的真实官方模型下载可显示进度、SHA-256 校验、原子安装和删除。
- `.part` 在取消/失败后不存在；损坏模型不显示可用。
- 使用中模型的删除在前后端双重阻止。

### AC-04 真实转写

- Windows 上以固定 whisper sidecar + 官方 Tiny/Base 模型对一段已知 WAV 完成真实本地推理，结果含非空文本和有序时间戳；运行期间无可见控制台。
- 输入音频不产生任何上传请求；模型下载是唯一必要外网行为。

### AC-05 状态、取消与恢复

- 合法状态转移、非法转移、取消幂等、重试、过期事件忽略和启动恢复均有自动化测试。
- 完成/失败/取消后临时 workspace 被清理。

### AC-06 结果与导出

- 五种格式对 fixture 生成内容正确；SRT/VTT/JSON 强制时间戳；命名冲突不覆盖。
- 文本编辑使用 revision，冲突被拒绝；重新转写绕过缓存，普通相同请求命中缓存。

### AC-07 设置与联动

- schema v1 可无损迁移 v2；11 个转写字段的默认、校验和即时保存有前后端测试。
- audio-only/video-audio 可自动转写，video-only 不可；重复完成事件仅创建一项。
- 自动转写失败不改变下载任务 completed 状态。

### AC-08 任务与通知

- 下载/转写联合列表正确筛选、指标和动作；原下载暂停/恢复/取消/清理回归测试通过。
- 转写完成/失败通知触发，取消不触发失败通知。

### AC-09 工程门禁

- `npm test`、`npm run build`、`cargo test`、`cargo check` 通过。
- IPC serde fixture、模型 manifest、sidecar locator/args/parser、设置迁移、导出和 adapter fallback 有测试。
- 不提交 ASR 模型文件；提交的 sidecar/许可符合仓库发布政策。

### AC-10 性能与平台交接

- 在记录 CPU、核数、sidecar build、模型、音频时长的环境中报告 realtime factor；若未达 PRD 目标，不能隐瞒，须记录阻断/优化结论。
- Windows 打包 smoke 通过；macOS/Linux 的目标命名和配置测试通过，真实签名/安装 smoke 明确列为平台发布门禁。

## 人工评审与交接

- 用户审批本规格后才生成实施计划。
- 计划审批后按 TDD 执行。
- 真实模型下载和 sidecar 测试可能耗时/占带宽，执行结果需在 verification 工件列明资产版本和摘要。
- macOS/Linux 需要对应主机/CI 和发布凭据时，作为外部发布交接，不得伪称完成。

## 风险

- 外部 CLI/模型资产漂移：固定版本、manifest 和 fixture。
- 大模型下载/磁盘：流式、进度、校验、partial 清理和错误可恢复。
- 长音频和并发资源：分段、低优先级、并发限制和 benchmark。
- 跨域任务页回归：展示 union 隔离，保留下载合同和全量回归。
- 设置老数据：显式 v1->v2 迁移。
- B 站字幕不稳定：adapter 失败无条件回退本地 ASR。
- sidecar 跨平台：契约测试不能替代实机发布 smoke，交接必须显式。
