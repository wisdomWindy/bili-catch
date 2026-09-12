# 执行记录：视频下载与音视频合并

## 执行约束

- 规格批准：是；计划批准：是；当前阶段：execute。
- 固定顺序：VID-01 -> VID-08，单agent串行。
- TDD：可测试行为先写red、确认预期失败，再做最小green与重构。
- 当前目录无Git元数据，以task-board与本记录作为检查点。
- 所有shell命令以 `rtk` 开头；Rust使用 `C:/Users/yangjianlin/.cargo/bin/cargo.exe`。

## VID-01

- 状态：completed。
- 目标：冻结VideoVariant跨端合同、quality/codec选择、draft与MP4命名规则。
- Red：前端纯规则测试因缺失 `video-options` 失败，组件因未过滤variant失败；Rust因缺失 `VideoVariant`/`video_variants` 编译失败；空清晰度任务规则测试失败。
- Green：新增Rust/TypeScript `VideoVariant` 镜像，由B站adapter按清晰度与AVC/HEVC/AV1顺序生成真实组合；store、组件和draft门禁统一复用pair规则；视频任务继续固定 `.mp4`，并拒绝空白/非数字清晰度。
- Refactor：删除旧 `preferredOption/firstAvailable` 死代码；保留 `qualities/codecs` 兼容投影，但不再用其生成笛卡尔组合。
- 定向门禁：前端4文件33项通过；Rust parse/task 8项与adapter 3项通过。
- 全量门禁：前端46文件175项通过，`npm run build`通过；Rust `fmt --check`、`check --all-targets -j 1`通过，66单元+69集成通过，1项公网opt-in忽略。

## VID-02

- 状态：completed。
- 目标：将音频专用字节传输边界收敛为带媒体类型与轨道身份的通用Source/Request，同时保持既有音频Range、redirect、预算与控制语义。
- Red：新媒体合同测试因 `services::download` 不存在而失败；旧测试夹具因共享类型尚未迁移而无法编译。
- Green：新增 `MediaKind`、`MediaSourceCandidate`、`MediaDownloadRequest` 与共享控制/进度端口；HTTP downloader 支持 `audio/*` 与严格 `video/mp4`，交叉kind/MIME、非MP4视频在请求前拒绝；source identity 纳入kind/MIME/codec/长度且排除临时URL。
- Refactor：删除音频专用 `services/audio/download.rs`，音频executor/source、cover downloader、progress和HTTP infrastructure统一依赖 `services/download`；保留音频tier/profile guard与既有Range/redirect/checkpoint算法。
- 定向门禁：媒体合同2项、共享HTTP 11项、音频source 5项、音频executor 8项、音频download contract 2项全部通过。
- 全量门禁：Rust `fmt --check`通过；`cargo test --all-targets -j 1`通过，68库测试中67通过、1项公网opt-in忽略，全部集成测试通过。

## VID-03

- 状态：completed。
- 目标：实现B站每次attempt fresh video source解析、variant精确匹配、权限与URL安全选择，并复用音频刷新语义。
- Red：视频source selector合同因 `services::video` 不存在而失败。
- Green：新增 `VideoSourcePort/Request/Bundle`、raw video URL/MIME/bandwidth字段、B站 video adapter；执行期按quality+codec精确匹配，匿名高于480P返回E003，非MP4视频返回E004，video-audio只选择普通AAC/M4A。
- Refactor：复用现有WBI/auth/view/playurl fresh链路和媒体URL allowlist；source URL、cookie和raw结构仍只停留在attempt内存，不进入任务或前端合同。
- 定向门禁：video source合同2项、adapter 1项、parser fresh resolve 1项通过。
- 全量门禁：Rust全量 `cargo test --all-targets -j 1`通过，69库测试通过、1项公网opt-in忽略，全部集成测试通过；`cargo fmt --check`与`check --all-targets`通过。

## VID-04

- 状态：completed。
- 目标：建立video/audio双轨workspace、独立checkpoint、兼容轨恢复和非空原子finalize。
- Red：workspace合同测试先于实现，要求contained paths、track-specific checkpoint和非覆盖finalize。
- Green：新增 `FsVideoWorkspace`、显式video/audio/processed/checkpoint paths、单轨失配重置、video-only非空hard-link finalize与cleanup。
- Refactor：checkpoint V1仅序列化任务/轨道/identity/字节/etag，不写入URL或凭据；失配只丢弃对应轨道。
- 定向门禁：`video_workspace` 4项通过；`cargo fmt --check`与`cargo check --all-targets`通过。

## VID-05

- 状态：completed。
- 目标：实现可信FFmpeg `-c copy` MP4双输入封装，复用音频共享runner并保持失败/取消清理语义。
- Red：新增video mux port和missing-sidecar集成测试，先验证请求类型分层与稳定E008。
- Green：新增 `FfmpegVideoMuxer`/deferred adapter、固定无shell argv（双输入、map video/audio、`-c copy`、faststart）、共享locator/process runner、fake success/exit/cancel/empty清理。
- Refactor：视频mux只接收workspace paths，不携带audio profile、URL或任务状态；FFmpeg失败不泄漏stderr。
- 定向门禁：`video_mux` 3项集成测试及2项fake runner单元测试通过；Rust全量 `cargo test --all-targets`共71项通过、1项公网opt-in忽略，`fmt --check`与`check --all-targets`通过。
- 定向门禁：`video_mux` 3项集成测试及2项fake runner单元测试通过；Rust全量 `cargo test --all-targets`、`fmt --check`与`check --all-targets`通过。
- 真实sidecar缺失，真实MP4 smoke交由system-release，不伪造pass；mux失败仅删除processed并保留双轨输入。

## VID-06

- 状态：completed。
- 目标：把source/workspace/download/mux/reporter组成可控 video-only/video-audio attempt，覆盖预算、聚合进度和状态出口。
- Green：新增 `VideoExecutor`，video-only跳过mux直接finalize；video-audio在预算1顺序下载、预算>=2并行拆分连接并进入Processing/mux；双轨进度按bytes/speed聚合。
- Refactor：复用既有TaskExecutorPort、ExecutionUpdate、ExecutionReporterPort与共享DownloadControl/ProcessControl；active controls按task+attempt隔离。
- 定向门禁：`video_executor` 2项fake端到端测试通过，覆盖video-only零mux、video-audio一次mux和2+2连接预算。
- 定向门禁：`video_executor` 2项fake端到端与budget单元测试通过；Rust全量 `cargo test --all-targets` 73项通过、1项公网opt-in忽略，`fmt --check`与`check --all-targets`通过。

## VID-07

- 状态：completed。
- 目标：建立audio/video strategy registry，完成共享依赖的production composition，并协调认证变化后的前端能力刷新。
- Green：新增 `StrategyRegistry`/`TaskExecutorRegistry`，按mode唯一选择executor并隔离attempt controls；lib.rs同时注入 `AudioExecutor`、`VideoExecutor`、共享parser/downloader与分离workspace/mux。
- Refactor：认证状态变化清空前端解析缓存并fresh reparse一次；有效quality+codec/audio选择保留，失效选择回退并通过中英文安全提示告知。
- 定向门禁：registry unit通过；前端46个测试文件176项测试通过，`npm run build`通过。

## VID-08

### Review Closeout

- status: `completed`
- verification/review: all local gates pass; no blocking issues.
- trusted FFmpeg sidecar inventory and real MP4 smoke remain owned by `system-release`.

- 状态：in_progress。
- 目标：完成视频模块全量验收、静态安全扫描、视觉检查和交付证据矩阵。
- 当前证据：Rust 73项通过/1项公网opt-in忽略，前端176项通过，typecheck/build、fmt/check均通过；真实FFmpeg sidecar缺失，MP4 real smoke需system-release完成。
- 待完成：运行完整验收清单并绑定VID-AC-01..24证据，明确sidecar skip与残余风险。
