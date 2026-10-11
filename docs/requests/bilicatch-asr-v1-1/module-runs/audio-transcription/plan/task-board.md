# `audio-transcription` 任务板

## 执行规则

- 当前门禁：等待本计划批准；批准前所有任务均不得进入代码实现。
- 执行人：当前 Codex 主线程；不启用多代理或 workflow。
- 每项按 red -> green -> refactor -> targeted verify 执行，完成后更新状态和证据。
- 状态值：`blocked`、`ready`、`in_progress`、`done`、`failed`。

| ID | 任务名称 | 状态 | 执行模式 | 前置任务 | 规格/验收映射 |
| --- | --- | --- | --- | --- | --- |
| T01 | 合同、错误码与设置迁移 | done | 串行 | 计划批准（已满足） | API/设置/错误；AC-07、AC-09 |
| T02 | 模型清单与本地仓库 | done | 串行 | T01（已满足） | 模型目录与下载；AC-03 |
| T03 | 状态机、持久化与恢复 | done | 串行 | T01（已满足） | 任务合同与状态机；AC-05、AC-08 |
| T04 | 来源检查与 FFmpeg 预处理 | done | 可并行候选，实际串行 | T03（已满足） | 来源/预处理；AC-02、AC-05 |
| T05 | whisper sidecar 适配器 | done | 可并行候选，实际串行 | T03（已满足） | sidecar/语言标点；AC-04、AC-05、AC-10 |
| T06 | 调度、字幕、缓存与导出 | done | 串行 | T02、T04、T05（已满足） | 调度/字幕/结果/导出；AC-04、AC-06 |
| T07 | Tauri 命令、事件与自动钩子 | done | 串行 | T06（已满足） | API/下载联动/通知；AC-02、AC-07、AC-08 |
| T08 | 前端 IPC、事件与 Pinia | done | 串行 | T01、T07（已满足） | API/联合任务；AC-01、AC-08、AC-09 |
| T09 | 转写工作台与结果编辑 | done | 串行 | T08（已满足） | 页面/表单/结果；AC-01、AC-02、AC-03、AC-06 |
| T10 | 任务、设置、下载与 i18n 集成 | done | 串行 | T08、T09（已满足） | 跨模块集成；AC-07、AC-08 |
| T11 | 分发、真实 smoke 与全量验收 | done | 串行 | T01-T10（已满足） | 分发/全部验收；AC-04、AC-09、AC-10 |

## T01 合同、错误码与设置迁移

- 执行说明：主线程正在执行；按测试先行完成 Rust/TypeScript 合同与设置迁移。
- 覆盖功能：E011-E017、转写/模型/结果 DTO、设置 schema v2、11 个字段、v1 兼容迁移、输入校验。
- 范围：Rust models/settings/error，TypeScript transcription types/settings types 和对应测试。
- 整洁性约束：Rust serde DTO 为权威；验证器纯函数；不修改 `DownloadTask` 语义。
- 模式边界：领域类型不依赖 Tauri；前端类型不包含视图临时态。
- 上下文/影响：设置 v1 fixture 是关键回归面，字段名必须与现有 IPC camelCase 一致。
- 交互/状态：保存设置须原子完成；失败保留旧设置；默认 Base 但不触发下载。
- API/类型：规格命令和事件 DTO；前端镜像并用 contract fixture 检查。
- 测试切入：先写迁移、非法 URL/并发数、默认值、序列化失败测试。
- 已批准假设：自定义模型源 trim 后必须 HTTPS，且无 query/fragment；并发 1-3。

## T02 模型清单与本地仓库

- 执行说明：主线程正在执行；先固定 manifest 与仓库测试，再实现文件/网络 adapter。
- 覆盖功能：五种模型、manifest、列表、下载、进度、取消、校验、删除和目录切换。
- 范围：Rust model repository/downloader，manifest/resource，commands 所需应用服务接口。
- 整洁性约束：网络/文件哈希在 adapter；manifest 静态版本化；不让 UI 判断文件可用性。
- 模式边界：`ModelRepository` port 隔离实际文件系统和 HTTP 客户端。
- 上下文/影响：模型不进安装包；目录迁移和已存在文件需避免覆盖。
- 交互/状态：`.part` -> 校验 -> 原子 rename；下载中禁止重复下载，使用中禁止删除。
- API/类型：`list_asr_models`、`download_asr_model`、`cancel_asr_model_download`、`verify_asr_model`、`delete_asr_model`。
- 测试切入：本地 HTTP fixture 的成功、断流、坏摘要、取消、已有有效文件。
- 待确认项：执行阶段必须从固定官方资产复核实际 URL、size、SHA-256 和许可证后写入 manifest。

## T03 状态机、持久化与恢复

- 执行说明：主线程正在执行；先固定状态表，再实现 store/manager 与恢复。
- 覆盖功能：独立实体、九状态、动作矩阵、revision/sequence、重启恢复、清理完成任务。
- 范围：Rust domain/store/manager/event sink contracts。
- 整洁性约束：状态转换纯函数；持久化成功后才发布事件；损坏文件不得静默丢弃。
- 模式边界：manager 管实体，scheduler 管并发，二者不合并为下载 manager。
- 上下文/影响：复用下载任务序号模式但使用独立文件和类型。
- 交互/状态：运行态重启后转为可重试失败；终态保持；非法动作不变更 revision。
- API/类型：为任务列表/控制/清理命令提供应用层接口。
- 测试切入：状态表参数化、重复动作、持久化失败、损坏恢复、陈旧事件。
- 已批准假设：转写持久化与下载持久化物理隔离。

## T04 来源检查与 FFmpeg 预处理

- 执行说明：主线程正在执行；测试来源校验、probe、转码参数和清理守卫。
- 覆盖功能：本地文件、已完成下载任务、媒体探测、16k mono s16 WAV、长音频分片、清理。
- 范围：Rust source inspector、`AudioPreprocessor` port/adapter 和 fixtures。
- 整洁性约束：不修改源媒体；临时文件全部受 cleanup guard 管理。
- 模式边界：复用 runner/locator 习惯，不复用下载专用 `FfmpegMediaProcessor` 领域接口。
- 上下文/影响：支持列表严格按规格；无音轨和未完成下载在创建前拒绝。
- 交互/状态：preprocessing 进度可见；取消/失败清理；静音探测失败退固定切分。
- API/类型：`inspect_transcription_source`；内部返回规范 source metadata/chunks。
- 测试切入：格式矩阵、无音轨、命令参数、30 分钟边界、时间偏移、取消清理。
- 已批准假设：静音边界允许在目标点前后 5 秒搜索。

## T05 whisper sidecar 适配器

- 执行说明：T04 后执行，虽然依赖上可与 T04 并行。
- 覆盖功能：v1.8.6 定位、能力、CPU/GPU 策略、隐藏窗口、进度/结果解析、进程树取消。
- 范围：Rust `Transcriber` port/whisper adapter、Tauri sidecar locator、parser fixtures。
- 整洁性约束：命令参数只在 adapter；日志有长度上限且不记录用户音频内容。
- 模式边界：应用服务依赖 `Transcriber`，不得依赖具体 CLI。
- 上下文/影响：sidecar 缺失不能让 `lib.rs` setup 失败。
- 交互/状态：loadingModel/transcribing/processing；auto GPU 可回退，enabled 不可回退时 E014。
- API/类型：内部 `TranscriptionRequest -> TranscriptSegments`；错误映射 E014/E015。
- 测试切入：fake executable、参数快照、输出 fixture、非零退出、不可解析、取消进程树。
- 待确认项：用固定发布二进制验证最终 CLI 参数和平台文件名。

## T06 调度、字幕、缓存与导出

- 执行说明：T02/T04/T05 全部完成后执行。
- 覆盖功能：并发、优先级、平台字幕回落、缓存、结果 revision、五种格式、复制/打开所需结果信息。
- 范围：Rust application services、scheduler、cache、subtitle/export adapters。
- 整洁性约束：识别缓存与输出渲染分离；格式器纯函数；网络字幕仅在 adapter。
- 模式边界：`SubtitleProvider` 三态合同；scheduler 不知道 Vue 或下载实现细节。
- 上下文/影响：仅 BVID/CID 下载来源尝试字幕；活跃下载只影响优先级，不暂停现有下载。
- 交互/状态：pending 到 saving/终态；失败可重试；编辑只改文本不改 timestamps。
- API/类型：结果读取/保存/导出/open commands 的服务层合同。
- 测试切入：并发 1-3、公平性、字幕 Available/Unavailable/Failed、缓存指纹、force、golden files、冲突命名。
- 已批准假设：格式/时间戳显示选择不进入识别缓存键；源 mtime 变化使缓存失效。

## T07 Tauri 命令、事件与自动钩子

- 执行说明：T06 后执行。
- 覆盖功能：13 个命令、3 类事件、通知、下载完成自动转写和幂等。
- 范围：Rust commands、event sink、`lib.rs` 装配、download runtime completion sink。
- 整洁性约束：command 无业务分支；下载域只发布完成事实，不持有转写 manager。
- 模式边界：显式 completion sink 和 activity probe；禁止前端轮询触发。
- 上下文/影响：既有下载 completed 状态和通知必须保持；启动时仅注册 sidecar 能力。
- 交互/状态：幂等键 `download:<downloadTaskId>:<outputRevision>`；转写失败不反写下载失败。
- API/类型：严格匹配规格命令名和事件名，错误统一序列化。
- 测试切入：command contract、payload、事件顺序、重复 hook、禁用设置、缺模型、sidecar 缺失启动。
- 已批准假设：自动转写绝不静默下载模型。

## T08 前端 IPC、事件与 Pinia

- 执行说明：T07 后执行。
- 覆盖功能：service、event source、store、格式化、联合任务投影、依赖注入。
- 范围：`src/features/transcription/**`、task view model、`src/main.ts`。
- 整洁性约束：组件无 `invoke/listen`；store 不拼路径或 sidecar 参数。
- 模式边界：`TaskListItem` 判别联合只用于展示，不成为新的后端实体。
- 上下文/影响：沿用下载 feature 的 transport/event 注入模式。
- 交互/状态：先快照后事件；sequence/revision 拒绝乱序；错误不清空已有实体。
- API/类型：TypeScript 镜像 Rust serde DTO；mapper 处理日期、错误和 UI 展示字段。
- 测试切入：命令名/参数、初始化、乱序、重复、删除、unsubscribe、联合动作分发。
- 已批准假设：刷新时以 Rust 快照为权威，未保存页面草稿除外。

## T09 转写工作台与结果编辑

- 执行说明：T08 后执行。
- 覆盖功能：路由/侧栏、来源表单、模型管理、任务进度、结果编辑/复制/导出/打开/重转写。
- 范围：TranscribePage、转写组件、样式和组件测试。
- 整洁性约束：无卡片套卡片；采用现有 Naive UI/icon；图标动作有 tooltip；固定控件尺寸防布局跳动。
- 模式边界：页面编排 store，组件通过 props/emits，不直接依赖 transport。
- 上下文/影响：AppRouteName 增加 `transcribe`；桌面双栏、窄屏单栏。
- 交互/状态：inspect 完成才提交；删除/清理确认；保存失败保留草稿；显式取消模型下载。
- API/类型：只消费 T08 feature API；不增加页面私有 IPC。
- 测试切入：文件/下载来源、拖放、模型缺失、提交 loading、失败重试、编辑 revision、长文本、键盘焦点。
- 已批准假设：页面首次进入不自动下载模型，也不自动开始转写。

## T10 任务、设置、下载与 i18n 集成

- 执行说明：T09 后执行，减少共享 store/i18n 冲突。
- 覆盖功能：任务联合展示、设置 11 字段、下载后转写选项、中英文文案。
- 范围：TasksPage、SettingsPage、DownloadPage/download feature、navigation/sidebar、locale files。
- 整洁性约束：领域动作按 kind 分发；既有下载组件保持职责；文案不散落硬编码。
- 模式边界：联合只在展示层，两个后端 manager 和持久化不合并。
- 上下文/影响：三种下载模式、批量暂停/清理、草稿序列化是高回归面。
- 交互/状态：自动转写默认关；设置 trim/校验；清理转写不删源媒体；每类任务显示自己的动作。
- API/类型：使用已有 settings service 扩展和 T08 task projection。
- 测试切入：设置 round-trip、任务 kind 动作、下载 draft、导航 active state、locale key 完整性。
- 已批准假设：下载页面选项只改变新任务意图，不批量追溯旧任务。

## T11 分发、真实 smoke 与全量验收

- 执行说明：全部代码任务完成后执行；结果进入 verify/review。
- 覆盖功能：externalBin、许可证、Windows 真实识别、取消/恢复、缺失能力启动、跨平台交接、视觉可访问性。
- 范围：Tauri config/resources、license docs、smoke fixtures/scripts、验收证据。
- 整洁性约束：不提交模型大文件或用户输出；fixture 小且许可证清楚。
- 模式边界：mock 测试与真实 smoke 证据分开，不以模拟结果替代 AC-04。
- 上下文/影响：externalBin 命名会影响现有 FFmpeg 与 updater/打包，必须回归。
- 交互/状态：真实任务输出非空、时间戳单调；取消无残留；缺 sidecar 可启动并给出 E014。
- API/类型：不新增业务合同，只验证已冻结命令/事件和分发配置。
- 测试切入：全量四门禁、真实 sidecar/model 短音频、桌面/窄屏截图、键盘流程、macOS/Linux locator/config tests。
- 待确认项：macOS/Linux 实机签名和运行属于外部平台交接；缺其证据时只标注平台待验，不夸大为已通过。
