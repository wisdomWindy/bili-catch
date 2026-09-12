# 工程规格：音频独立下载

## 交付单元标识

`audio-download`

## 背景与目标

现有下载中心可以创建 `audio-only` 草稿，任务管理模块也已交付队列、状态机、持久化、进度事件和控制操作，但 production executor 仍为 deferred，解析合同的音频选项仍把 B 站源码率当作输出码率。本模块交付第一个真实下载执行策略：只下载音频轨，在权限和源能力允许时输出 MP3、M4A 或 FLAC，写入标题、UP 主和封面元数据，并完整接入暂停、恢复、取消、重试和临时文件清理。

目标不是建立新的页面或队列，而是把既有下载中心和任务页串成可执行闭环，同时提供后续视频模块可复用的 runner、HTTP 下载和 FFmpeg process 边界。

## 范围内

- “仅音频”模式的 MP3 128K/192K/320K、M4A 原始码率、FLAC 无损能力矩阵。
- 匿名源最高 64K、登录源最高 192K + Hi-Res 的能力约束；FLAC 需要已认证且源明确支持无损。
- 入队前的 FLAC 不可用降级为 M4A并显示可见提示；执行期能力变化则失败，不静默改格式。
- fresh playurl 与元数据获取，只下载一个音频轨，不请求/下载视频轨。
- HTTP Range 断点恢复、设置连接数消费、进度/速度/ETA回写和应用重启恢复。
- FFmpeg M4A stream-copy remux、MP3 encode、FLAC encode，以及标题、artist=UP 主、封面 metadata。
- 可信临时工作区、checkpoint、原子输出、文件名规则、冲突去重和 success/cancel/failure 清理策略。
- 共享 DownloadRuntimeRunner/dispatcher 与 audio executor 注册，修正活动任务取消确认语义。
- E007/E008/E009、本地化错误、最多三次网络自动重试和手动重试。
- 前端/Rust contracts、单元/集成/恢复测试与现有页面回归。

## 范围外

- 视频轨下载、视频+音频并行拉取、DASH 合并和视频 remux，归 `video-download`。
- 通知、托盘、窗口关闭、FFmpeg sidecar 二进制安装/签名/打包和自动更新，归 `system-release`。
- 字幕、弹幕、直播、播放列表、音频标签编辑器、均衡/音量处理、采样率/声道自定义。
- 通过转码宣称提升源音质；MP3 320K 只表示输出编码参数。
- WebView 直接访问媒体 URL、Cookie、文件系统写入或 FFmpeg。
- 新页面、新路由、音频专属事件、任务优先级和任务模型之外的第二套队列。

## 触发与开始条件

- 用户成功解析视频，在下载中心选择一个或多个分 P，切换到“仅音频”，选择格式/码率和输出目录后添加到队列。
- DownloadRuntimeRunner 发现可认领的 `audio-only` queued 或 resume-requested 任务且并发槽可用。
- 用户在任务页对下载中的音频任务暂停/取消，或对 paused/failed 任务恢复/重试。
- 应用重启后 TaskManager 将未完成任务恢复为 queued，runner 以新 attempt 读取已有 checkpoint。

前置条件：`parse-download-center`、`task-management`、`authentication`、`settings` 已通过 review；`audio-download/design/architecture-design.md` 已存在。FFmpeg 7.x sidecar 的最终打包不是本模块开始条件，但真实 process adapter 的路径合同和缺失时 E008 必须交付。

## 需求拆分摘要与来源

- 当前模块边界来自 `requirements/modules/audio-download.md`，只承接独立音频下载及其直接依赖接入。
- 原始来源为 `biliCatch_PRD.md` 模块二（244-294）、模块四任务规则、5.1 功能约束、7.3 错误码和技术选型中的 reqwest/Tokio/FFmpeg 7.x。
- 保留的显式要求：MP3 三档、M4A 原始码率、FLAC 无损；仅 MP3 可选码率；音频轨下载、转换、metadata、临时清理；断点续传；E007/E008/E009；失败可重试。
- 与原始 PRD 的冲突处理记录在 `spec/clarifications.md`：FLAC 自动降级只发生在入队前，执行中不得改变已承诺文件类型；输出编码码率与源质量分开解释。
- 不吸收 `video-download` 的双轨并行/合并，也不吸收 `system-release` 的 sidecar 分发和系统通知。

## 用户流程

1. 解析成功后，用户选择“仅音频”；清晰度和视频编码控件隐藏，音频格式与码率控件显示。
2. 格式默认为 settings 已确认的默认音频格式；若默认 FLAC 在当前认证/源能力下不可用，自动改为 M4A，并在选项区显示一次可访问提示。
3. 选择 MP3 时码率下拉启用，可选 128K、192K、320K，默认 320K；选择 M4A 时下拉禁用并显示“原始码率”；选择 FLAC 时下拉禁用并显示“无损”。
4. FLAC 始终显示在格式选项中：匿名时禁用并标注需登录；登录但源无损不可用时禁用并标注源不支持。当前值因登录/解析结果变化而失效时，回退 M4A 并提示。
5. 至少一个分 P、合法格式 profile 和合法下载目录存在时可入队。每个分 P创建一个任务，成功后沿用现有提示并跳转 `/tasks`。
6. runner 认领任务后，任务页展示 downloading；解析源、准备工作区期间进度可为 0，字节下载阶段展示 0-90% 的进度、速度和 ETA。
7. 下载完成后状态进入 processing；M4A进行 copy remux，MP3/FLAC 处理格式并写 metadata。处理阶段保持至少 90%，完成后变为 100%。
8. 成功后任务显示 completed，可打开文件/目录；临时工作区已清理。暂停保留断点；取消在真实 writer/process 停止并清理后才显示 cancelled。
9. 网络中断自动重试；磁盘/FFmpeg/权限失败显示本地化错误与 Retry。手动重试复用兼容 partial 或已下载 source，不复用不完整 processed 输出。

## 页面与模块设计

### 下载中心“仅音频”选项

- 不改变已批准页面层级、宽度、卡片规则或路由；继续使用 `DownloadOptions.vue` 中的原生 select 和现有 field styling。
- 下载类型控件：选择 `audio-only` 后隐藏 quality/codec，显示 format/bitrate；切出后清空音频 profile，保持现有视频字段规则。
- 音频格式：required，选项顺序 `MP3 / M4A / FLAC`。MP3、M4A 在有任一可用音频源时可选；FLAC 按认证+source capability禁用。
- 音频码率/profile：required。MP3 显示并启用 `128K / 192K / 320K`，默认 `320K`；M4A 显示 disabled `原始码率`；FLAC 显示 disabled `无损`。disabled 控件仍有程序化 label 与可读当前值。
- 能力提示：仅在自动回退或选中不可继续的意图发生时显示 inline status，不长期堆叠说明性文案；中文/英文完整，不能显示 raw source id、API message 或 cookie。
- 添加队列：沿用原按钮、loading/busy和一次性 handoff；非法/失效 profile 禁止提交，不能依赖 Rust 失败后再修正 UI。
- 窄屏沿用已批准 DownloadPage 单列规则；新增提示和最长英文选项不得制造横向滚动或遮挡。

### 任务页

- 不新增列或状态。`downloading` 表示获取源/下载字节，`processing` 表示 remux/encode/metadata/finalize。
- 任务副信息显示 `仅音频 · MP3 320K`、`仅音频 · M4A 原始码率` 或 `仅音频 · FLAC 无损`；不得把 B 站源上限误显示成输出 profile。
- E007/E008/E009通过既有 task error区域本地化：磁盘空间不足、媒体处理失败、网络中断可重试。UI 不显示 backend message/details/stderr/URL。
- 暂停、恢复、取消、重试、删除和打开操作沿用现有 action policy；active cancel pending期间状态仍为当前状态且按钮 busy，直到权威事件确认。

## 功能完备行为拆分

### 1. 音频能力与表单归一化

- 输入：`ParseVideoResult` 的音频可用性、当前 `AuthSnapshot.status`、settings default、当前 mode/format/profile。
- `AudioOutputProfile` 映射固定：`mp3+128|192|320`、`m4a+source`、`flac+lossless`。其他组合在前端和 Rust 均拒绝。
- `audioBitrateId` 继续作为 string 跨端字段，值域改为上述 profile id；它不再直接表示解析响应中的 source bandwidth。
- MP3 输出选项不因源只有 64K 而消失；界面不得暗示输出编码码率等于源质量。匿名任务执行器只能选择不超过 64K 的源，登录任务选择账号/视频允许的最佳源，最高 192K/Hi-Res。
- 解析没有任何音频流：MP3/M4A/FLAC全部禁用、添加队列不可用，沿用解析能力不可用提示。
- FLAC availability = authenticated AND current source reports lossless/Hi-Res usable。匿名、auth error、源不支持均不可入队 FLAC。
- 当前 format失效时只自动回退到 M4A；若 M4A也不可用则清空 format/profile。自动回退只执行一次状态变更并产生一次提示，不形成 watcher 循环。
- 切换 MP3 -> M4A 设置 profile=`source`；MP3 -> FLAC 设置 `lossless`；从 M4A/FLAC -> MP3 恢复本次解析会话最后一次合法 MP3码率，没有历史则 320。
- 入队 Rust validation 再校验 mode/format/profile 组合和新增文件命名字段；任一 draft非法则整批原子拒绝。

### 2. 草稿与文件命名

- `DownloadTaskDraft` 新增 required `videoTitle: string` 和 `partCount: number`，后端 serde 使用 camelCase；前端由当前解析结果直接填写，不接受用户输入。
- `videoTitle` 和 `partTitle` 在生成文件名时 trim；换行与连续空白折叠为单空格；Windows 非法字符与控制字符移除；净化后为空使用现有 `bvid-P{page}` fallback。
- `partCount=1` 使用 `{videoTitle}.{ext}`；`partCount>1` 使用 `{page}_{partTitle}.{ext}`。页码为十进制原值，不补零。
- 最终 stem+extension仍限制 200字符；截断按 Rust char边界，不截断扩展名。既有同名文件不得覆盖，按 `{stem} (1).{ext}` 递增选择可用名。
- draft文本由解析结果产生，但 Rust仍限制非空规范化结果、`partCount>=1`、`1<=page<=partCount` 并验证 outputDir为绝对存在目录。

### 3. Runner 与执行器分派

- Tauri setup 创建 `Arc<TaskManager>`、audio executor、dispatcher 和一个 app-lifecycle runner；runner 不阻塞 setup，不 busy-spin，退出时停止接收新任务并取消/等待 active handles。
- dispatcher 仅对 `audio-only` 返回 available audio strategy；`video-audio`/`video-only` 在后续模块注册前保持 queued，不进入 failed或消耗 retry。
- `claim_next` 在持久化 downloading/attempt后返回完整不可变任务快照、attemptId、connectionCount、temporaryDirectory。executor不可修改 spec。
- runner 中同一 `(taskId, attemptId)` 只允许一个 handle；start应快速 spawn作业。作业结束/失败/panic都必须释放 active slot；panic归一化为 Internal failure且不得丢失任务控制权。
- runner 每轮先转送 active pause/cancel request再认领新任务；重复 request幂等。executor确认 Paused/Cancelled后 manager 清除 attempt/control。

### 4. Fresh source解析与权限

- 每个新 attempt通过 `AuthContextProvider::validated_context` 获取当前 context，再获取 view、WBI key和 playurl；不得复用解析页保存的 URL。
- 请求只提交稳定 `bvid/cid`、WBI参数和当前 credential header；credential只存在于认证/request adapter内。
- `BilibiliAudioSource` 从 raw DASH提取 audio id、primary/backup URLs、bandwidth、codec/container和 FLAC/Hi-Res capability，再转为内部 `AudioSourceBundle`。
- source选择以“当前权限允许且与输出要求兼容”为前提：M4A/MP3选最佳可用有损源；FLAC只选明确无损/Hi-Res源。匿名候选过滤到 <=64K；已认证有损候选过滤到 <=192K，另允许明确 Hi-Res/无损分支。
- 选择不到普通音频为 E004/source unavailable；匿名请求 FLAC为 E005；已登录但账号/源无权限为 E006。均不自动网络重试。
- raw media URL只允许 HTTPS 和批准的 Bilibili media host suffix；redirect每跳重新校验。URL、query、headers不进入 error details、checkpoint、task JSON或 event。
- metadata required：title、uploader、coverUrl。title/uploader空值使用可识别 fallback；cover抓取/解码失败按 E008，不静默产出缺封面文件。

### 5. 临时工作区与 checkpoint

- workspace root来自 SettingsManager 当前 `temporaryDirectory`；每个 task使用内部 hash目录，不直接拼标题；每次 attempt可复用同 task稳定 source/checkpoint并拥有独立 processed临时名。
- prepare在写入前 canonicalize/验证 root与派生路径；所有递归清理目标必须位于该 task workspace下。绝不清理 root、outputDir、最终输出或前端传入的任意路径。
- checkpoint V1 required：schemaVersion、taskId、sourceIdentityHash、completedBytes；optional：totalBytes、etag。禁止持久化 URL/cookie/header/API body。
- source identity不兼容、partial长度与 checkpoint不一致、offset大于 total、schema未知时：隔离/删除不兼容 partial并从 0重启；不得把损坏数据 append到新源。
- 每次完整写入边界后更新 checkpoint，使用 `.next` + rename；pause前 flush数据和 checkpoint后才回报 Paused。
- 应用重启把 downloading/processing恢复 queued并保留工作区；下一 attempt重新解析源、验证 identity后恢复。正在 processing崩溃时删除 processed临时项，但可复用已完成 source。

### 6. HTTP Range下载与进度

- 首次或 offset=0请求普通 GET；恢复请求使用 `Range: bytes={offset}-`。206必须验证 Content-Range起点；200表示服务端忽略 Range，清空 partial/checkpoint后仅重试一次完整下载。
- 416仅在本地长度等于已知 total时视为下载已完成，否则 checkpoint无效并从 0重新开始一次；重复不一致失败为 E009。
- connectionCount在 1-32内。只有 total长度稳定且服务器支持 Range时才允许分段并发；否则退化单连接。分段合并必须按 byte range顺序、无空洞/重叠并校验最终长度。
- downloader流式写磁盘，不把媒体完整载入内存；写入错误若是 ENOSPC映射 E007，超时/连接重置/短读映射 E009，4xx/权限错误使用相应 E004/E005/E006。
- progress bytes单调；下载阶段 percent=`floor(downloaded/total*90)`，范围 0-90。total未知时 totalBytes=null、percent保持已有值，速度仍可更新；ETA仅在 total与有效速度均存在时给出。
- 下载结束必须 flush/close所有writer并验证期望长度，才进入 processing。任何 track为视频类型都拒绝；该模块不得发起视频 URL请求。

### 7. FFmpeg处理与 metadata

- 所有命令使用 executable + argv array直接 spawn，不经过 shell。路径、标题、UP 主均作为单独参数，不能由用户构造额外开关。
- 通用参数包含 non-interactive、覆盖仅限内部 processed临时文件、输入 source/cover、禁用视频主体、显式 metadata title/artist、将 cover映射为 attached picture；最终参数由格式专属纯函数决定。
- MP3：`libmp3lame`，`-b:a`只能为 `128k|192k|320k`；输出容器 mp3，封面 attached picture，禁止复制视频轨。
- M4A：音频 `-c:a copy`并输出兼容 MP4/M4A容器；该过程是 remux/metadata，不重新编码音频。
- FLAC：仅在 source tier无损/Hi-Res验证后使用 FLAC encoder；不得从普通有损源生成标记为 lossless的输出。
- FFmpeg不存在、无法启动、非零退出、cancel后未终止、输出为空或 metadata/cover不受支持均为 E008；stderr只保留受限诊断供内部测试/日志策略，不进入前端 AppError.message。
- processing阶段cancel先请求终止子进程，等待确认退出并关闭文件句柄，再删除 processed/source/cover/checkpoint；成功前不能回报 Cancelled。
- FFmpeg成功后验证输出存在且非空；可选测试通过 ffprobe/等价 probe验证音频 codec、title、artist和attached picture。验证失败视为 E008。

### 8. Finalize与清理

- 输出目录取任务创建时确认的 `outputDir`；执行时目录不存在/不可写或空间不足则 E007/受控 path error，不自动改到其他目录。
- processed输出先位于 outputDir中的隐藏/内部临时名，以保证最终 rename同卷；确定不冲突目标后原子 rename。不得覆盖已有用户文件。
- Completed只有在 rename成功、最终文件存在且非空后回报；TaskManager设置 outputPath、100%、清空 transient error/retry/control/attempt。
- success：完成状态提交后幂等删除 task workspace。若 post-completion cleanup失败，记录受控 cleanup warning供后续启动清理，但不得删除已完成输出或把已完成文件当失败重跑。
- cancel：停止所有工作后删除整个 task workspace；清理失败回报 Failed并保留可重试清理记录，不能显示 Cancelled。
- E009：保留合法 source partial/checkpoint，删除无效 processed；E007/E008：保留合法 source/checkpoint以便手动重试，删除 processed。任何失败均保留原 task format/profile/outputDir。

### 9. 暂停、恢复、取消与重试

- downloading pause：manager设置 pauseRequested，runner转送，downloader停止读取、flush checkpoint，回报 Paused。source解析尚未开始写入时也可直接确认 Paused。
- processing不提供 Pause；沿用状态矩阵仅在 canCancel=true时提供 Cancel。FFmpeg处理不可暂停并恢复。
- paused resume：设置 resumeRequested，待槽位可用后新 attempt进入 downloading，重新解析 fresh URL并验证 checkpoint。
- queued/paused cancel：无 active writer时同步清理并进入 Cancelled；downloading/processing cancel必须走 acknowledgement协议。
- NetworkFailure/E009沿用 TaskManager三次自动重试，间隔1/2/4秒且保留 checkpoint；第四次进入 Failed。其他 error不自动重试。
- failed Retry清 error/nextRetry/automaticRetryCount并进入 queued；新 attempt总是重新验证 auth/source，复用文件前重新验证 checkpoint。
- 旧 attempt在 pause/cancel/retry/重启后回传的 progress/complete/failure全部忽略，不能复活终止态或覆盖新 outputPath。

### 10. 错误与前端反馈

- E007中文“磁盘空间不足或目标目录不可写”，英文等义；操作为释放空间/检查目录后重试。
- E008中文“音频处理失败”，英文等义；可重试。FFmpeg sidecar缺失使用 stable details `FFMPEG_UNAVAILABLE`，UI仍只显示本地化安全文案。
- E009中文“网络中断，已保留下载进度”，英文等义；自动重试中任务回到 queued并保留错误提示，耗尽后显示 Retry。
- E005/E006沿用认证模块文案并提供既有登录入口；不把无损源缺失误写成网络错误。
- 失败、自动重试、取消和完成事件继续使用 `download://progress` 完整 task快照；不创建 `audio://*`事件。

## 设计约束

### 职责与边界

- TaskManager独占 task/attempt/control/persistence/event；runner只协调；AudioExecutor只编排；source/downloader/workspace/processor adapter各自拥有外部副作用。
- Vue component只渲染 props/emits；格式/profile规则集中在 `audio-options.ts`，store协调状态，Tauri调用仍只在既有 service层。
- B 站 raw结构只在 infrastructure adapter；稳定 Rust DTO为前端合同权威。credential/media URL不跨越 adapter或序列化边界。

### 命名与领域语言

- `outputProfile`/`audioBitrateId`表示输出目标，`sourceTier`/`sourceBandwidth`表示实际源；代码、测试和文案不得混称。
- `M4aOriginal`表示音频 stream copy而非不经处理；`FlacLossless`只允许真实无损源。
- `Cancelled`只表示工作已停止且临时项清理成功，`cancelRequested`只是意图。

### 规则归属与重复

- format/profile矩阵在前端 `audio-options.ts` 一处拥有、Rust validation/source policy一处拥有，并通过同一 fixture矩阵契约测试对齐；不在 component、store、executor各写散落条件。
- FFmpeg argv只有一个 builder；error分类只有 adapter mapper；状态转换只有 TaskManager；URL allowlist复用/扩展现有 Bilibili URL validator。
- 音频与视频共享 runner/download/process基础设施只共享真实相同机制；媒体流程分开，禁止为消除语法相似而合并业务编排。

### Side effect与依赖

- HTTP、文件、process、auth和task update通过窄 port注入；单元测试不访问公网、不依赖系统 FFmpeg或真实 credential。
- runner/background task只能由 Tauri composition root创建并在应用生命周期结束时收束；组件/store不得启动后台下载。
- 不新增 WebView shell、filesystem、HTTP capability；FFmpeg由 Rust直接spawn可信路径。

### 复杂度与可读性

- `AudioExecutor::execute`保持线性阶段，阶段细节下沉到具名函数；不得形成包含 HTTP循环、文件路径和 argv构造的超大函数。
- runner不得在持 manager锁时 await；控制转送和handle回收必须显式，避免隐藏 detached task。
- 新文件建议250行进入拆分审查、超过350行必须说明或拆分；测试可因fixture表适度超出但需保持场景分组。
- 不建立通用 plugin framework、DI container、repository、event bus、每格式class或任意 pipeline DSL。

## 项目脚手架与 Starter 决策

本模块不是 greenfield。继续复用现有 Vue 3 + TypeScript + Vite + Pinia + Tauri v2/Rust scaffold、既有 feature/service/store/component分层、TaskManager和Bilibili client。允许新增 Rust service/infrastructure目录及必要 crate feature，不替换框架、UI库、状态管理、router或构建工具；plan/execute不得重新脚手架项目。

## 变化轴与 Pattern 决策

- audio/video执行流程是已确认变化轴，使用轻量 **Strategy**：dispatcher按 DownloadMode选择窄 executor port。直接在runner写 mode大分支会把后续视频双轨流程耦合进协调器，因此不采用。
- B站raw、HTTP、filesystem、FFmpeg是独立不稳定边界，使用 **Adapter/Port** 归一化并提供fake。若某端口最终只有一个纯函数且无外部副作用，应回退直接函数，避免空包装。
- task lifecycle继续使用既有 **State/Command**；新增 cancel acknowledgement是修正真实副作用时序，不建立第二套audio状态机。
- 继续使用既有 **Observer** 事件链，不新增音频事件或通用event bus。
- 格式差异用 `AudioOutputProfile` enum + match/纯argv builder直接表达；三种固定格式不足以证明每格式Strategy class合理，因此明确拒绝。

## 代码上下文与影响假设

- `src-tauri/src/services/tasks/ports.rs` 当前 `TaskExecutionSpec`只有task/attempt/connection标识且 production `DeferredTaskExecutor`不可用；本模块扩展spec和真实runtime接线。
- `TaskManager::claim_next`已处理并发、attempt和状态持久化；`mutations.rs`已处理单调进度、自动重试和迟到事件，但 active cancel当前过早清理，必须改为request/ack。
- `StoredTaskRecord.temporaryPaths`与启动恢复已存在；schema V1新增 checkpoint/workspace语义时优先用 serde default兼容旧文件，若字段形状变化则显式迁移而非静默清空。
- `BilibiliClient::fetch_playurl`与raw DASH目前只读取bandwidth，必须扩展内部raw字段/FLAC分支，但保持 parse命令公共结构受控演进。
- `DownloadPage` store当前从 `result.audioFormats/audioBitrates`选值；需改为 output profile matrix并读取认证/能力，不把规则放进template。
- code graph仍不可用，已有 `artifacts/code-context.md`记录fallback；本规格用 `rg`调用点、关键文件读取和既有task/settings/auth review工件重建影响闭包。

## TypeScript 上下文

- 根 `tsconfig.json` governs `src/**/*.ts|tsx|vue|d.ts`：`strict=true`、`noUnusedLocals/Parameters=true`、`target=ES2020`、`module=ESNext`、`moduleResolution=bundler`、`isolatedModules=true`、`noEmit=true`、`jsx=preserve`。
- 无 `baseUrl/paths` alias，新增导入使用相对路径；ambient declaration只有 `src/vite-env.d.ts` 的 `vite/client`。
- 音频类型来源为 `src/contracts/media.ts`、download-center/task-management contracts及Rust serde field table；没有 protobuf/OpenAPI/generated TS或backend-owned TypeScript可复用。
- `AudioOutputProfile`需要 discriminated/readonly typing，format与profile关联不能退化为任意 string；后端非TS合同保持camelCase字段名镜像。

## API 与数据合同

### 权威来源与适配策略

- 前端 task/parse DTO权威来源：Rust `models/task.rs`、`models/parse.rs` serde字段表；TypeScript直接保留camelCase字段，通过fixture/serialization测试对齐。
- B站外部合同：现有私有 raw field table与fixture是adapter输入；没有稳定官方OpenAPI/TS declaration。Raw字段变化只由 `infrastructure/bilibili` 处理，不在前端复制。
- FFmpeg合同：FFmpeg 7.x CLI argv与process exit；通过 `MediaProcessorPort` adapter，前端不直接消费。
- HTTP合同：reqwest Response/Range headers；通过 `ByteDownloaderPort` adapter，executor只消费稳定 outcome/error。

### ParseVideoResult扩展

新增 required：

```text
audioCapability: {
  maxLossyKbps: number | null,
  losslessAvailable: boolean,
  hiResAvailable: boolean
}
```

- `maxLossyKbps`表示当前 validated auth响应中可用有损源上限；没有普通audio为null。
- `losslessAvailable/hiResAvailable`仅反映当前响应，不替代执行时fresh校验。
- 既有 `audioFormats/audioBitrates`保留一版兼容，但UI输出profile不再以 `audioBitrates` 动态生成；后续移除需独立schema决定。
- loading/error/empty沿用 parse模块；字段缺失的旧demo/test fixture必须显式补齐，production raw适配不能用危险默认把FLAC标为可用。

### CreateDownloadTasksRequest扩展

每个 `DownloadTaskDraft` 新增：

```text
videoTitle: string      // required, parse result title
partCount: number       // required integer >= 1
```

音频draft约束：

```text
mode = "audio-only"
qualityId = null
codec = null
audioFormat = "mp3" | "m4a" | "flac"
audioBitrateId = "128" | "192" | "320" | "source" | "lossless"
```

- format/profile必须匹配；请求仍为 `{request:{requestId,drafts}}`，成功/错误/idempotency/100项原子语义不变。
- 新字段只用于服务端命名；不允许前端提交 `fileName`/outputPath/source URL/FFmpeg args。

### Internal TaskExecutionSpec

```text
{
  task: DownloadTask,
  attemptId: string,
  connectionCount: 1..32,
  temporaryDirectory: absolute trusted path
}
```

- Rust internal required fields，不序列化到WebView。
- task是claim时快照；后续control通过taskId+attemptId，不修改spec。

### ExecutionUpdate扩展

- `WorkspacePrepared { temporaryPaths }`：路径经workspace adapter验证后由manager持久化，仅internal。
- `Progress`、`Paused`、`Processing {canCancel}`、`Completed {outputPath}`、`NetworkFailure {error}`、`Failed {error}` 保持既有含义。
- `Cancelled`：executor确认所有活动I/O/进程停止且workspace cleanup成功；manager转为cancelled并清除attempt/control/temp paths。
- 所有update带外层taskId+attemptId；不匹配或终止态update返回ignored success，不发event。

### AudioSourcePort

```text
resolve({bvid,cid,outputProfile}, ValidatedAuthContext)
  -> AudioSourceBundle { source, metadata }
```

- response中 URL/backupUrls required至少一个、只在内存；contentLength/etag optional；metadata title/uploader/coverUrl required但可由adapter安全fallback。
- timeout/transport error E009，登录 E005，权限 E006，不存在 E004，raw invalid Internal/controlled parse error。

### ByteDownloaderPort

```text
download(DownloadRequest, DownloadControl, ProgressSink)
  -> Completed | Paused | Cancelled
```

- request含已验证source、workspace paths、checkpoint、connectionCount；不含task store引用。
- ProgressSink payload为u64 bytes/total?/speed/eta；executor映射到task 0-90%。
- 网络错误返回typed retryable E009；disk full typed E007；禁止用message string判断类别。

### MediaProcessorPort

```text
process(AudioProcessRequest, ProcessControl)
  -> Completed | Cancelled
```

- request含trusted input/output/cover paths、outputProfile、title、uploader；不接收自由argv。
- E008 details只允许稳定类别，例如 `FFMPEG_UNAVAILABLE|FFMPEG_EXITED|FFMPEG_OUTPUT_INVALID`；UI不显示details。

### 前端 IPC/Event

- 不新增 command或event。继续消费 `create_download_tasks`、`control_download_task`、`download://progress`、`download://removed`。
- task snapshot字段、nullable、byte string、sequence/revision规则不变；音频新行为通过已有mode/format/profile/status/error表达。

## 上下文与依赖来源

- 请求与PRD：`request.md`、`artifacts/prd-snapshot.md`、`requirements/requirement-map.md`、`requirements/modules/audio-download.md`、原始 `biliCatch_PRD.md`。
- 设计：`module-runs/audio-download/design/architecture-design.md`，以及已批准 download-center/task-management page/architecture/spec。
- 代码：download-center store/components/contracts、task models/manager/mutations/ports/store、Bilibili client/raw/adapter、auth context、settings manager、Tauri `lib.rs`。
- 工具链：根 `tsconfig.json`、`src/vite-env.d.ts`、`package.json`、`src-tauri/Cargo.toml`、Tauri config/capabilities。
- 外部运行合同：reqwest 0.12/Tokio 1.x，FFmpeg 7.x；真实B站与系统FFmpeg不作为默认自动测试依赖。

## 边界情况

- settings默认FLAC但匿名/源不支持：M4A回退+一次提示；无音频则format=null且不能入队。
- 用户快速切换format或登录状态变化：最后一次合法意图胜出；失效FLAC不得在迟到parse/auth结果后恢复。
- 解析时FLAC可用、执行时不可用：E006 failed；不生成`.flac`假文件、不自动改`.m4a`。
- 匿名选择MP3 320K：允许输出320K，但source选择<=64K；UI文案不得称为320K源音质。
- 登录过期：fresh auth变anonymous，普通音频按匿名上限继续；FLAC失败E005，不能使用旧cookie。
- source URL过期或403：重新resolve一次fresh URL；仍失败按权限/源错误，不盲目自动重试旧URL。
- Range ignored、416、ETag变化、content-length缺失、partial长于total：按下载合同回退/重置，最多一次，避免无限循环。
- pause与最后一个chunk竞态：若下载已闭合则允许进入processing；pause acknowledgement只在仍可暂停阶段。任一结果由attempt状态机串行决定，不能同时Paused/Completed。
- cancel与FFmpeg退出竞态：cancel intent优先于完成上报，只有final rename已提交且Completed已持久化时cancel不再合法。
- output同名：不覆盖，递增 `(n)`；并发任务最终名选择/创建必须原子避免两任务选到同名。
- outputDir被删除/改权限、临时目录跨卷、Windows文件占用：返回E007/受控I/O错误；关闭handle后再重试/清理。
- cover过大、非图片、请求失败或FFmpeg不支持attached picture：E008并保留source供retry；不得静默省略封面。
- 标题/UP主含换行、非法字符、极长Unicode：metadata作为单argv值，不影响shell；filename规范化后在char边界截断。
- cleanup重复调用、应用崩溃发生在rename前/后：幂等；启动recovery通过最终文件与task状态判定，不删除可信已完成输出。
- task store旧V1记录缺新增draft来源字段：既有task已含fileName，不需反推命名；serde default/migration保证可加载。
- FFmpeg sidecar尚未安装：下载可完成source阶段后E008并保留；系统发布模块装入后Retry无需重新下载源。

## 验收标准

- **AUD-AC-01** 仅音频模式隐藏quality/codec，显示format/profile；格式顺序MP3/M4A/FLAC，MP3可选128/192/320且默认320，M4A显示disabled原始码率，FLAC显示disabled无损。
- **AUD-AC-02** FLAC只有authenticated且source lossless/HiRes可用时启用；匿名标记需登录，登录但源不支持标记不可用。
- **AUD-AC-03** 当前/default FLAC失效时入队前自动回退M4A并显示一次本地化提示；执行时能力变化产生E005/E006且不静默改格式。
- **AUD-AC-04** output profile矩阵在TS和Rust完全一致，非法format/profile或音频draft含quality/codec时整批原子拒绝。
- **AUD-AC-05** 匿名source选择不超过64K，authenticated有损不超过192K并允许明确Hi-Res/无损；MP3输出码率与source tier分离且测试覆盖320K-from-lower-source。
- **AUD-AC-06** 单P文件名为视频标题，多P为 `{page}_{partTitle}`，非法字符/空白/200字符/fallback/同名 `(n)` 均有测试且不覆盖文件。
- **AUD-AC-07** runner只认领有available strategy的mode，遵守settings实时并发/连接数；audio start不阻塞调度，未实现video任务保持queued。
- **AUD-AC-08** 每个attempt获取fresh validated auth、view/playurl；只选择/下载音频轨，任务持久化/event/checkpoint/log/error均不含cookie或media URL。
- **AUD-AC-09** loopback HTTP测试证明206 resume、200 Range fallback、416、ETag/长度不兼容重置、无Range单连接降级和最终byte完整性。
- **AUD-AC-10** 下载流式写盘，progress bytes单调、下载阶段0-90、total未知可表达、速度/ETA规则正确；现有event合并与旧attempt保护不回归。
- **AUD-AC-11** MP3 argv只允许libmp3lame 128/192/320K；M4A为audio stream copy remux；FLAC只接受无损/HiRes source；所有命令不经shell。
- **AUD-AC-12** 三种输出都写入title、artist=UP主和cover；fake/probe测试验证参数与非空输出，失败/空输出/封面不支持映射E008。
- **AUD-AC-13** 下载完成后进入processing，finalize成功且文件非空后才completed/100/outputPath；输出原子落盘且不覆盖同名文件。
- **AUD-AC-14** pause flush checkpoint并进入paused，resume/重启用新attempt恢复兼容partial；processing不显示pause。
- **AUD-AC-15** active cancel先记录intent并停止writer/process，清理成功后才cancelled；queued/paused cancel同步清理；清理失败为failed。
- **AUD-AC-16** success和cancel清空task workspace；E007/E008/E009保留合法source/checkpoint、删除processed；Retry保留原format/profile/path并重新验证源。
- **AUD-AC-17** E009自动重试恰好最多3次且间隔1/2/4秒；E007/E008/E005/E006不自动重试；迟到update不能回退终止态或新attempt。
- **AUD-AC-18** E007/E008/E009和权限错误均有中英文安全文案，UI不展示raw backend message/details/stderr/path/URL，任务页操作矩阵不新增状态。
- **AUD-AC-19** workspace canonical containment、redirect host revalidation、argv array和secret scans通过；WebView不新增shell/filesystem/http capability。
- **AUD-AC-20** 下载中心与任务页在既有desktop/narrow viewport无溢出、重叠或不可达控件；select/disabled状态/inline alert具备label、focus、aria-live语义。
- **AUD-AC-21** TypeScript typecheck、前端单测、Rust fmt/check/test、production build全部通过；现有156前端测试与85 Rust测试无回归（基线若因合法新增测试增长，以全量通过为准）。
- **AUD-AC-22** fake端到端覆盖MP3/M4A/FLAC happy path、metadata、E007/E008/E009、pause/resume/restart/cancel/retry/cleanup；默认CI不依赖公网或系统FFmpeg。
- **AUD-AC-23** real FFmpeg 7 smoke在可信binary存在时验证三容器和封面；binary缺失明确skip并由 `system-release` sidecar打包E2E承接，不误报最终安装包已具备FFmpeg。
- **AUD-AC-24** clean-code review确认runner、TaskManager、AudioExecutor、source/downloader/workspace/processor职责不混杂，format/source/状态/error规则没有漂移复制，Strategy/Adapter边界保持必要且最小。

## 人工评审与交接

- 本规格需要用户明确批准后才能进入plan；批准前 `audio-download.approvals.spec_approved=false`，`state.stage=spec`，`loop.pending_gate=spec_approval`。
- plan必须把 AUD-AC-01..24 映射到contract/option UI、task runtime/control修正、source adapter、HTTP/workspace、FFmpeg、集成/安全/视觉验证任务。
- execute若发现B站当前FLAC raw结构无法从fixture/可控合同可靠识别，必须回到architecture/spec，不得用有损源伪造FLAC或以静默M4A替代已入队FLAC。
- execute若发现Tauri lifecycle无法可靠收束runner或active process，必须回到architecture-design，不得使用无法控制的detached任务。
- audio review PASS后交接 `video-download`；其复用runner/downloader/workspace/process adapter，但不能改变已批准audio profile/cleanup语义。

## 风险

- B站非公开playurl字段与media host会变化，fixture覆盖只能证明adapter合同；real smoke需opt-in且失败不能把凭据写入证据。
- 多连接Range的性能收益依赖CDN；正确性优先，无法证明范围稳定时退化单连接，不能以PRD性能目标驱动不安全并发拼接。
- FFmpeg不同Windows build的encoder/cover支持不完全一致；本模块以7.x为合同，最终随包binary仍需system-release验证。
- task-management active cancel语义调整触及共享状态机，是本模块最大回归面；必须保留全部task tests并增加下载/processing竞态测试。
- 工作区恢复与最终rename跨崩溃点复杂；实现必须用少量明确状态/文件存在性规则，不引入不可审查的通用事务框架。
- 输出MP3码率高于source码率可能造成用户误解；规格通过术语、提示和测试约束避免把编码参数宣传为源音质。

