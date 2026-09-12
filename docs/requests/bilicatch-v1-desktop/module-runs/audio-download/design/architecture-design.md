# 架构设计：音频下载执行器

## 交付单元标识

`audio-download`

## 架构目标

把已经入队的 `audio-only` 任务变成可暂停、恢复、取消、重试并最终产生可信音频文件的后台作业。Rust 负责刷新媒体地址、选择当前认证上下文允许的音频源、可恢复下载、格式处理、元数据写入、原子落盘和临时文件生命周期；现有 Vue 下载中心只补齐输出格式/码率能力，任务页继续消费统一任务状态，不建立第二套执行状态。

该交付还要把 task-management 预留的 deferred executor 替换成真实但可扩展的运行宿主，使后续 `video-download` 能注册自己的执行策略而不复制调度循环、控制协议或进度回写逻辑。

## 架构范围与触发因素

- 当前范围：MP3 128/192/320K、M4A 原始音频、FLAC 无损输出；授权能力检查；音频源刷新；HTTP Range 恢复；临时工作区；FFmpeg copy/encode 与标题、UP 主、封面元数据；E007/E008/E009；进度、暂停、恢复、取消、自动/手动重试和成功清理。
- UI 范围：复用 `/` 下载选项与 `/tasks` 状态/操作，只调整音频选项语义、登录限制和错误文案；不新增路由、页面、卡片或独立音频队列。
- 当前不实现：视频轨下载、音视频合并、字幕/弹幕、通知、托盘、FFmpeg 二进制打包和更新安装；后四项仍由 `video-download`/`system-release` 承担。
- 触发因素：媒体 URL 有时效性，必须在每次 attempt 开始时刷新；下载可中断且需跨应用重启恢复；FFmpeg 是可取消子进程；任务管理器是状态唯一权威。

## 上游输入与假设

- `DownloadTask` 已持久化 `bvid/cid/page/outputDir/audioFormat/audioBitrateId`，元数据在执行时按 `bvid` 重新获取，不把过期流 URL 或 cookie 写入任务 JSON。
- `AuthContextProvider` 是认证事实唯一来源；执行器只接收当前已验证 context，不读取系统凭据存储。
- SettingsManager 提供当前临时目录和连接数；单个 attempt 固化读取到的值，运行中修改设置只影响后续 attempt。
- MP3 的 128/192/320K 是输出编码码率，不代表或解锁同码率源流。匿名源能力上限 64K，已认证源能力最高 192K/Hi-Res；转码不能提升源质量。
- FLAC 仅在已认证且 playurl 明确返回可用无损/Hi-Res 源时可选、可执行；不把普通有损源升采样并标为无损。
- M4A 采用音频 stream copy/remux 以保持源码率；为写入封面和文本元数据，仍通过 FFmpeg 容器重封装，但不重新编码音频。
- 音频执行代码与 FFmpeg 命令适配器在本模块完成；实际 FFmpeg 7.x sidecar 文件和打包清单由 `system-release` 提供。开发/测试可通过受控 locator 注入可执行文件；缺失时稳定失败为 E008。

## 模块边界设计

### Rust 服务层

- `services/download_runtime/runner.rs`：唯一后台协调循环。处理 pending control、按 TaskManager 空闲槽认领任务、按 `DownloadMode` 分派 executor；自身不下载字节。
- `services/download_runtime/ports.rs`：`DownloadExecutorPort`、`ExecutionReporterPort` 与 dispatcher 契约。`video-download` 后续实现同一端口。
- `services/audio/executor.rs`：音频用例编排，严格执行 resolve -> workspace -> download -> process -> finalize -> cleanup，并把所有状态通过 reporter 回写。
- `services/audio/source.rs`：源能力选择纯函数，区分输出 profile 与 B 站源 profile，验证匿名/登录/FLAC规则。
- `services/audio/ffmpeg_args.rs`：按 MP3/M4A/FLAC 生成无 shell argv 的纯函数，负责元数据与封面映射。
- `services/audio/ports.rs`：媒体源、字节下载、工作区、处理器、时钟/采样器等真实变化轴，不暴露 Tauri。
- `services/tasks`：扩展 `TaskExecutionSpec` 为当前 attempt 的不可变完整任务快照与临时目录；增加工作区登记、取消确认和控制查询。仍是任务状态与持久化唯一权威。

### Rust 基础设施层

- `infrastructure/bilibili/audio_source.rs`：复用现有 WBI/client/auth 规则，获取 view + playurl，适配标题、UP 主、封面与可下载音频流；URL 只保存在内存。
- `infrastructure/download/http_downloader.rs`：受限 HTTPS、Range、分段/顺序恢复、长度校验和进度采样；不理解任务状态或音频格式。
- `infrastructure/audio/workspace.rs`：在 settings 临时目录创建 task/attempt 作用域工作区，管理 source、cover、processed、checkpoint 与最终文件的原子移动。
- `infrastructure/audio/ffmpeg.rs`：直接 spawn 可执行文件并传 argv；采集受限 stderr，支持终止，映射 E008，不拼 shell 字符串。
- `infrastructure/audio/ffmpeg_locator.rs`：只接受已验证的资源/开发注入路径；不扫描任意 PATH，不让 WebView 提交可执行文件路径。

### 前端

- `features/download-center/audio-options.ts`：输出格式对应的稳定 profile：MP3 128/192/320K、M4A original、FLAC lossless，并集中决定 requiresLogin/sourceSupported。
- `features/download-center/store.ts`：切换格式时规范化 `audioBitrateId`；登录状态或解析能力变化时回退到首个合法组合。
- `features/download-center/components/DownloadOptions.vue`：继续使用原有 select；禁用原因和输出/源质量区别用既有表单说明呈现，不展示 raw stream id。
- `features/task-management`：仅增加 E007/E008/E009 的本地化呈现和必要状态测试；不新增执行状态或直接控制 FFmpeg。

## 文件与目录结构

```text
src/
  features/download-center/
    audio-options.ts
    store.ts
    components/DownloadOptions.vue
  features/task-management/
    error-presenter.ts

src-tauri/src/
  services/
    download_runtime/
      mod.rs
      ports.rs
      runner.rs
      dispatcher.rs
    audio/
      mod.rs
      ports.rs
      executor.rs
      source.rs
      ffmpeg_args.rs
  infrastructure/
    bilibili/audio_source.rs
    download/http_downloader.rs
    audio/workspace.rs
    audio/ffmpeg.rs
    audio/ffmpeg_locator.rs
  services/tasks/
    ports.rs
    manager.rs
    mutations.rs
```

测试优先放在相邻 `#[cfg(test)]` 和现有前端测试目录；跨层 Rust 场景放 `src-tauri/tests/audio_executor.rs`，HTTP Range fixture 使用仅绑定 loopback 的测试服务器，不访问真实 B 站。

## 代码关系与依赖方向

```text
Tauri setup
  -> DownloadRuntimeRunner
      -> TaskManager (claim/control/update authority)
      -> DownloadExecutorDispatcher
          -> AudioExecutor
              -> AudioSourcePort -> BilibiliAudioSource -> Bilibili client/auth
              -> ByteDownloaderPort -> ReqwestRangeDownloader
              -> AudioWorkspacePort -> FileAudioWorkspace
              -> MediaProcessorPort -> FfmpegProcessAdapter

DownloadPage -> download-center store -> audio-options -> existing task draft handoff
TasksPage -> task-management service/store -> existing task IPC/events
```

- domain/service modules不依赖 Tauri command、Vue、系统 credential store 或 shell plugin。
- downloader 不直接回写 TaskManager；AudioExecutor 通过 `ExecutionReporterPort` 报告 attempt-scoped update。
- executor 不持有用户 cookie 字符串的可序列化副本，不在 error、日志、checkpoint 或 event 中包含 URL query/cookie。
- 后续 video executor 可复用 runtime、downloader、workspace 与 FFmpeg process adapter，但不得向 audio service 添加视频分支。

## 职责切分

| 层 | 拥有 | 禁止 |
| --- | --- | --- |
| DownloadRuntimeRunner | 认领、分派、控制转送、任务生命周期 | 媒体选择、文件写入、状态直接修改 |
| TaskManager | 权威 task/attempt/control、持久化、事件 | HTTP、FFmpeg、长时间 await |
| AudioExecutor | 单次音频 attempt 编排与错误分类 | 直接写任务库、读 UI 状态 |
| Source adapter | fresh metadata/stream capability | 输出编码决策、持久化 cookie/URL |
| Downloader | Range/checkpoint/bytes/progress | 任务状态机、格式转换 |
| Workspace | 可信路径、原子 finalize、幂等 cleanup | 网络与媒体业务 |
| FFmpeg adapter | process start/exit/cancel/stderr 限制 | shell 解释、任意用户参数 |
| Vue store/UI | 合法输出选项与意图提交 | 源 URL、FFmpeg、伪造进度 |

## 函数设计与公开入口

### 运行时与任务端口

- `DownloadRuntimeRunner::run()`：应用生命周期后台循环；空闲时使用有上限的通知/定时等待，不 busy-spin。
- `TaskManager::claim_next() -> Option<TaskExecutionSpec>`：返回 `task` 快照、`attemptId`、`connectionCount`、`temporaryDirectory`；仅在 executor 支持该 mode 时认领。
- `TaskManager::pending_controls() -> Vec<ExecutionControlSpec>`：只返回当前 attempt 的 pause/cancel request。
- `TaskManager::apply_execution_update(task_id, attempt_id, update)`：扩展 `WorkspacePrepared`、`Cancelled`，继续拒绝旧 attempt 与非单调进度。
- `DownloadExecutorPort::supports(mode)`、`start(spec)`、`pause(taskId, attemptId)`、`cancel(taskId, attemptId)`：runner 唯一调用面；start 必须快速启动受控异步作业，不占住调度循环。

### 音频执行

- `AudioExecutor::execute(spec, cancellation)`：单 attempt 编排入口。
- `AudioSourcePort::resolve(bvid, cid, requested, auth) -> AudioSourceBundle`：返回 fresh source/backup URL、可选长度、container/codec、元数据和 source tier。
- `select_audio_source(streams, outputProfile, authState)`：确定满足权限与 FLAC真实性的源；不可用返回 E005/E006，而不是静默降级到伪无损。
- `ByteDownloaderPort::download(request, checkpoint, control, progress) -> DownloadOutcome`：支持 resumed offset、Range 206/200 回退、暂停/取消和网络错误分类。
- `MediaProcessorPort::process(AudioProcessRequest, control) -> ProcessOutcome`：M4A copy、MP3/FLAC encode及 metadata；非零退出/不支持 codec/封面失败映射 E008。
- `AudioWorkspacePort::prepare/finalize/cleanup`：所有路径基于可信 root 与 task id 派生；finalize 使用同目录临时输出 + rename，禁止覆盖现有文件并使用确定性去重名。

## 状态归属与数据流

1. 下载中心按解析能力、认证快照和输出格式生成合法 `audio-only` 草稿。`audioBitrateId` 表示输出 profile：`128|192|320|source|lossless`。
2. TaskManager 持久化 queued 任务；runner 只在 audio executor available 且有并发槽时认领，创建新的 attempt 并转为 downloading。
3. AudioExecutor 获取当前 validated auth，再按 bvid/cid 请求 fresh view/playurl；选择源并下载封面。源 URL、cookie 和封面 URL不进入持久任务或事件。
4. Workspace 根据任务稳定目录加载 checkpoint；downloader 校验源 identity/长度/ETag（存在时），从已完成 offset 恢复。服务端忽略 Range 时清空不兼容 partial 后从 0 开始。
5. 每个有效 progress 通过 reporter 回写；TaskManager 保持 attempt/单调性/合并规则。pause 请求使 downloader 停在完整写入边界、flush checkpoint 后回报 Paused；重启后 queued attempt 读取同一 checkpoint。
6. 下载完成后回报 Processing。FFmpeg 使用临时 processed 文件：M4A stream copy；MP3 指定 128/192/320K；FLAC 只接受 lossless/Hi-Res source。标题、artist=UP 主和 cover 作为显式 argv 输入。
7. FFmpeg 成功且输出非空后，workspace 原子移动到 outputDir，回报 Completed，然后幂等删除 source/cover/checkpoint/processed 临时项。输出文件永不列入临时清理集合。
8. cancel 对 queued/paused 可同步清理并取消；对 downloading/processing 只先记录 cancelRequested，runner 请求 executor 停止，子进程/下载流确认退出并清理后才回报 Cancelled。清理失败进入 Failed，不谎报取消成功。
9. 网络中断回报 E009/NetworkFailure，沿用 1/2/4 秒最多三次自动重试并保留兼容 checkpoint；E007/E008 直接 Failed，保留输入与重试参数，手动 Retry 创建新 attempt。重试前删除不完整 processed 文件，不删除可复用 source partial。

## 数据结构与类型策略

```text
AudioOutputProfile
  Mp3 { bitrateKbps: 128 | 192 | 320 }
  M4aOriginal
  FlacLossless

AudioSource
  primaryUrl, backupUrls[]       // in-memory secret-like transport data
  sourceId, bandwidth, codec, container, tier
  contentLength?, etag?

AudioMetadata
  title, uploader, coverUrl

AudioCheckpoint (internal JSON)
  schemaVersion=1, taskId, sourceIdentityHash
  completedBytes, totalBytes?, etag?

TaskExecutionSpec
  task, attemptId, connectionCount, temporaryDirectory
```

- 跨前后端 `audioBitrateId` 保留 string，避免迁移已持久任务；validation 按 format 约束组合，不接受任意字符串。
- 下载 byte/offset 使用 `u64`，事件继续以十进制字符串跨 JS；百分比只在 total 已知时计算并限制 0-99，finalize 后由 manager 置 100。
- checkpoint 不保存 URL、cookie、Authorization 或完整 API response；`sourceIdentityHash` 由不含凭据的 bvid/cid/sourceId/长度构成。
- output filename 继续使用 task-management 已批准的净化名；冲突采用 `name (1).ext` 的最小可用序号并在 workspace 内原子确定。
- 临时工作区名只使用内部安全 task hash/attempt，不直接拼接标题或用户路径片段。

## 契约与 Adapter 边界

- `AudioSourcePort` 是 B 站私有 response 到稳定执行合同的唯一 adapter。Raw DASH 扩展支持 `id/baseUrl/backupUrl/bandwidth/codecs/mimeType` 以及 FLAC/Hi-Res 分支，但 raw 类型不得流入 service/model/TS。
- `ByteDownloaderPort` 隔离 HTTP 与本地 loopback fake；只接受 source bundle 生成的已验证 HTTPS URL，redirect 每跳重新校验允许 host/scheme。
- `MediaProcessorPort` 隔离 FFmpeg binary/process；参数由纯函数生成，adapter 不接收前端自由参数或 shell string。
- `ExecutionReporterPort` 隔离 audio executor 与 TaskManager；所有 update 带 attemptId，runner/executor 不能 emit Tauri event。
- `AudioWorkspacePort` 隔离路径/磁盘；在写入前检查可用空间或写错误，disk-full 统一映射 E007；不把任意 path 删除能力暴露给 WebView。
- `DownloadExecutorDispatcher` 是 audio/video 的窄 Strategy 选择点；未知/尚未实现的 mode 保持 queued，不以失败消耗重试。

## Pattern 决策与拒绝项

- **Strategy**：audio 与后续 video 的执行流程实质不同，由 dispatcher 按 mode 选择 `DownloadExecutorPort`。只建立一个分派点，不为 MP3/M4A/FLAC各建 class；三种格式用数据 profile + 纯 argv builder 表达。
- **Adapter/Port**：B 站 source、HTTP、文件工作区和 FFmpeg 都是可失败外部边界，分别需要 deterministic fake。端口保持用例所需的窄方法，不建立通用 downloader/repository/event bus。
- **State/Command**：沿用 TaskManager 的权威状态机和 `TaskAction`；新增 active cancellation acknowledgement，避免“状态已取消但进程仍写文件”。不在 AudioExecutor 内复制状态枚举。
- **Observer**：沿用 reporter -> manager -> `download://progress`，进度回调只传值对象；不新增音频专属事件。
- 拒绝 WebView 直接请求媒体 URL、前端启动 FFmpeg、持久化签名 URL、使用 shell command 字符串、把 PATH 中任意 ffmpeg 当可信程序，以及为未来格式建立插件框架。

## 可读性与维护护栏

- `AudioExecutor::execute` 用线性阶段函数，每段最多一个外部 await 边界和明确 cleanup owner；错误映射集中在 adapter，不在编排层匹配错误字符串。
- 源选择、输出 profile validation、FFmpeg argv 与 retry classification 均为纯函数并使用表驱动测试。
- runner 不持 TaskManager 锁 await；同一 task/attempt 只能有一个 active job handle，完成时必须移除。
- 子进程 argv 每个值单独传递；不记录完整 argv 中的 URL/path，不记录 stderr 全文，错误 details 只使用稳定标识。
- 临时目录清理前必须验证 canonical path 位于当前配置的临时 root/task workspace 下；不得对 root、outputDir 或未登记路径递归删除。
- 不改变已有 task IPC/event 名称和公开状态集合；必要扩展只发生在 Rust internal persisted record，保持 schema migration 明确。

## 错误与恢复规则

- E007：空间预检不足、写入 ENOSPC 或 finalize 因空间失败。停止写入，保留可识别 checkpoint/source partial，清除无效 processed 文件；手动释放空间后 Retry。
- E008：FFmpeg 不存在、无法启动、非零退出、不支持目标编码/封面写入或产生空输出。保留已下载 source 供 Retry，删除 processed 临时文件；不自动重试。
- E009：连接/读取超时、连接重置、可恢复 HTTP 中断。保留完整分块/offset，最多自动重试三次；权限、4xx、源消失不是 E009。
- E005/E006：需要登录或当前账号无 FLAC/源权限。执行前失败且不自动重试；登录/重新解析后用户可新建或 Retry，executor 必须重新校验。
- pause/cancel 不是 error。pause 保留 checkpoint；cancel 仅在停止所有 writer/process 并完成 cleanup 后进入 cancelled。

## 验证策略

- 纯单测：输出 profile matrix、source selection、匿名/登录限制、FFmpeg argv、文件名冲突、error mapping、control race、旧 attempt 忽略。
- 端口契约测试：loopback HTTP 覆盖 206 resume、200 Range fallback、长度不匹配、中断和 redirect allowlist；fake process 覆盖成功/失败/取消/空输出。
- 用例集成：fake source/downloader/workspace/processor 验证完整阶段、MP3/M4A/FLAC、metadata、E007/E008/E009、pause/resume/cancel cleanup、automatic retry 与 manual retry reuse。
- 持久恢复：应用重启后 downloading/processing -> queued，checkpoint 保留且下一 attempt 恢复；完成/cancel 后工作区为空。
- 前端：格式切换规范化、登录限制、草稿 exact payload、中英文错误、现有下载/任务页视觉和键盘回归。
- 可选 real FFmpeg 7 smoke 只在受控 binary 可用时运行；sidecar 打包 E2E 在 `system-release` 最终门禁完成，不能用 smoke 缺失掩盖单元/集成失败。

## 架构风险

- B 站 playurl 的 DASH 音频/FLAC字段可能变化；raw adapter 必须宽容未知字段，但 stable source contract 与权限失败必须严格。
- 多连接 Range 只有服务端稳定支持长度/范围时才能启用；不满足条件退化为单连接，不把并发数当正确性前提。
- Windows 正在被占用的文件无法 rename/delete；必须先等待 downloader/FFmpeg 句柄退出，再 finalize/cleanup，并测试取消竞态。
- FFmpeg 封面嵌入对容器支持存在平台差异；本模块用 FFmpeg 7.x argv fixture 和可选 smoke 验证，若目标容器明确不支持则 E008，不静默产出缺元数据文件。
- system-release 尚未提供 sidecar 时，真实生产音频任务会以可解释 E008 失败；当前规格/验证必须明确这是跨模块依赖，不误报打包可用。

## 开放架构问题与已选默认

- MP3 320K 与源上限的冲突：已选“输出编码码率与源质量分离”；UI/错误不得暗示 320K 输出提升了源质量。
- failure 临时文件：已选保留可恢复 source/checkpoint、删除不完整 processed；success/cancel 全清。这样同时满足断点恢复与取消清理。
- M4A 元数据：已选 FFmpeg `-c:a copy` 重封装而非裸文件直接 rename；保持原始音频数据并满足 metadata。
- executor 接入范围：本模块交付共享 runner/dispatcher 和 audio strategy，video mode 在下个模块注册；未支持 mode 保持 queued，不进入错误重试。
