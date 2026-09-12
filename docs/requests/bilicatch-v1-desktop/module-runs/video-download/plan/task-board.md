# 任务板：视频下载与音视频合并

## 执行规则

- 固定顺序：VID-01 -> VID-02 -> VID-03 -> VID-04 -> VID-05 -> VID-06 -> VID-07 -> VID-08。
- 状态域：`pending | in_progress | completed | blocked`；同一时间只允许一个 `in_progress`。
- 每项完成可信red、最小green、refactor、定向门禁和changelog后才能completed。
- 单agent串行，不启用workflow/subagent；目录不是Git仓库。
- 所有shell命令以 `rtk` 开头；Rust使用 `C:/Users/yangjianlin/.cargo/bin/cargo.exe`。

## 总览

| ID | 名称 | 状态 | 模式 | 前置 |
| --- | --- | --- | --- | --- |
| VID-01 | VideoVariant合同与选择 | completed | 串行 | 计划批准 |
| VID-02 | 通用媒体Source与HTTP边界 | completed | 串行 | VID-01 |
| VID-03 | Fresh Video Source与权限 | completed | 串行 | VID-02 |
| VID-04 | 双轨Workspace与Checkpoint | completed | 串行 | VID-03 |
| VID-05 | FFmpeg MP4 Mux Adapter | completed | 串行 | VID-04 |
| VID-06 | VideoExecutor与聚合进度 | completed | 串行 | VID-05 |
| VID-07 | Strategy Registry与UI协调 | completed | 串行 | VID-06 |
| VID-08 | Production全量验收 | in_progress | 串行 | VID-07 |

## VID-01

- 任务名称：VideoVariant跨端合同、选择回退、draft与MP4命名。
- 状态：completed；执行说明：contract-first TDD已完成。
- 触发/前置：用户批准计划；读取tsconfig与serde source。
- 规格映射：VID-AC-01..03、18、20、22、24。
- 功能单元：variant、quality/codec联动、mode/auth回退、atomic validation、filename。
- 页面/模块：download-center contracts/options/store/component；Rust parse/task rules。
- 整洁性：pair规则单owner；组件只props/emits。
- Pattern边界：直接纯函数，不建codec strategy class。
- 上下文影响：兼容qualities/codecs；严格TS无alias。
- 状态约束：invalid pair禁入队；提示一次；空variants不换mode。
- API类型：Rust serde权威、TS camelCase镜像。
- 测试切入：serde、纯规则、store/component、task validation。
- 已批准假设：Q4/Q6；无待确认项。

## VID-02

- 任务名称：MediaKind/Source/Request通用化与HTTP安全边界。
- 状态：completed；执行说明：audio regression-first TDD已完成。
- 触发/前置：VID-01 completed。
- 规格映射：VID-AC-06、08、09、15、17、22、24。
- 功能单元：media candidate、MIME-kind、identity、Range/redirect/预算。
- 页面/模块：Rust audio/download services与HTTP infrastructure。
- 整洁性：Range算法单owner；不弱化audio guard。
- Pattern边界：Adapter共享字节传输，不建媒体pipeline。
- 上下文影响：audio executor/source tests必须全过。
- 状态约束：paused/cancelled typed outcome；未证明Range则单连接。
- API类型：内部非serde；URL只在attempt内存。
- 测试切入：loopback audio/video、交叉MIME、track identity。
- 已批准假设：Q2/Q3；无待确认项。

## VID-03

- 任务名称：B站Fresh Video Source、variant与权限选择。
- 状态：completed；执行说明：fixture/adapter TDD已完成。
- 触发/前置：VID-02 completed。
- 规格映射：VID-AC-01、02、04..06、19..21、24。
- 功能单元：raw stream、codec adapter、fresh auth/WBI、video/audio selector、URL allowlist。
- 页面/模块：Bilibili raw/adapter/ParserService/video service。
- 整洁性：raw不出infrastructure；选择规则不复制。
- Pattern边界：VideoSourcePort Adapter；拒绝provider插件。
- 上下文影响：parse cache/auth revision、audio source复用。
- 状态约束：运行中不换pair；至多fresh resolve一次。
- API类型：public VideoVariant；private MediaSourceBundle。
- 测试切入：codec/quality/auth/缺轨/host/signature/secret。
- 已批准假设：Q2/Q4/Q5/Q6；无待确认项。

## VID-04

- 任务名称：双轨Workspace、独立Checkpoint与原子输出。
- 状态：completed；执行说明：tempdir/fault TDD已完成。
- 触发/前置：VID-03 completed。
- 规格映射：VID-AC-03、07、09、11、12、14、15、21、24。
- 功能单元：contained paths、per-track resume/reset、finalize、cleanup/startup。
- 页面/模块：Rust workspace/shared safe-path/task startup tests。
- 整洁性：workspace独占filesystem；显式paths不用string map。
- Pattern边界：窄port/adapter；只共享path原语。
- 上下文影响：audio workspace/finalize不回归。
- 状态约束：pause/failure保留，cancel/success清理。
- API类型：Checkpoint V1/versioned，无URL/credential。
- 测试切入：containment、单轨失配、同名、empty、restart。
- 已批准假设：Q1/Q2；无待确认项。

- 定向门禁：`video_workspace` 4项通过；`cargo fmt --check`、`cargo check --all-targets`通过。

## VID-05

- 任务名称：可信FFmpeg Stream-copy MP4 Mux Adapter。
- 状态：completed；执行说明：argv/fake process TDD已完成。
- 触发/前置：VID-04 completed。
- 规格映射：VID-AC-12..15、19、21、23、24。
- 功能单元：mux request、argv、shared runner、cancel、probe、E008。
- 页面/模块：Rust video mux/FFmpeg infrastructure/tests。
- 整洁性：无shell；video mux不含audio profile或任务状态。
- Pattern边界：Adapter复用locator/runner；拒绝每codec class。
- 上下文影响：audio FFmpeg 14项回归；system-release sidecar handoff。
- 状态约束：cancel等进程退出；失败保留输入、删processed。
- API类型：internal paths only；stable E008 details。
- 测试切入：argv、特殊路径、spawn/exit/cancel/empty/skip。
- 已批准假设：Q1/Q2/Q5；无待确认项。
- 已完成：service mux contract、shared FFmpeg runner、离散argv、fake success/exit/cancel/empty及missing-sidecar E008；mux失败只删除processed并保留双轨输入。
- 定向门禁：`video_mux` 3项集成测试、2项fake runner单元测试通过；Rust全量门禁通过。
- 真实sidecar缺失按system-release交接，不伪造真实MP4 smoke通过。

## VID-06

- 任务名称：VideoExecutor双模式、双轨预算与聚合进度。
- 状态：completed；执行说明：fake end-to-end TDD已完成。
- 触发/前置：VID-05 completed。
- 规格映射：VID-AC-07..16、21、22、24。
- 功能单元：single/dual track、budget、progress、controls、mux/finalize、errors。
- 页面/模块：Rust video executor/progress/runtime/tests。
- 整洁性：线性阶段；不持锁await；所有future收束。
- Pattern边界：轻量Composite progress；不建pipeline DSL。
- 上下文影响：TaskManager attempt/cancel/retry和audio executor。
- 状态约束：0..90、双轨Processing、cancel优先、stale ignored。
- API类型：ports + typed outcomes；reporter唯一回写。
- 测试切入：两happy path、预算、unknown total、竞态、E007/8/9。
- 已完成：VideoExecutor双模式、预算1顺序与预算>=2并行、聚合progress、Processing/mux/finalize及2项fake端到端测试。
- 定向门禁：`video_executor` 2项fake端到端与budget单元测试通过；Rust全量门禁通过。
- 异常处理：下载/混流失败保留输入、取消清理workspace、未知total不输出total/ETA；TaskExecutorPort控制按attempt隔离。
- 已批准假设：Q1/Q3/Q5；无待确认项。

## VID-07

- 任务名称：Audio/Video Strategy Registry、production composition与认证UI协调。
- 状态：completed；执行说明：integration/store/page TDD已完成。
- 触发/前置：VID-06 completed。
- 规格映射：VID-AC-02、17..20、22..24。
- 功能单元：mode routing、control隔离、composition、auth cache invalidation/reparse、safe UI。
- 页面/模块：runner/lib.rs、DownloadPage/store/options/task row/locales/tests。
- 整洁性：composition只组装；组件不import transport/store。
- Pattern边界：静态Strategy registry；拒绝plugin/event bus。
- 上下文影响：root auth listener、parse token/cache、audio runner。
- 状态约束：reparse loading可靠结束；stale结果不覆盖；任务矩阵不变。
- API类型：无新command/event/capability。
- 测试切入：registry/control、auth登入登出、zh/en、a11y。
- 已批准假设：Q6；无待确认项。
- 已完成：静态Strategy registry路由audio/video mode；production composition共享parser/downloader、分离workspace/mux；auth变化清cache并fresh reparse，保留有效pair并对失配显示中英文提示。
- 定向门禁：registry unit、store/page auth refresh回归、前端全量测试与build通过。

## VID-08

### Review Closeout

- status: `completed` after full verification and review.
- blocking issues: 0.
- trusted FFmpeg sidecar inventory and real MP4 smoke are handed to `system-release`.

- 任务名称：Production全量回归、安全/视觉与验收证据。
- 状态：in_progress；执行说明：串行收口，失败回owner。
- 触发/前置：VID-07 completed。
- 规格映射：VID-AC-01..24全部。
- 功能单元：full gates、static scan、visual、AC matrix、clean-code review。
- 页面/模块：完整video/audio/task/auth/settings、config/docs。
- 整洁性：生产文件阈值、职责/pattern逐项复核。
- Pattern边界：确认Strategy/Adapter/Composite必要且最小。
- 上下文影响：全量基线、capabilities、FFmpeg release handoff。
- 状态约束：blocker清零才进verify；system-release仍queued。
- API类型：serde/TS/IPC/event/checkpoint一致。
- 测试切入：npm/Cargo full、loopback/fake、static、screenshots、optional smoke。
- 已批准假设：Q1..Q6；真实sidecar打包deferred。
