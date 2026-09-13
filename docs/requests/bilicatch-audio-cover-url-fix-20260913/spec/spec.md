# 工程规格：仅音频封面 URL 规范化

## Delivery Unit Identifier

`bilicatch-audio-cover-url-fix-20260913`

## Background And Goals

目标视频 `BV1FNb366EH2` 当前提供可下载的普通 M4A 音轨，但 view 接口把封面返回为 `http://i2.hdslb.com/...`。仅音频 source resolve 在音轨下载前验证封面 URL，现有安全规则只允许 HTTPS，因此返回 `E004`。本交付单元在 Bilibili adapter 边界安全升级受信任 HTTP 封面，不降低全局 URL 安全等级。

## In Scope

- 为 audio metadata 增加封面 URL 规范化与验证。
- 将 `http://` 封面升级为等价 `https://` 后再执行既有 host、凭据、fragment 与 scheme 校验。
- 让 `adapt_audio_metadata` 显式返回成功或 `AppError`，由 `ParserService` 传播稳定错误。
- 为 raw HTTP、既有 HTTPS 和不安全 URL 添加回归测试。
- 对 `BV1FNb366EH2` 执行可选公网 source-resolve smoke，确认音轨与封面路径同时通过。

## Out Of Scope

- 不放宽音视频媒体源为 HTTP。
- 不允许任意外部封面主机。
- 不修改下载目录、任务 IPC、下载模式、音频格式/码率、登录或错误码文案。
- 不增加 URL proxy、封面缓存或新的 retry 策略。
- 本轮不自动推送代码或发布 Release。

## Trigger And Start Conditions

- Trigger：下载中心已解析视频，用户选择“仅音频”并加入队列。
- Preconditions：所选 part 存在，playurl 提供与所选输出 profile 匹配的音频候选。
- Start：`AudioExecutor` 调用 `AudioSourcePort::resolve` fresh resolve 当前 BVID/CID。

## Requirement Split Summary

这是非 PRD、非拆分的单一缺陷修复。唯一功能单元是 Bilibili audio metadata 封面 URL 兼容，不引入 module flow。

## User Flow

1. 用户解析 Bilibili 视频。
2. 用户选择“仅音频”以及 M4A/MP3/允许的 FLAC profile。
3. 任务入队并由 audio executor claim。
4. ParserService 获取 fresh view/playurl。
5. Adapter 将 raw HTTP 封面升级为 HTTPS，并通过既有安全校验。
6. Source bundle 返回 executor；executor 下载封面、音轨并处理输出。
7. 真正无音轨或不安全 URL 继续返回稳定错误。

## Page And Module Design

- 页面布局、控件与交互结构不变。
- `infrastructure/bilibili/audio_source.rs`：拥有 raw view 到稳定 `AudioMetadata` 的转换与封面规范化调用。
- `infrastructure/bilibili/media_url.rs`：继续拥有 URL scheme、host、凭据与 fragment 的安全裁决；可增加一个窄范围的 cover normalization helper，但不得改变 `validate_media_url` 的既有成功集合。
- `services/parser.rs`：只编排并传播 adapter 结果，不复制 URL 字符串规则。
- `services/audio/executor.rs`：不感知 raw HTTP 兼容逻辑，不改副作用顺序。

## Function-Complete Behavior Breakdown

### Audio metadata adaptation

- 输入：Bilibili `ViewData.pic` 原始字符串。
- `http://`：构造保持 host、port、path 和 query 等价的 HTTPS URL，再用既有 validator 检查。
- `https://`：保持等价并用既有 validator 检查。
- 非 HTTP(S)、无 host、用户名/密码、fragment、外部 host：返回 `E004`，不得输出 source bundle。
- 输出：`AudioMetadata.cover_url` 必须是通过 `validate_media_url` 的 HTTPS URL。

### Audio source resolution

- view part 与音轨选择规则保持不变。
- 音轨存在且封面可规范化时，返回 `AudioSourceBundle`。
- 音轨不存在仍由 `select_audio_source` 返回 `E004`。
- lossless 的 E005/E006 行为保持不变。
- adapter 的封面错误直接以安全 `AppError` 传播，不泄露 raw URL。

### UI outcome

- 成功路径不再显示“视频不可用”，任务进入 workspace/download/processing 状态。
- 真实 E004 仍使用当前本地化文案；本交付不改错误文案契约。

## Design Constraints

- Responsibility：raw Bilibili 兼容属于 adapter，不进入 Vue/store 或 executor。
- Rule ownership：host/scheme 最终校验仅由 `validate_media_url` 所有；新增逻辑只能规范化后复用该 validator。
- Side effects：规范化 helper 必须纯函数；网络与文件副作用仍留在现有 client/downloader/executor。
- Naming：使用 `cover_url`、`normalize`、`validate` 等现有领域语言，避免泛化为 manager/handler。
- Duplication：不得在 ParserService 与 AudioExecutor 各写一份 HTTP -> HTTPS 规则。
- Complexity：采用一个窄 helper 与显式 `Result`，不引入新 trait、factory 或配置层。
- Security：不得通过把全局 validator 改为接受 HTTP 来修复。

## Project Bootstrap And Scaffold Decision

不适用。现有 Vue/Tauri 项目、模块与测试框架继续使用，不新增脚手架或依赖。

## Change Axes And Pattern Decision

- 变化轴只有 Bilibili raw cover URL 的 scheme 兼容。
- 继续使用现有 Adapter 边界；这是既有结构，不新增模式层。
- 直接纯函数足够，Strategy/Factory/Manager 均不成立且禁止引入。

## Code Context And Impact Assumptions

- `adapt_audio_metadata` 当前唯一生产 caller 是 `ParserService` 的 `AudioSourcePort::resolve`。
- audio executor 在 source resolve 后才准备 workspace，因此当前失败不会产生下载文件。
- video-only 不消费 audio metadata；video+audio 当前也不下载封面，因此行为不变。
- code graph 不可用，限定 symbol search 已确认 caller 和邻近测试。

## API And Data Contracts

- 权威外部合同：Bilibili view JSON 的 `data.pic: string` 与 playurl `dash.audio[]`；通过私有 Rust raw DTO 消费。
- Adapter boundary：`ViewData` 不暴露到前端，映射为稳定 `AudioMetadata`。
- 内部变更：`adapt_audio_metadata(&ViewData)` 从直接返回 `AudioMetadata` 改为 `Result<AudioMetadata, AppError>`；仅 crate-private caller 需要适配。
- IPC：`ParseVideoResult`、`DownloadTaskDraft`、Tauri commands 和 TypeScript contracts 不变。
- Error：不安全封面为 E004；网络 E001/E002 到 source resolve 的 E009 映射规则不变。
- TypeScript context：本次预计不改 TypeScript。若执行中确需修改，必须遵循根目录 strict `tsconfig.json`（ES2020、bundler resolution、noEmit）及现有 contracts 后再实施。

## Context And Dependency Sources

- 用户缺陷描述与历史测试 URL。
- Bilibili 官方 view/playurl/CDN 当前响应。
- `audio_source.rs`、`media_url.rs`、`parser.rs`、`audio/executor.rs`。
- `src-tauri/tests/audio_source.rs`、parser 内部测试和 audio executor 集成测试。

## Edge Cases

- HTTP 受信任子域：升级并通过。
- HTTPS 受信任子域：保持并通过。
- HTTP/HTTPS 外部域：E004。
- URL 中有用户名、密码或 fragment：E004。
- 相对 URL、空字符串、非 HTTP(S)：E004。
- query token：规范化后保持，不记录、不泄露到错误详情。
- 目标视频未来不再提供音轨：仍为真实 E004，公网 smoke 不作为确定性单测。

## Acceptance Criteria

- **AUDCOV-AC-01**：HTTP `*.hdslb.com` 封面被规范化为等价 HTTPS，path/query 保持。
- **AUDCOV-AC-02**：既有 HTTPS `*.hdslb.com` 封面仍成功且无语义变化。
- **AUDCOV-AC-03**：外部 host、带凭据、fragment、相对/空/非 HTTP(S) URL 均返回 E004。
- **AUDCOV-AC-04**：含 HTTP 封面和普通音轨的 parser fixture 能成功 resolve `AudioSourceBundle`，其 cover URL 通过 `validate_media_url`。
- **AUDCOV-AC-05**：普通 M4A/MP3、lossless auth gate、video source 和 downloader 既有测试全部通过。
- **AUDCOV-AC-06**：`BV1FNb366EH2` 的 opt-in 公网 smoke 能看到音轨并得到 HTTPS 封面；若外部接口临时不可用，必须单独记录，不替代离线验收。
- **AUDCOV-AC-07**：`cargo fmt --check`、全量 Rust tests、前端 tests/typecheck 通过。

## Human Review And Handoff

- 用户批准本规格后进入 plan。
- 用户批准计划后才允许写失败测试和生产实现。
- 验证与评审通过后交付改动；推送和 Release 需要单独明确请求。

## Risks

- Bilibili 可能未来迁移图片域名；未在白名单中的新域名会继续安全失败，需要基于证据单独扩展。
- 公网 URL 与 token 会过期；确定性验证必须使用 fixture。
- 通用 E004 文案不区分视频、音轨或封面，本轮保持兼容，后续可单独设计更精确的用户提示。
