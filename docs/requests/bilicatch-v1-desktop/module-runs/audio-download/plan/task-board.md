# 任务板：音频独立下载

## 执行规则

- 固定顺序：AUD-01 -> AUD-02 -> AUD-03 -> AUD-04 -> AUD-05 -> AUD-06 -> AUD-07 -> AUD-08。
- 状态域：`pending | in_progress | completed | blocked`；同一时间只允许一个 `in_progress`。
- 每项完成可信red、最小green、refactor、定向门禁和changelog记录后才能completed。
- 单agent串行，不启用workflow/subagent。目录不是Git仓库，不创建虚假commit检查点。
- 所有shell命令以 `rtk` 开头；Rust命令使用 `C:/Users/yangjianlin/.cargo/bin/cargo.exe`。

## 总览

| ID | 名称 | 状态 | 模式 | 前置 |
| --- | --- | --- | --- | --- |
| AUD-01 | 跨端合同、Profile矩阵与命名 | completed | 串行 | 计划批准 |
| AUD-02 | Task Runtime与取消确认 | completed | 串行 | AUD-01 |
| AUD-03 | B站Fresh Source与权限 | completed | 串行 | AUD-02 |
| AUD-04 | Workspace、Range与Checkpoint | completed | 串行 | AUD-03 |
| AUD-05 | FFmpeg参数与Process Adapter | completed | 串行 | AUD-04 |
| AUD-06 | AudioExecutor编排与恢复 | completed | 串行 | AUD-05 |
| AUD-07 | 下载/任务UI联动 | completed | 串行 | AUD-06 |
| AUD-08 | Production接线与全量证据 | completed | 串行 | AUD-07 |

## AUD-01

- 任务名称：跨端音频合同、Profile矩阵与命名规则。
- 状态：completed。
- 执行模式/说明：串行，contract-first TDD。
- 触发/前置：用户批准计划。
- 规格映射：AUD-AC-01、04..06、18、21、24。
- 功能单元：AudioOutputProfile、audioCapability、draft字段、validation、filename。
- 页面/模块范围：TS/Rust contracts、download options纯规则、task validation/filename/tests。
- 整洁性：output/source术语分离；矩阵单owner；不在组件复制。
- Pattern/边界：Rust serde source -> TS camelCase镜像；纯enum/match，不建格式class。
- 上下文影响：strict/bundler/no alias；旧task V1加载与existing tests。
- 关键状态：非法组合整批拒绝；无I/O/UI副作用。
- API/类型：videoTitle required string、partCount required integer、profile fixed union。
- 测试切入：TS矩阵、exact serde、filename边界、typecheck/fmt/check。
- 已批准假设：clarifications Q1/Q7；无待确认项。

## AUD-02

- 任务名称：Task Runtime、支持模式认领与取消确认。
- 状态：completed。
- 执行模式/说明：串行，fake runtime竞态TDD。
- 触发/前置：AUD-01 completed。
- 规格映射：AUD-AC-07、10、14、15、17、21、24。
- 功能单元：execution spec、dispatcher、runner、control forwarding、update ack、slot recovery。
- 页面/模块范围：Rust tasks/download_runtime/settings provider/store tests。
- 整洁性：runner不持锁await；TaskManager唯一状态owner；handle显式回收。
- Pattern/边界：Strategy dispatcher + State/Command；拒绝audio状态机。
- 上下文影响：现有active cancel语义、recovery、progress coalescer和commands。
- 关键状态：active cancel pending -> executor stop/cleanup -> Cancelled；旧attempt ignored。
- API/类型：internal spec/update/control，不新增WebView合同。
- 测试切入：fake executor、动态limits、panic、重复control、cleanup failure。
- 已批准假设：clarifications Q6/Q9；无待确认项。

## AUD-03

- 任务名称：B站Fresh Source、认证能力与安全URL适配。
- 状态：completed。
- 执行模式/说明：串行，fixture/adapter TDD。
- 触发/前置：AUD-02 completed。
- 规格映射：AUD-AC-02、03、05、08、18、19、22、24。
- 功能单元：raw DASH/FLAC、shared WBI flow、source policy、metadata、URL allowlist。
- 页面/模块范围：Rust bilibili/parser/audio source与parse fixtures。
- 整洁性：raw不出infrastructure；签名规则不复制；credential不序列化。
- Pattern/边界：BilibiliAudioSource Adapter；不建立通用provider框架。
- 上下文影响：parse cache/auth revision、BilibiliPort tests、network error mapping。
- 关键状态：每attempt fresh auth/source；FLAC E005/E006；普通源E004/E009。
- API/类型：stable AudioSourceBundle、ParseVideoResult.audioCapability。
- 测试切入：anonymous/auth/lossless、恶意host、signature refresh、secret scan。
- 已批准假设：clarifications Q1/Q2/Q9；无待确认项。

## AUD-04

- 任务名称：可信Workspace、Range下载与Checkpoint恢复。
- 状态：completed。
- 执行模式/说明：串行，loopback/filesystem TDD。
- 触发/前置：AUD-03 completed。
- 规格映射：AUD-AC-06、09、10、13..17、19、21、22、24。
- 功能单元：workspace containment、checkpoint、Range/multipart、progress、pause/cancel、finalize。
- 页面/模块范围：Rust download/audio infrastructure和temp integration tests。
- 整洁性：downloader不懂TaskStatus；workspace独占路径副作用；typed errors。
- Pattern/边界：ByteDownloaderPort/AudioWorkspacePort Adapter；无repository。
- 上下文影响：Windows handles、settings temp/output、task temporaryPaths。
- 关键状态：pause保留；cancel清理；failure保留合法source；success原子output。
- API/类型：checkpoint V1无URL/cookie；DownloadOutcome typed union。
- 测试切入：206/200/416/ETag/短读/断流/containment/同名并发。
- 已批准假设：clarifications Q6/Q9；无待确认项。

## AUD-05

- 任务名称：FFmpeg参数、可信Locator与可取消Process Adapter。
- 状态：completed。
- 执行模式/说明：串行，argv/fake process TDD。
- 触发/前置：AUD-04 completed。
- 规格映射：AUD-AC-11、12、15、16、18、19、22、23、24。
- 功能单元：profile argv、metadata/cover、locator、process cancel、probe、E008。
- 页面/模块范围：Rust audio service/infrastructure、Cargo features、tests。
- 整洁性：无shell字符串；stderr受限；参数builder纯函数。
- Pattern/边界：MediaProcessorPort Adapter；拒绝PATH扫描/shell plugin/每格式class。
- 上下文影响：system-release sidecar handoff、Windows child process关闭。
- 关键状态：processing cancel等待退出；输出无效不成功。
- API/类型：AudioProcessRequest typed；stable E008 details。
- 测试切入：三格式argv、metadata特殊字符、spawn/exit/cancel/empty/probe。
- 已批准假设：clarifications Q4/Q5/Q8；无待确认项。

## AUD-06

- 任务名称：AudioExecutor线性编排、Finalize与错误恢复。
- 状态：completed。
- 执行模式/说明：串行，fake end-to-end TDD。
- 触发/前置：AUD-05 completed。
- 规格映射：AUD-AC-07..18、22、24。
- 功能单元：source->download->process->finalize->cleanup、reporter、retry分类。
- 页面/模块范围：Rust audio executor/ports/runtime glue/integration tests。
- 整洁性：线性阶段；外部await前后control check；不直写task store。
- Pattern/边界：port composition；TaskManager reporter唯一回写。
- 上下文影响：attempt/state/event顺序、cleanup与final rename竞态。
- 关键状态：0-90 download、processing、100 completed；pause/cancel/failure分支。
- API/类型：ExecutionReporterPort、typed outcomes/errors。
- 测试切入：三格式、pause/restart/cancel、E007/8/9、late update、workspace集合。
- 已批准假设：clarifications Q2/Q6/Q9；无待确认项。

## AUD-07

- 任务名称：下载中心/任务页联动、双语与可访问性。
- 状态：completed。
- 执行模式/说明：串行，store/component/page TDD。
- 触发/前置：AUD-06 completed。
- 规格映射：AUD-AC-01..06、10、14、15、18、20、21、24。
- 功能单元：format/profile controls、FLAC回退、exact draft、task副信息/error。
- 页面/模块范围：download-center、task-management、locales、demo/tests/CSS。
- 整洁性：components props/emits；规则在audio-options/store；不importTauri/Pinia到leaf。
- Pattern/边界：直接模块函数+existing store/service；拒绝event bus/watch fan-out。
- 上下文影响：auth公开snapshot、settings default、existing download/task visual。
- 关键状态：disabled/current、一次提示、busy起止、stale result、safe errors。
- API/类型：direct Rust DTO镜像；无新command/event。
- 测试切入：矩阵联动、auth变化、aria/focus、zh/en、visual DOM。
- 已批准假设：clarifications Q1..Q3/Q7；无待确认项。

## AUD-08

- 任务名称：Production Composition、全量回归与验收证据。
- 状态：completed。
- 执行模式/说明：串行收口；失败回owner任务。
- 触发/前置：AUD-07 completed。
- 规格映射：AUD-AC-01..24全部。
- 功能单元：Tauri composition/shutdown、full gates、static security、visual、evidence。
- 页面/模块范围：完整audio/task/parse/auth/settings邻居、config/docs。
- 整洁性：composition只组装；AC逐条证据；deferred sidecar不伪装pass。
- Pattern/边界：复核Strategy/Adapter/State/Observer必要且最小。
- 上下文影响：全量基线、capability、Cargo locks、system-release handoff。
- 关键状态：所有blocker清零才移交verify；video仍queued。
- API/类型：全链serde/TS/internal ports/FFmpeg合同。
- 测试切入：npm/Cargo full、loopback/fake、static scan、screenshots、opt-in smoke。
- 已批准假设：clarifications Q5/Q8；真实sidecar打包明确deferred。
