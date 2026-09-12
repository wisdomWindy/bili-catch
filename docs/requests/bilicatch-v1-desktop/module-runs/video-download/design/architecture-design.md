# 架构设计：视频下载与音视频合并

## Delivery unit identifier

`video-download`

## Architecture objective

在不改变已验收audio/task/auth/settings合同的前提下，为 `video-audio` 与 `video-only` 提供fresh DASH source、单/双轨断点下载、聚合进度、取消/暂停确认、FFmpeg无损封装和原子输出。视频模块复用真实变化轴上的基础能力，但不把音频专用profile、metadata转换或workspace路径硬塞进通用接口。

## Architecture scope and triggers

- 触发：runner认领 `video-audio` 或 `video-only` 任务。
- 用户可见范围：复用 `/download` 的quality/codec控件与 `/tasks` 的既有状态/操作；无新路由、无新页面设计。
- 包含：视频能力配对、fresh source、双轨协调、独立checkpoint、聚合progress、MP4无损mux、仅视频原子输出、失败恢复和production composition。
- 不包含：转码、字幕、HDR色彩转换、播放列表导入、直播、sidecar打包/签名、托盘/通知/更新。

## Upstream inputs and assumptions

- `DownloadTaskDraft` 已固定 `mode/qualityId/codec`；视频模式不得携带audio output profile。
- `TaskManager` 是attempt、状态、cancel intent、retry和最终输出的唯一owner；video executor只能通过reporter回写。
- `DownloadRuntimeRunner` 已具备supported-mode认领与control forwarding，但需从单executor升级为有序strategy registry。
- B站raw DASH video目前只读 `id/codecs`，尚缺URL、bandwidth、mime、尺寸/帧率等source字段。
- 默认输出容器为MP4。`video-only` 保存原始fragmented MP4视频轨并以 `.mp4` 原子落盘，不调用FFmpeg；`video-audio` 以stream copy mux为MP4。
- 合并用音频选择当前账号有权访问的最高普通AAC/M4A源，不请求FLAC/HiRes，不复用独立音频输出profile。

## Module boundary design

### Frontend

- `download-center/video-options`：拥有quality与codec可选组合规则，禁止页面或组件自己计算笛卡尔积。
- `download-center/store`：拥有当前variant选择、默认quality回退、auth变化协调和draft生成。
- `DownloadOptions.vue`：只渲染typed options并emit选择；不理解B站codec字符串或账号策略。
- 前端不接收DASH URL、bandwidth、checkpoint或FFmpeg参数。

### Rust service

- `services/video/source`：定义稳定video source bundle、选择规则和 `VideoSourcePort`。
- `services/video/executor`：线性编排source、workspace、单/双轨下载、processing/finalize/cleanup。
- `services/video/progress`：聚合双轨bytes/total/speed/ETA，并保证0..90单调。
- `services/video/mux`：定义MP4 stream-copy request/outcome port，不暴露进程实现。
- `services/download_runtime/registry`：按mode路由到audio/video executor；不复制runner循环。

### Rust infrastructure

- `infrastructure/bilibili/video_source`：把raw playurl适配为稳定source bundle，并执行quality/codec/auth能力校验与URL allowlist。
- `infrastructure/download`：把现有audio source输入泛化为带 `MediaKind` 的可信媒体字节请求；Range/redirect/checkpoint算法保持一个owner。
- `infrastructure/video/workspace`：管理video/audio双轨、双checkpoint、processed输出和原子finalize；路径安全原语与audio workspace共享。
- `infrastructure/video/ffmpeg_mux`：复用可信locator/process runner，生成固定stream-copy argv并验证非空输出。

## File and directory structure

```text
src/features/download-center/
  video-options.ts
  video-options.test.ts
  contracts.ts                 # 增加videoVariants稳定字段
  store.ts                     # 选择/回退/draft owner
  components/DownloadOptions.vue

src-tauri/src/models/
  parse.rs                     # VideoVariant DTO
  task.rs                      # 复用既有qualityId/codec

src-tauri/src/services/
  download/
    source.rs                  # MediaKind/MediaSourceCandidate
  download_runtime/
    registry.rs                # audio/video Strategy路由
  video/
    mod.rs
    source.rs
    progress.rs
    mux.rs
    executor.rs
    runtime.rs

src-tauri/src/infrastructure/
  bilibili/
    raw.rs                     # 完整RawVideoStream私有字段
    video_source.rs
  download/                    # 复用HTTP byte算法
  workspace/
    safe_paths.rs              # containment/collision公共原语
  video/
    workspace.rs
    ffmpeg_mux.rs
```

允许根据实现期实际依赖把 `services/download/source.rs` 保留在现有audio/download模块中，但稳定类型必须脱离 `AudioSourceCandidate` 命名；不得通过type alias掩盖仍然音频专用的MIME规则。

## Code relationship and dependency direction

```text
DownloadPage -> download-center store -> video-options -> stable ParseVideoResult
                                             |
                                             v
                                    DownloadTaskDraft

lib.rs -> runner -> executor registry -> VideoExecutor -> VideoSourcePort
                                      |       |-> ByteDownloaderPort x 1/2
                                      |       |-> VideoWorkspacePort
                                      |       |-> VideoMuxerPort (video-audio only)
                                      |       `-> ExecutionReporterPort -> TaskManager
                                      `-> AudioExecutor (unchanged behavior)

VideoSourceAdapter -> Parser/Auth/WBI/Bilibili raw
HTTP/Workspace/FFmpeg adapters -> OS/network/process side effects
```

依赖只从composition/service指向ports，再由infrastructure实现；B站raw、reqwest、filesystem和Tokio Command不得进入models、TaskManager或Vue。

## Responsibility split

- capability adapter回答“哪些quality+codec组合当前存在/需登录”；source selector回答“本attempt选择哪条精确轨道”。
- executor决定阶段顺序，不解释HTTP状态、不拼FFmpeg argv、不直接写task store。
- downloader只处理一个source到一个track path；双轨并行与聚合属于video executor/progress。
- workspace唯一创建/验证/删除路径；downloader和mux只接收已验证路径。
- muxer只做 `-c copy` MP4封装，不承担下载、转码或任务状态。
- TaskManager继续处理cancel优先于迟到Completed，video不得另建第二套状态机。

## Function design and public entrypoints

- `adapt_video_variants(play, auth) -> Vec<VideoVariant>`：从raw streams生成稳定且去重的quality/codec配对。
- `select_video_source(request, candidates) -> Result<VideoSourceBundle, AppError>`：精确匹配quality+codec，执行权限/可用性错误分类。
- `resolve_video_source(request)`：每attempt fresh验证auth/view/playurl，并返回video与可选普通audio source。
- `prepare_video_workspace(request)`：为video/audio/processed和各自checkpoint生成contained路径。
- `download_tracks(bundle, controls, progress)`：仅视频单轨；视频+音频用两个future并行，任一pause/cancel时传播给另一轨并等待确认。
- `aggregate_track_progress(track, snapshot)`：按下载bytes求和；总长均已知时计算0..90，否则保持单调并汇总speed。
- `mux_video_audio(request, control)`：固定两输入、map video/audio、`-c copy`、faststart、无shell。
- `finalize_video(paths, mode)`：仅视频finalize video轨；双轨finalize processed；非空且不覆盖。
- `TaskExecutorRegistry::for_mode(mode)`：返回唯一strategy；没有strategy的mode保持queued。

## State ownership and data flow

1. 前端从 `videoVariants` 选择有效pair并创建既有draft；不把raw source写入任务。
2. runner按mode从registry选择VideoExecutor，TaskManager签发不可变spec与attempt id。
3. executor fresh resolve并注册workspace临时路径。
4. `video-only` 下载video；`video-audio` 并发下载video与普通audio。每轨独立partial/checkpoint/source identity。
5. 聚合器向TaskManager报告0..90；pause只有在所有活跃writer停下并刷checkpoint后确认。
6. 双轨完成后报告Processing并调用mux；仅视频跳过Processing或按规格决定直接finalize，最终spec必须冻结可观察状态。
7. 非空原子finalize后上报Completed；CancelRequested若与Completed竞争，沿用TaskManager取消优先规则。
8. success/cancel清全workspace；E007/E008/E009保留兼容track/checkpoint，删除processed；retry重新解析source并逐轨校验checkpoint identity。

## Data structures and type strategy

- `VideoVariant { quality_id: String, quality_label: String, codec: VideoCodec, requires_login: bool }` 是Rust serde权威合同；TS保留camelCase镜像，不按本地偏好重命名。
- `MediaKind = Audio | Video` 与 `MediaSourceCandidate { id, kind, primary_url, backup_urls, mime_type, codecs, bandwidth, content_length, etag }` 属于Rust进程内，不序列化到WebView/task/event/checkpoint。
- `VideoSourceBundle { video, audio: Option<MediaSourceCandidate> }`：audio只在video-audio存在；metadata不承担音频标签写入。
- `VideoWorkspacePaths` 显式列出video/audio/processed及两个checkpoint；不用字符串key map，避免路径混淆。
- `TrackProgress` 以track enum索引；所有跨IPC u64 bytes继续序列化为十进制字符串。
- checkpoint保持versioned、无URL/credential，并加入track kind/source identity；旧audio V1读取合同不破坏。

## Contract and adapter boundaries

- 权威公开合同：Rust `ParseVideoResult/VideoVariant` 与 `DownloadTaskDraft` serde；TS只做等义镜像。
- B站raw contract只在infrastructure，支持snake/camel URL字段；redirect后的每一跳重新验证HTTPS host。
- UI错误只消费稳定AppError code的本地化文案，不显示backend message/details/path/URL/stderr。
- FFmpeg executable路径只来自Tauri resource dir；WebView不能指定可执行文件或媒体URL。
- WebView capability保持无shell、无filesystem、无HTTP；不因视频模块扩权。

## Pattern decisions and rejected alternatives

- 使用Strategy registry：真实变化轴已从一个audio executor增长为audio/video两个mode strategy；直接在runner写mode条件会把控制转发和执行细节耦合。若未来仍只有两个固定实现，registry保持简单Vec/array，不建插件系统。
- 使用Adapter：B站raw、HTTP、filesystem、FFmpeg均是不稳定或副作用边界；稳定ports便于fixture/loopback/fake验证。
- 使用Composite progress（轻量聚合对象）：双轨并发需要把两个进度合为一个任务快照；不建立通用事件总线。
- 拒绝每codec一个executor、通用媒体pipeline DSL、动态provider插件、前端DASH URL选择、shell command字符串和无依据的并行Range。

## Readability and maintenance guardrails

- 新生产文件约250行进入职责审查，超过350行必须拆分；测试fixture可按场景分组并说明。
- audio已验收行为必须由全量回归保护；通用化只能下移真正共享的source bytes/path/process runner，不得弱化audio MIME/tier/profile guard。
- quality/codec配对只有 `videoVariants` 一个公开owner；任务校验与source选择必须拒绝非法pair，不能靠UI正确性。
- 双轨协调函数保持显式阶段与结果枚举，禁止深层callback、detached writer或持锁await。
- 所有失败出口写清“保留哪些轨、删除哪些输出、TaskStatus如何变化”。

## Architecture risks

- B站video raw字段、quality/HDR标签和codec字符串可能变化；需以fixture + opt-in public smoke隔离，不把raw字段扩散到公开合同。
- DASH video-only的fragmented MP4兼容性依播放器而异；若真实smoke证明必须remux，则规格需明确“仅视频不合并”是否允许无转码remux，不能在execute阶段暗改。
- 双轨并发可能把单任务连接预算翻倍；架构要求总预算分配给两轨，不能每轨各使用完整settings值。
- 一轨完成另一轨失败时需要逐轨恢复；错误清理若按任务根目录粗删会破坏断点，必须测试。
- 真实FFmpeg sidecar仍缺失；video模块只能完成adapter/fake/skip证据，system-release负责随包binary E2E。

## Open architecture questions

- 默认采用MP4作为两种视频模式的输出容器；若后续fixture出现非MP4 DASH视频源，规格应拒绝该variant还是允许无转码容器适配，需要在spec澄清。
- HDR/杜比视界标识是否只作为quality label保留，还是需要独立稳定字段；V1 UI没有额外控件，默认不新增字段，按source variant label展示。
- `video-only` 是否必须经过FFmpeg无转码remux以提高播放器兼容性；当前按PRD“只下载视频流、不调用合并”设计为直接finalize，需在spec固定。
- 双轨连接预算为1时无法同时各分配一个连接；默认仍并行两个单连接HTTP请求并把settings解释为每轨上限，还是将总预算最小值提升为2，需在spec固定语义。
