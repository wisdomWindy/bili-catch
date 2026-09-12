# 工程规格：视频下载与音视频合并

## Delivery unit identifier

`video-download`

## Background and goals

在已完成解析、任务、认证、设置和音频执行基础上，使 `video-only` 与 `video-audio` 任务可被production runner认领并可靠完成。用户只能选择B站当前返回的有效quality/codec组合；执行时重新验证账号与source；单轨或双轨支持恢复、暂停、取消、错误重试和安全清理；双轨使用FFmpeg stream copy封装MP4，不转码。

## In scope

- 下载中心quality/codec有效组合与登录状态协调。
- Rust稳定 `VideoVariant`、video source、通用media byte request和任务校验。
- `video-only` 单轨下载并直接原子保存 `.mp4`。
- `video-audio` 单任务内双轨下载、进度聚合与FFmpeg无损mux。
- 每轨checkpoint、resume、fresh source、取消/暂停确认、E007-E009与清理。
- audio/video executor strategy registry与production composition。
- fixture、loopback、fake process、全量回归、安全/视觉证据。

## Out of scope

- 视频转码、codec转换、HDR色彩处理、字幕、弹幕、直播、编辑。
- WebM/MKV等额外输出容器；V1只输出MP4。
- 新页面、新路由或任务状态。
- FFmpeg sidecar打包、签名、安装包与托盘/通知；归属 `system-release`。
- 绕过付费、地区、账号或源能力限制。

## Trigger and start conditions

- 用户已解析视频、至少选择一个分P，mode为 `video-only` 或 `video-audio`，并选择有效quality/codec与非空输出目录。
- create task批次通过100项容量和Rust原子校验后进入queued。
- runner仅在VideoExecutor strategy available时认领两种video mode；未满足合同的任务不得开始网络I/O。

## Requirement split summary

- 来源：`requirements/modules/video-download.md`，对应原PRD模块三、任务进度/重试与FFmpeg发布约束。
- 上游：`parse-download-center` 提供展示合同，`task-management` 提供attempt/state/retry，`authentication` 提供fresh权限，`settings` 提供并发/连接数，`audio-download` 提供已验收runner/HTTP/workspace/process边界。
- 当前模块只实现视频执行及必要的quality/codec合同修正；系统发布能力不前置。

## User flow

1. 解析结果展示quality；需要登录的quality保持disabled并标记登录要求。
2. 选择可用quality后，codec控件只展示该quality实际存在的AVC/HEVC/AV1；当前codec失效时回退兼容优先级最高的可用codec。
3. `video-only` 与 `video-audio` 只显示quality/codec，隐藏audio format/profile。
4. 加入队列时每个分P生成精确draft并跳转任务页。
5. runner认领后fresh解析source；仅视频下载一轨，视频+音频按连接预算顺序或并行下载两轨。
6. 页面沿用Downloading/Paused/Processing/Completed/Failed/Cancelled；仅视频下载完成后直接finalize，不额外显示Processing；双轨进入Processing后mux。
7. pause/cancel/retry/open/delete沿用已批准任务操作矩阵。

## Page and module design

- 无独立page-design。沿用 `DownloadPage -> DownloadOptions` 与 `TasksPage -> TaskRow`。
- `video-options.ts` 是quality/codec配对、排序和回退唯一前端owner；组件只渲染props/emits。
- quality顺序沿B站 `accept_quality`；codec同一quality内按AVC、HEVC、AV1兼容优先级展示。
- quality disabled原因继续使用既有“需登录”；没有有效codec的quality不得可选。
- mode切换保留最近有效video pair；从audio切回video时优先settings默认quality，失败则首个可用quality及其首个codec。
- auth状态变化使前端parse cache失效；当前成功结果自动fresh reparse一次。仍有效选择保留，否则按上述规则回退并显示一次本地化能力变化提示。

## Function-complete behavior breakdown

### Video option contract

- `ParseVideoResult` 新增required `videoVariants: VideoVariant[]`。
- 每个variant包含required `qualityId: string`、`codec: avc|hevc|av1`；仅代表当前响应实际DASH stream，不从全局codec列表猜组合。
- 既有 `qualities` 保留label与 `requiresLogin`，供locked quality展示；既有 `codecs` 在兼容期保留为variants去重投影，前端选择逻辑不得继续依赖其笛卡尔积。
- 空variants时两种video mode不可入队；不自动切换audio-only。

### Selection and draft

- quality/codec变化即时校验，不发网络请求；invalid pair使入队按钮disabled。
- draft的qualityId/codec required；audioFormat/audioBitrateId必须null。
- Rust按固定quality字符串与codec enum做结构校验；执行期source adapter再次验证pair存在，前端正确性不是安全边界。
- 单P文件名为videoTitle，多P为 `{page}_{partTitle}`，沿用200字符清洗与同名 `(n)` 不覆盖，扩展名固定 `.mp4`。

### Fresh source and permission

- 每个attempt都fresh获取validated auth、view与playurl；不得从task/checkpoint恢复时效URL。
- 匿名只允许API实际返回且quality不高于480P的variant；登录按账号/source实际返回允许至8K/HDR。
- 精确匹配qualityId+codec；无匹配为E004，明确需要登录为E003，权限拒绝为E006，网络中断/超时归一化E009。
- video-audio选择当前账号允许的最高普通AAC/M4A audio source，不选择FLAC/HiRes；无普通audio source为E004。
- video/audio URL与每次redirect必须HTTPS且命中B站媒体host allowlist。

### Connection budget and track scheduling

- `connectionsPerTask` 在attempt创建时冻结为1..32。
- video-only把完整预算交给video track。
- video-audio在预算>=2时按 `ceil(n/2)` 给video、`floor(n/2)` 给audio并并行；预算=1时先完成video再下载audio，保证总活跃下载连接不超过设置。
- downloader只有在total稳定且Range能力被证明时启用同轨多段；否则每轨降级为单连接，不把设置值当服务端能力证明。

### Workspace and checkpoints

- task root使用task/attempt哈希且canonical contained；路径显式区分video/audio/processed和两个checkpoint。
- 每轨checkpoint包含schema、task id、track kind、source identity hash、completed bytes、optional total/etag；不含URL/header/cookie。
- source identity/etag/total不兼容只重置对应轨；另一条已完成兼容轨保留。
- video-only finalize源video轨；video-audio只finalizemux输出。所有finalize要求非空、同卷原子保留、永不覆盖。

### Progress, pause and cancel

- 单轨与双轨都只在下载阶段报告0..90；双轨bytes/total/speed求和，两个total均已知才计算百分比和ETA，否则保持单调且total/ETA为null。
- pause请求传播到所有活跃writer；所有writer刷盘并确认后才报告Paused。已完成轨不回退。
- processing不允许pause；cancel同时终止所有writer或FFmpeg，等待句柄退出，清理成功后才Cancelled。
- CancelRequested与迟到Completed竞争时沿用TaskManager“取消优先并删除刚完成输出”。

### MP4 mux and output

- video-audio argv固定使用两个输入、显式map第一路video/第二路audio、`-c copy`、MP4 faststart；不使用shell，不拼接命令字符串。
- 输入必须为非空允许MIME的MP4/AAC轨；输出必须非空。spawn/exit/cancel/output invalid统一E008并使用稳定details，UI不显示details/stderr。
- video-only不调用FFmpeg，直接finalize原始MP4 video轨；遇到非 `video/mp4` source直接E004，不暗中转码或换容器。

### Failure, retry and cleanup

- E009由TaskManager最多自动重试3次，间隔1/2/4秒；E007/E008/E003/E004/E006不自动重试。
- download E004/E003/E006允许同attempt再fresh resolve一次后再失败，覆盖时效URL/权限刚变化；不得静默换quality/codec。
- success/cancel删除task workspace；E007/E008/E009保留合法已下载轨与checkpoint，删除processed；manual Retry保留mode/quality/codec/outputDir并fresh source。
- 启动恢复只删除内部processed残留；保留两轨partial/checkpoint并创建新attempt。

### UI error and task display

- 任务页继续显示quality/codec副信息；video-audio processing表示封装，video-only不产生假processing。
- E003/E004/E006/E007/E008/E009和internal均使用中英文安全文案；禁止展示raw backend message/details/stderr/path/URL。
- loading、busy、confirmation、focus return和task action矩阵不新增分支。

## Design constraints

- TaskManager唯一拥有状态/attempt/retry/cancel终态；executor不得直写store或emit。
- `videoVariants` 是quality/codec配对唯一公开规则；Rust source selector是执行期权威验证。
- B站raw/URL/credential只在infrastructure与内存attempt；WebView/task/event/checkpoint不可见。
- HTTP字节逻辑通用化必须保留audio MIME/tier/profile guard；不得以通用接口降低已验收安全规则。
- filesystem只在workspace，FFmpeg只在adapter，双轨协调只在video executor/progress。
- 新生产文件约250行进入审查，超过350行拆分；不以无语义wrapper规避。
- WebView capability不新增shell/filesystem/http；executable只从Tauri resource dir定位。

## Project bootstrap and scaffold decision

- 复用现有create-tauri-app Vue/TypeScript/Tauri工程与既有依赖，不新建package或替换starter。
- 允许新增当前模块Rust/TS文件并抽取真正共享的media bytes/path/process runner；不升级依赖、不改路由、不改视觉系统。
- 现有 `tsconfig.json` governing前端：strict、ES2020、DOM、ESNext、bundler、isolatedModules、noEmit、无alias；实现必须读取直接import的本地contract，不扫描全仓声明猜类型。

## Change axes and pattern decision

- Strategy registry：audio/video已有两个真实executor变化轴；registry只做mode路由和control委派，不做动态插件。
- Adapter：B站raw、HTTP、filesystem、FFmpeg为真实外部边界；fixture/loopback/fake替代默认公网和系统binary。
- 轻量Composite progress：只聚合两个固定track，不引入event bus或通用pipeline。
- 拒绝codec subclass、provider插件、pipeline DSL、shell command、前端source选择与无证据Range分段。

## Code context and impact assumptions

- code graph missing，仓库无bootstrap入口；沿用 `artifacts/code-context.md` 的 `rg`/入口/测试fallback。
- 直接影响：parse DTO/raw adapter、download-center options/store/component、task validation/file naming、download byte request、workspace、FFmpeg runner、download runtime registry、lib composition。
- 高风险邻居：audio 135项Rust回归、task cancel/retry/startup、auth cache、settings connection值、task row安全错误。
- architecture-design中定义的依赖方向、职责与类型策略为本规格上游，不在plan/execute重做。

## API and data contracts

### Public parse contract

- 权威source：Rust serde `ParseVideoResult` field table；仓库无backend-owned TypeScript声明/protobuf/OpenAPI。
- 通过现有Tauri `parse_video` command直接消费camelCase结果；TS等义镜像新增：

```ts
interface VideoVariant {
  qualityId: string;
  codec: "avc" | "hevc" | "av1";
}
```

- `videoVariants` required repeated array；空数组是成功但无可下载视频流，不是transport error。
- raw stream URL/bandwidth/mime不进入公开contract；语义归一化在Rust Bilibili adapter完成。

### Internal source/download contracts

- `VideoSourceRequest` required bvid/cid/mode/qualityId/codec；response required video source，video-audio时required audio source。
- `MediaSourceCandidate` required id/kind/primary/backup/mime/codecs/bandwidth；contentLength/etag nullable。
- `MediaDownloadRequest` required task/track/source/workspace/connectionCount；downloader验证kind与MIME匹配。
- `VideoMuxRequest` required video/audio/output paths；所有路径由workspace创建，不接受WebView输入。

### Existing task contract

- 不新增IPC command/event/status/action。`DownloadTaskDraft` 与 `DownloadTask` 继续使用qualityId/codec，video字段required/non-null，audio字段null。
- u64 progress仍跨IPC为十进制字符串；error保持 `{ code, message, details? }`，UI只按code展示。

## Context and dependency sources

- `biliCatch_PRD.md` 与 `artifacts/prd-snapshot.md`
- `requirements/requirement-map.md` 与 `requirements/modules/video-download.md`
- 当前 `design/architecture-design.md`
- 已批准audio/task/auth/settings规格、verification/review
- 根 `tsconfig.json`、Rust serde models、Bilibili raw/adapter、runner、TaskManager、HTTP/workspace/FFmpeg实现

## Edge cases

- quality存在但当前响应没有任何支持codec：disabled，不入队。
- 当前codec在quality切换后消失：按AVC/HEVC/AV1首个可用回退一次。
- 登录状态变化导致高quality失效：fresh reparse并回退；运行中任务不换quality，按稳定错误失败。
- 一轨完成、另一轨pause/fail：完成轨保留；resume只继续不完整轨。
- Range被忽略、416、ETag/总长变化、短读、redirect换host：沿用逐轨恢复/拒绝矩阵。
- 双轨一侧cancel已确认、另一侧迟到完成：等待两侧终止结果后统一cleanup，TaskManager cancel优先。
- FFmpeg缺失/退出非零/输出空：E008，保留两输入轨以便Retry。
- 同名输出并发：分别获得原名与 `(n)`，不覆盖。
- total未知：任务total/ETA为null，percent不回退；最终完成100。
- 非MP4 video或非普通AAC audio：拒绝，不转码、不静默换codec/quality。

## Acceptance criteria

- **VID-AC-01** `videoVariants` 精确表达实际quality/codec配对；UI与Rust拒绝无效笛卡尔组合，既有locked quality仍可见且disabled。
- **VID-AC-02** 匿名不超过480P；登录只展示账号/source实际可用至8K/HDR；auth变化清前端cache并fresh reparse，选择保留或一次回退。
- **VID-AC-03** video draft必须quality+codec非空且audio字段为null；非法批次原子拒绝，MP4文件名清洗/长度/多P/同名规则有测试。
- **VID-AC-04** 每attempt fresh auth/view/playurl并精确匹配quality+codec；不得持久化或向前端暴露media URL/credential。
- **VID-AC-05** video-audio只选择当前权限最高普通AAC/M4A，不使用FLAC；缺轨、登录、权限、地区/不可用错误分类可测试。
- **VID-AC-06** URL和每次redirect均重验HTTPS allowlist；video/audio MIME-kind交叉、非MP4视频和恶意host在写盘前拒绝。
- **VID-AC-07** video-only仅下载video并直接非空原子finalize为MP4，绝不调用FFmpeg。
- **VID-AC-08** video-audio在预算>=2并行、预算1顺序；分配和Range降级不超过attempt连接预算且byte完整。
- **VID-AC-09** video/audio各自checkpoint；206/200/416/ETag/长度/短读恢复矩阵逐轨通过，一轨失效不删除兼容另一轨。
- **VID-AC-10** 聚合progress bytes/speed单调，下载0..90，只有两total均已知才给total/ETA；最终100。
- **VID-AC-11** pause传播全部writer并在刷盘/确认后Paused；resume/restart新attempt恢复；已完成轨不重下。
- **VID-AC-12** cancel停止writer/process并等待确认，清理成功才Cancelled；cancel与Completed竞态保持取消优先。
- **VID-AC-13** video-audio进入Processing；FFmpeg argv为两输入、显式map、`-c copy`、faststart、无shell；仅视频不进入Processing。
- **VID-AC-14** mux输入/输出非空，失败稳定E008；成功原子落盘、不覆盖并清workspace。
- **VID-AC-15** E007/E008/E009保留兼容轨/checkpoint且删除processed；manual Retry保留选择并fresh resolve；启动只删processed残留。
- **VID-AC-16** E009最多3次1/2/4秒；其他错误不自动重试；stale attempt与回退进度不能覆盖新/终止态。
- **VID-AC-17** strategy registry同时路由audio-only与两种video mode；control只到对应active executor，audio全量行为不回归。
- **VID-AC-18** 下载中心/任务页继续使用既有布局、操作矩阵和a11y；video副信息、disabled状态与中英文能力变化提示可观察。
- **VID-AC-19** UI对E003/E004/E006/E007/E008/E009只显示本地化安全文案，不显示raw message/details/stderr/path/URL。
- **VID-AC-20** Rust serde/TS合同、nullability/enum/字符串大整数一致；无新command/event/WebView capability。
- **VID-AC-21** fake端到端覆盖video-only与video-audio happy path、单/双轨pause/resume/cancel/fail/retry/cleanup和metadata-independent mux。
- **VID-AC-22** TypeScript typecheck、前端全量测试/build、Rust fmt/check/full tests全部通过；audio/task/auth/settings无回归。
- **VID-AC-23** trusted FFmpeg存在时跑真实stream-copy MP4 smoke；缺失明确skip并由system-release实际sidecar E2E承接，不误报安装包能力。
- **VID-AC-24** clean-code review确认variant/source/downloader/workspace/progress/mux/executor/registry职责不混杂，生产文件阈值与Strategy/Adapter/Composite最小性通过。

## Human review and handoff

- 本规格需用户明确批准后才能进入plan；批准前 `video-download.approvals.spec_approved=false`、`stage=spec`、`pending_gate=spec_approval`。
- plan必须把VID-AC-01..24映射到contract/UI、source、media generalization、workspace、双轨协调、mux、registry/composition与验证任务。
- execute若证明video-only原始fragmented MP4无法满足可用输出，必须回到spec，不得暗中新增FFmpeg remux。
- video review PASS后交接 `system-release`；其不得改变本模块已批准的source、checkpoint、cancel和mux语义。

## Risks

- B站DASH字段/host/quality/HDR/codec持续变化；fixture只能证明adapter合同，公网smoke必须opt-in且不记录credential。
- 双轨并发和多段Range会放大文件句柄/网络压力；连接预算、降级和取消必须用loopback确定性测试。
- AV1/HEVC MP4 stream copy受播放器支持影响；本模块保证源无损封装，不承诺所有外部播放器解码。
- 真实FFmpeg、平台资源命名和安装路径仍依赖system-release外部产物。
