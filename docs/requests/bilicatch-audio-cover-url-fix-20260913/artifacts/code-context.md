# Code Context

## Context Requirement

- bugfix intake 首次需要跨页面、task runtime、source resolver 与 Bilibili adapter 追踪 `E004` 来源。
- 根因确认前 code graph 为优先选择；根因落到单一 adapter 边界后，限定 symbol search 足以确认影响面。

## Graph Availability Check

- Repository graph status：missing。
- Detection method：检查可用工具/技能和仓库内 graph、SCIP、LSIF、CodeQL 配置。
- Tool or runtime found：无可用 repository code graph。
- Check context：intake iteration 1，2026-09-13。

## Installation Or Bootstrap Record

- Attempted：no。
- Method：仓库没有团队指定的 graph bootstrap 或可复用配置。
- Result：not_needed。
- Output summary：使用限定 `rg` symbol/call-site 搜索和完整关键文件读取恢复调用链。
- Next step：由 adapter 单元测试和 parser/audio executor 定向回归验证影响面。

## Fallback Record

- Fallback used：yes。
- Fallback method：`rg` 调用点搜索、关键 Rust/TypeScript 文件读取、官方接口最小诊断。
- Why fallback was needed：无可用 code graph，且根因可由单一数据流稳定复现。
- Residual confidence：高；仍需通过测试证明不安全主机和既有 HTTPS 地址不回归。

## Relevant Entrypoints

- 用户入口：`DownloadPage.vue` 的仅音频选择与入队。
- draft 入口：`useDownloadCenterStore.buildDrafts()`。
- runtime 入口：`AudioExecutor::run_attempt()`。
- source 入口：`AudioSourcePort for ParserService::resolve()`。
- adapter 入口：`adapt_audio_metadata()`。
- 安全边界：`validate_media_url()`。

## Key Symbols And Modules

- `AudioExecutor::run_attempt`
- `ParserService::resolve(AudioSourceRequest)`
- `adapt_audio_metadata`
- `validate_media_url`
- `HttpByteDownloader::download_cover`

## Dependency And Side-Effect Boundaries

- Vue 只创建任务，不直接访问 Bilibili 或文件系统。
- ParserService 编排 auth、view、playurl 和稳定 source bundle。
- Bilibili adapter 拥有 raw 字段兼容和外部 URL 规范化。
- URL validator 拥有 HTTPS/域名安全规则，不应为兼容 HTTP raw 值而放宽。
- AudioExecutor 与 downloader 拥有文件和网络副作用。

## Impact Scope

- 预计生产改动集中在 Bilibili audio metadata adapter 或其专用 helper。
- 测试覆盖 HTTP 受信任封面升级、HTTPS 保持、恶意/不安全 URL 拒绝和目标 source resolve。
- 回归敏感邻居：解析结果封面展示、video modes、媒体 URL 白名单和 cover downloader。

## Open Follow-Up Checks

- 在 execute 阶段用 TDD 确认 adapter 输出，再运行目标视频联网 smoke。
- 若规范化 helper 同时服务 parse result，需在规格中明确其共享范围后才能实施。
