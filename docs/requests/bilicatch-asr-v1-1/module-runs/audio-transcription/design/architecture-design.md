# `audio-transcription` 架构设计

## 交付单元标识

`audio-transcription`

## 架构目标

在现有 Vue/Tauri 应用中增加独立、可测试、可取消、可持久化的本地转写领域，同时只通过稳定 adapter 与下载、任务展示、设置和系统能力协作。下载状态机不能因 ASR 的阶段、进度和结果字段而膨胀。

## 架构范围与触发

### 范围

- 前端转写 feature、页面、模型管理和联合任务展示。
- Rust 转写模型、命令、服务、调度、持久化、缓存和基础设施 adapters。
- 设置/下载/任务/通知/sidecar 分发的必要扩展。

### 触发

- 用户从 `/transcribe` 创建或重新转写。
- 用户从已完成下载任务点击“转写”。
- 带音频下载任务完成且该任务启用自动转写。
- 平台字幕优先分支在创建任务后、FFmpeg/ASR 前执行。

## 上游输入与假设

- `requirements/modules/audio-transcription.md` 是产品边界。
- `design/page-design.md` 固定页面区块和交互层级。
- `artifacts/code-context.md` 固定现有依赖和回归边界。
- whisper.cpp 采用固定发布版本的 `whisper-cli` sidecar；模型采用固定 GGML manifest。
- CPU sidecar 是跨平台最低基线，GPU 是构建能力而不是所有包的无条件保证。

## 模块边界设计

### 前端

`src/features/transcription/` 作为独立 feature：

- `contracts.ts`：Rust IPC 的 camelCase 镜像和 UI 展示类型。
- `service.ts`：转写/模型/结果命令封装。
- `events.ts`：转写与模型下载事件订阅。
- `store.ts`：页面会话、任务/模型/结果快照和事件合并。
- `source.ts`：文件选择、拖拽路径规范化和来源展示 adapter。
- `formatters.ts`：时长、大小、时间戳和复制文本格式化。
- `components/`：来源、参数、进度、结果、模型弹窗等聚焦组件。

页面 `TranscribePage.vue` 只负责编排 route query、store 初始化、用户命令和组件组合。

### Rust

`models/transcription.rs` 定义稳定序列化合同；`services/transcription/` 持有领域行为；`infrastructure/transcription/` 持有外部副作用。

- `manager.rs`：任务/结果查询、创建、取消、重试、编辑保存、导出入口。
- `scheduler.rs`：并发、下载优先级、排队和 active model usage。
- `state_machine.rs`：允许的状态转移和动作。
- `cache.rs`：请求指纹、缓存查找和 bypass。
- `export.rs`：从规范化结果生成五种格式。
- `ports.rs`：`Transcriber`、`AudioPreprocessor`、`ModelRepository`、`SubtitleProvider`、`TranscriptionStore`、`EventSink`、`DownloadActivityProbe`。
- `runner.rs`：单个任务的有序 pipeline 和清理守卫。

基础设施包含：

- `whisper_sidecar.rs`：命令参数、隐藏进程、输出解析、取消和能力探测。
- `ffmpeg_preprocessor.rs`：probe、16k mono PCM、分段/临时文件。
- `model_repository.rs`：manifest、HTTP 流、SHA-256、原子落盘和删除。
- `json_store.rs`：任务/结果索引持久化和 schema 迁移。
- `bilibili_subtitles.rs`：可选平台字幕 adapter。
- `tauri_adapter.rs`：事件与系统通知。

## 文件与目录结构

```text
src/
  features/transcription/
    contracts.ts
    service.ts
    events.ts
    injection.ts
    store.ts
    source.ts
    formatters.ts
    transcription.css
    components/
      SourcePicker.vue
      FileSummary.vue
      TranscriptionOptions.vue
      TranscriptionProgress.vue
      TranscriptEditor.vue
      TranscriptActions.vue
      ModelManagerDialog.vue
      ModelRow.vue
  pages/TranscribePage.vue

src-tauri/src/
  models/transcription.rs
  commands/transcription.rs
  services/transcription/
    mod.rs
    manager.rs
    scheduler.rs
    state_machine.rs
    runner.rs
    cache.rs
    export.rs
    ports.rs
  infrastructure/transcription/
    mod.rs
    ffmpeg_preprocessor.rs
    whisper_sidecar.rs
    model_repository.rs
    json_store.rs
    bilibili_subtitles.rs
    tauri_adapter.rs
```

测试与 fixture 跟随既有约定放在前端同目录测试及 `src-tauri/tests/`、`tests/fixtures/transcription/`。

## 代码关系与依赖方向

```text
Vue components -> Pinia store -> TranscriptionService/EventSource -> IpcTransport
                                                             |
Tauri commands -> TranscriptionManager -> domain ports ------+
                                    |
                         Scheduler -> Runner -> ports
                                                   |
        FFmpeg / whisper / HTTP / filesystem / subtitles / events adapters
```

- 内层服务只依赖 ports 和 models，不依赖 Tauri 类型。
- commands 不包含状态转移或文件逻辑。
- infrastructure adapters 可依赖 Tauri/reqwest/tokio/sha2，但不能反向被 models 引用。
- 下载模块仅调用 `AutoTranscriptionSink::enqueue_completed_download`；转写模块通过只读 probe 查询下载活跃度。
- 任务页通过 `TaskListItem` 展示 union 消费两个 feature store，不让任一领域 import 另一个领域实体。

## 职责拆分

### 页面与组件

- `TranscribePage`：生命周期和 route handoff。
- `SourcePicker`：来源选择与验证反馈，不加载模型。
- `TranscriptionOptions`：参数输入与格式联动，不创建任务。
- `ModelManagerDialog`：模型命令与 per-model 状态，不操作转写任务。
- `TranscriptEditor`：编辑本地 draft，不直接写文件。
- `TranscriptActions`：调用复制/保存/打开/重新转写命令。

### Store 与 Service

- store 拥有异步状态和快照合并，不拥有格式生成规则。
- service 保留 IPC 字段与错误，不将 E011-E017 变成任意字符串。
- 展示 adapter 将 DownloadTask/TranscribeTask 转为 `TaskListItem`；action dispatcher 按 `kind` 分派。

### Rust 服务与 adapter

- Manager 是用例 facade；Scheduler 只调度；Runner 只执行一次 pipeline。
- exporter 是纯函数/纯文件内容生成器；实际写文件由 manager/port 完成。
- cleanup guard 集中管理临时文件，避免每个错误分支重复删除。
- ModelRepository 是模型完整性唯一所有者；UI 和 runner 不自行判定文件存在即安装。

## 函数设计与公开入口

### 前端服务

- `listTranscribeTasks()`
- `inspectTranscriptionSource(request)`
- `createTranscribeTask(request)`
- `controlTranscribeTask({ taskId, action })`
- `listAsrModels()`
- `downloadAsrModel({ modelId })`
- `cancelAsrModelDownload({ modelId })`
- `deleteAsrModel({ modelId })`
- `verifyAsrModel({ modelId })`
- `getTranscriptResult({ taskId })`
- `saveTranscriptEdits(request)`
- `exportTranscript(request)`

### Rust manager

- queries 与 commands 分离；所有 command 返回更新后的实体或明确结果。
- `create` 先完成文件/参数/模型校验，再持久化 pending task；失败不得留下幽灵任务。
- `control` 执行 cancel/retry/delete，终态幂等。
- `export` 使用 revision 防止旧编辑覆盖新结果。

### Pipeline

`run_task(task_id)`：

1. 获取排他运行租约并进入 preflight。
2. 若允许且存在平台字幕，规范化字幕并跳到结果保存。
3. 获取模型使用租约。
4. 预处理/分段源文件。
5. 对段执行 `Transcriber::transcribe` 并合并时间偏移。
6. 检测空语音，执行本地后处理。
7. 持久化规范化结果并生成默认格式。
8. 进入 Completed，发送事件/通知。
9. cleanup guard 在所有返回路径删除临时数据并释放租约。

## 状态归属与数据流

### 后端事实源

- 转写任务、结果索引、模型安装状态和设置由 Rust 持有。
- 前端 store 是查询快照与事件的可丢弃镜像。
- 编辑中的未保存文本属于页面 store；保存后由后端 result revision 成为事实源。

### 前端页面状态

- `sourceDraft`、`optionDraft`、`editorDraft` 分开，避免切换来源覆盖完成结果。
- `tasksById`、`modelsById` 和 `resultsByTaskId` 使用 ID map。
- `taskSequence`/`revision` 与 `modelRevision` 分别合并事件，模型下载 busy 不阻塞转写事件。

### 持久化

- 转写 store 使用独立 schema 文件，避免修改下载任务 JSON 结构。
- 结果正文可单独 JSON 存储，任务只保存 result ID/path；大结果不塞入任务列表响应。
- 设置文档升级为 schema v2，缺失转写字段时填默认值；保留现有用户字段。

## 数据结构与类型策略

### 核心 Rust/IPC 类型

- `TranscribeTaskStatus`: `pending | preprocessing | loadingModel | transcribing | processing | saving | completed | failed | cancelled`。
- UI 可将细粒度状态映射为 PRD 的 Pending/Transcribing/Processing/Completed；后端保留阶段可观测性。
- `TranscribeTask`: ID、revision、来源、参数、状态、progress、timing、resultId/outputPath、error、cacheReused、timestamps。
- `TranscriptResult`: resultId、taskId、revision、detectedLanguage、segments、plainText、createdAt、updatedAt。
- `TranscriptSegment`: index、startMs、endMs、text。
- `AsrModel`: id、displayName、bytes、installed/valid/default/inUse、path、capabilities、download state/error。
- `TranscriptionProgressEvent` 和 `ModelDownloadProgressEvent` 保留 PRD snake_case 语义，经 serde camelCase 到前端。

### 来源类型

`TranscriptionSource` 使用 tagged enum：

- `LocalFile { path }`
- `DownloadTask { task_id, output_path, bvid?, cid? }`

下载来源保留 B 站身份用于平台字幕，但本地来源不伪造 BVID。

### TypeScript

- `tsconfig.json` governs `src/**/*.ts|vue`，关键约束为 `strict`、`noUnused*`、ES2020、bundler resolution、无路径别名。
- `src/vite-env.d.ts` 只提供 Vite client；没有可复用后端生成 TS 类型。
- 因 Rust 是权威合同，前端 `contracts.ts` 镜像 serde 输出字段并用契约 fixture 测试保持对称。
- UI `TaskListItem` 是显式 adapter 产物，不是后端合同。

## 合同与 Adapter 边界

- Rust models 是 IPC 权威 source；TypeScript 保留字段含义和 enum 字符串。
- whisper CLI 输出通过 adapter 解析为规范化 segment；原始 CLI 文本不进入 UI。
- B 站字幕 raw DTO 停在 adapter，核心只消费规范化 segment。
- 模型 manifest 固定 `id/file_name/size/sha256/official_url`；镜像只替换 base URL。
- FFmpeg adapter 返回媒体元信息和临时 WAV parts，不泄漏 process handle。
- 系统文件选择、剪贴板和打开目录停留在前端/系统 adapter，不进入纯组件。

## 模式决策与拒绝方案

### 使用 Port/Adapter（Hexagonal 边界）

问题：FFmpeg、whisper、模型 HTTP、B 站字幕、文件系统和 Tauri 事件是变化快且难以单测的外部依赖。使用 traits 隔离可替换性和失败注入。直接在 manager 调进程/网络会导致取消、清理和测试不可控。

### 使用 State Machine

问题：任务阶段、取消、重试和终态并发更新有真实变化轴。集中转移表防止散落 match 规则。无需引入通用工作流引擎。

### 使用 Facade

`TranscriptionManager` 为 commands 提供小而诚实的用例入口，隐藏 scheduler/store/ports 协作。它不是万能 manager；导出、状态机和 adapters 保持独立。

### 使用展示 Adapter

任务页需要联合展示不兼容的下载/转写合同。用 `toTaskListItem` 将领域实体映射到 UI union，拒绝扩充 `DownloadTask` 为大量 nullable 字段。

### 拒绝方案

- 拒绝 `whisper-rs` 进程内绑定：增加 native linking、崩溃隔离和跨平台构建复杂度，与已批准 sidecar 冲突。
- 拒绝把转写加入 `StrategyRegistry`：该 registry 选择下载模式，ASR 是独立队列和资源策略。
- 拒绝通用“任务引擎”大重构：当前只有两个不同领域，没有足够证据承担迁移风险。
- 拒绝前端轮询作为主要进度机制：复用事件+快照恢复模式。
- 拒绝为五种格式建立类工厂层：纯函数按 enum 分派足够清晰。

## 可读性与维护护栏

- 一个模块只拥有一种副作用类别；命令、查询和纯转换分离。
- 所有状态转移由 `state_machine` 唯一拥有；UI action policy 只映射可用操作。
- 所有临时文件清理由 cleanup guard 唯一拥有。
- 所有模型有效性由 repository/manifest 唯一拥有。
- 页面文件不直接解析 IPC、CLI 或远程 DTO。
- 不创建只有一个空实现的 factory/handler 层；traits 仅用于真实外部依赖和测试替换。
- 复杂 pipeline 通过命名步骤保持同一抽象层，低层参数构建和解析放入 adapter。
- 新增字段必须有默认值、序列化测试和前后端合同测试。

## 架构风险

- whisper CLI 的实时进度输出可能不稳定：adapter 应把 stderr 解析视为尽力而为，最终状态以退出码和结果文件为准。
- 长音频分段与静音对齐复杂：先由 FFmpeg 生成可控分段，合并时统一时间偏移；算法必须有 fixture 测试。
- 下载活跃度与转写优先级存在跨 scheduler 协作：只读 probe 防止相互持锁。
- 模型下载和转写使用同一模型目录：下载原子 rename 和 active usage 锁必须避免读到半文件。
- 结果编辑会产生 revision 冲突：保存命令使用 expected revision 并返回最新结果。

## 未决架构问题

- 平台字幕 raw DTO 在取得 fixture 后固化；port 已足以隔离变化。
- GPU capability 参数以选定 whisper v1.8.6 的实际 CLI help 为准；不影响 CPU 主合同。
- 大结果列表是否虚拟化在实现期用 DOM/滚动基准决定，不改变 store/contract 边界。
