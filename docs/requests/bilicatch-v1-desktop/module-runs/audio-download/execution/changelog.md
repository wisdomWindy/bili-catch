# 执行记录：音频独立下载

## 执行约束

- 规格批准：是；计划批准：是；当前阶段：execute。
- 固定顺序：AUD-01 -> AUD-08，单agent串行。
- TDD：所有可测试行为先写red、观察预期失败，再实现最小green并重构。
- 当前目录无Git元数据，不能建立worktree/commit；用task-board与本记录作为检查点。
- 所有shell命令以 `rtk` 开头；Rust使用 `C:/Users/yangjianlin/.cargo/bin/cargo.exe`。

## AUD-01

- 状态：completed。
- 目标：冻结跨端profile、capability、draft与文件命名合同。
- Red 1：TS因缺少 `audio-options` import失败；Rust因缺少 `AudioOutputProfile`/`sanitize_audio_filename`编译失败。
- Green 1：新增固定profile矩阵、Rust领域类型和单/多P音频命名纯函数；TS 2项、Rust 6项通过。
- Red 2：store仍返回M4A=`192`且draft缺命名字段；Rust缺 `AudioCapability`/draft字段并无法校验profile。
- Green 2：扩展serde/TS合同、validation、manager命名与保守capability适配；定向TS 22项、Rust 22项通过。
- 收口：补完整task-drafts fixture；`cargo fmt`因环境文件替换拒绝访问，按其diff用补丁应用后 `fmt --check`通过。
- 全量门禁：Vitest 44文件/158项通过；Vue typecheck+Vite build通过；Rust 86项通过（49 unit + 37 integration，另1个公网smoke ignored）。

## AUD-02

- 状态：completed。
- 目标：交付支持模式认领、完整execution spec、runner和active cancel acknowledgement。
- Red：Rust integration因缺download_runtime、`supports`、temp settings、supported claim、pending control和Cancelled update编译失败。
- Green：新增可单步runner、supported-mode claim、完整internal spec、control查询和WorkspacePrepared/Cancelled；active cancel等待executor ack。
- 定向门禁：download_runtime 1项、task manager 16项、task rules 7项、task store 5项通过；Cargo check/fmt通过。
- 结构记录：production composition/run-loop/shutdown按计划保留AUD-08；本任务先以tick提供deterministic runtime seam。

## AUD-03

- 状态：completed。
- 目标：交付fresh B站音频source、能力选择、metadata和URL安全适配。
- Red 1：跨模块测试因缺少 `services::audio`、source selector与media URL validator编译失败。
- Green 1：匿名源限制到64K、登录有损限制到192K；FLAC使用E005/E006；HTTPS CDN suffix与userinfo/loopback拒绝矩阵4项通过。
- Red 2：私有DASH fixture因缺少raw URL/FLAC字段和adapter函数编译失败。
- Green 2：兼容snake/camel raw字段，适配标准与Hi-Res无损候选、metadata fallback，并让parse capability/FLAC format反映当前响应。
- Red 3：ParserService尚未实现 `AudioSourcePort`，fresh-attempt测试编译失败。
- Green 3：复用现有auth/WBI/signature retry，每个resolve重新请求auth/view/playurl；BVID/CID、cover/source/backup URL均在后端校验，transport归一化为E009。
- 定向门禁：Rust lib 53 passed + 1 ignored，audio_source integration 4 passed，`cargo fmt --check` passed；secret/URL静态扫描仅命中内存合同与adapter赋值。

## AUD-04

- 状态：completed。
- 目标：交付受控workspace、无敏感字段checkpoint、Range恢复、进度/暂停/取消与原子finalize。
- Red 1：workspace integration因缺少 `infrastructure::audio` 编译失败。
- Green 1：哈希task workspace、同卷processed、V1原子checkpoint、断点失配归零、同名 `(n)` 不覆盖、幂等受限cleanup共4项通过。
- Red 2：download contract因缺少control/progress/downloader port编译失败。
- Green 2：原子pause/cancel（cancel优先）、0-90进度映射与typed port共2项通过。
- Red 3：loopback HTTP fixture因缺少 `HttpByteDownloader` 编译失败；首轮实现另由短读测试发现响应长度覆盖已知源长度的问题。
- Green 3：206起点/总长校验、200忽略Range单次归零、416完整确认、短读/错误range E009、音频MIME预检、redirect重验与pause/cancel刷盘；loopback 7项通过。
- 全量门禁：Rust 60 unit + 50 integration passed，1个公网smoke ignored；`cargo check --all-targets`与`cargo fmt --check` passed；checkpoint静态扫描无URL/cookie/header字段。

## AUD-05

- 状态：completed。
- 目标：交付可信FFmpeg定位、三种输出profile参数、可取消子进程与安全E008映射。
- Red 1：processor integration因缺少process contract、argv builder与locator编译失败。
- Green 1：MP3仅允许libmp3lame 128/192/320K，M4A stream copy，FLAC要求真实无损tier；metadata与路径均为独立argv；显式绝对locator矩阵4项通过。
- Red 2：进程边界测试因缺少 `FfmpegMediaProcessor` 编译失败。
- Green 2：Tokio `Command` 直接spawn、无shell/PATH扫描、预取消、输入预检、启动/退出/空输出稳定E008分类共6项通过。
- Red 3：fake success/empty/cancel测试因缺少runner seam编译失败。
- Green 3：生产Tokio runner与测试fake共用processor校验；success非空、空输出E008、cancel清理残留共2项通过。
- 环境记录：初始“无效exe内容”夹具在Windows产生2个挂起测试进程，确认PID后已仅终止这2个进程；改用“定位后删除exe”稳定覆盖spawn失败。
- 全量门禁：Rust 62 unit + 52 integration passed，1个公网smoke ignored；`cargo check --all-targets`与`cargo fmt --check` passed；静态扫描确认仅 `Command::new(executable)`，stderr丢弃且无shell/PATH调用。

## AUD-06

- 状态：completed。
- 目标：交付source -> download -> processing -> finalize -> cleanup线性编排、reporter状态回写与控制竞态。
- Red 1：fake end-to-end因缺少cover/reporter ports与 `AudioExecutor` 编译失败。
- Green 1：fresh source -> workspace/register -> cover/source -> 0-90 progress -> Processing -> process -> atomic finalize -> Completed -> cleanup线性流程；MP3/M4A/FLAC、pause/cancel、E007/E009共4项通过。
- Green 2：`TaskExecutorPort`非阻塞start、supported mode、active controls与后台handle；修正duplicate start覆盖原控制句柄的竞态，start/duplicate/pause测试通过。
- Green 3：processing cancel使用共享control，processor确认后先cleanup再报告Cancelled；有界异步等待测试通过。
- Green 4：cover downloader限制HTTPS重验、image MIME、16MiB和非空内容；image/text loopback测试通过，失败为安全E008。
- 全量门禁：Rust 63 unit + 58 integration passed，1个公网smoke ignored；`cargo check --all-targets`与`cargo fmt --check` passed。

## AUD-07

- 状态：completed。
- 目标：交付仅音频格式/profile联动、FLAC权限回退、精确draft、任务副信息与中英文安全错误。
- Red 1：audio options测试因缺少format/profile owner、FLAC授权规则和默认回退失败。
- Green 1：新增固定MP3/M4A/FLAC矩阵、authenticated/lossless能力判断及一次性M4A回退提示；draft精确携带format/profile和命名字段。
- Red 2：页面/任务测试证明raw backend `message` 会直接进入UI，且音频副信息和disabled原因缺失。
- Green 2：下载页与任务页只按稳定error code映射双语文案；任务行呈现format/profile，组件维持typed props/emits边界。
- 视觉证据：1280匿名浅色、1280登录浅色、800登录深色英文三组DOM/截图均无溢出、裁切或小于36px控件；匿名FLAC disabled并显示需登录，登录无损可选择。
- 定向与全量门禁：Vitest 45文件/166项通过；production build通过；视觉脚本3场景通过。

## AUD-08

- 状态：completed。
- 目标：完成production composition、生命周期、全量门禁、安全扫描和AUD-AC证据入口。
- Green 1：`lib.rs` 组装共享ParserService/TaskManager、AudioExecutor、HTTP/workspace/deferred FFmpeg与100ms runner；Tauri Exit通过AtomicBool停止后续tick。
- Green 2：每次attempt重新解析过期source一次；Range resume在ETag/总长变化时清空旧partial后单次从零重启；无已证实Range能力时保守单连接。
- Green 3：启动只清理内部 `.bilicatch-*.processing.*`，保留source/checkpoint；processing/finalize失败删除partial output并保留可恢复下载。
- Review回流：新增取消/完成竞态测试，确认已持久化CancelRequested优先于迟到Completed，并清理刚生成的最终文件；测试先失败后修复通过。
- 整洁性回流：拆出HTTP封面适配、响应支持函数、executor runtime和progress adapter；核心executor 341行、HTTP生产段317行，测试夹具保持场景分组。
- 最终门禁：前端166项；Rust 135项通过、1项公网opt-in ignored；Rust fmt/check、production build、三组视觉检查全部通过。
- 延后项：可信FFmpeg 7 sidecar及真实三容器smoke由 `system-release` 打包E2E承接，不把本模块的deferred processor误报为最终安装包能力。
