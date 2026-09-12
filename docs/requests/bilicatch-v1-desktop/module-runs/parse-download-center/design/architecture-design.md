# 架构设计：解析与下载中心

## 交付单元标识

`parse-download-center`

## 架构目标

在既有 AppShell/IPC 契约之上新增一个纵向 download-center feature。前端拥有表单与选择状态，Rust 拥有输入规范化、短链解析、WBI 签名、B 站请求/响应适配和 12 小时缓存；任何外部字段都不得泄漏到 Vue 页面。

## 模块边界

### 前端

- `features/download-center/contracts.ts`：ParseVideoRequest/Result、媒体能力、DownloadTaskDraft。
- `features/download-center/store.ts`：输入、ParseState、结果、选中分 P、模式与参数；缓存以 canonical video key 为键。
- `features/download-center/service.ts`：通过现有 IpcTransport 调用 `parse_video`，只做 AppError 归一。
- `features/download-center/components/`：InputPanel、VideoSummary、PartSelector、DownloadOptions、ActionBar。
- `pages/DownloadPage.vue`：组合 feature，不直接 invoke、不解析 URL、不映射外部 JSON。
- `stores/task-drafts.ts`：最小会话级 `DownloadTaskDraft[]` 交接队列；不拥有进度、持久化、重试或并发规则。

### Rust

- `models/parse.rs`：稳定 command DTO。
- `commands/parse.rs`：薄 command，调用 parser service。
- `services/parser.rs`：规范化 -> 缓存 -> metadata/playurl -> adapter -> stable result。
- `infrastructure/bilibili/`：HTTP client、URL resolver、WBI signer、raw response structs 和 adapter。
- `infrastructure/cache.rs`：canonical key 与 12 小时 TTL；WBI key 失败时失效并只重试一次。

## 数据流

```text
DownloadPage -> download store -> parse service -> IpcTransport
  -> parse_video command -> parser service
  -> URL/WBI/Bilibili adapters -> stable ParseVideoResult
  -> selection state -> DownloadTaskDraft -> task-drafts store -> /tasks
```

## 稳定契约

```ts
type DownloadMode = 'video-audio' | 'video-only' | 'audio-only'
type VideoCodec = 'avc' | 'hevc' | 'av1'
type AudioFormat = 'mp3' | 'm4a' | 'flac'

interface VideoPart {
  cid: number
  page: number
  title: string
  durationSeconds: number
}

interface MediaOption {
  id: string
  label: string
  requiresLogin: boolean
}

interface ParseVideoResult {
  canonicalUrl: string
  bvid: string
  aid: number
  title: string
  ownerName: string
  coverUrl: string
  durationSeconds: number
  requestedPage: number | null
  parts: VideoPart[]
  qualities: MediaOption[]
  codecs: VideoCodec[]
  audioFormats: AudioFormat[]
  audioBitrates: MediaOption[]
}
```

`parse_video` 请求为 `{ input: string }`，响应为 `ParseVideoResult`，错误沿用 AppError。外部 B 站 raw structs 只在 infrastructure 可见。

`DownloadTaskDraft` 包含 canonicalUrl、bvid、选中 cid/page、mode、quality/codec 或 audioFormat/bitrate、outputDir；不包含 task id、进度或持久化字段。

## 输入规范化与安全

- 纯 ID 使用严格 BV/AV pattern；URL 只接受 HTTPS 且 host allowlist 为 `bilibili.com` 子域与 `b23.tv`。
- 短链最多跟随受限次数重定向，最终 host 仍需 allowlist；不向任意 host 发请求，避免 SSRF。
- `p` 必须为正整数；不存在的指定 P 返回 E003。
- 日志不记录 Cookie、WBI key 或完整敏感响应；错误 details 不进入主 UI。

## 状态与缓存

- 页面状态用明确 discriminant：idle/parsing/success/failed/enqueueing；派生 disabled/选项不重复落 state。
- 前端相同 canonical key 的成功结果在会话中复用；清空 UI 不必清除 cache。
- Rust WBI key TTL 12h；签名相关失败先使 key 失效并重试一次，第二次转 AppError。
- 请求竞态使用递增 request token，过期响应不得覆盖最后一次输入。

## 登录与权限边界

- parser service 通过认证端口读取可选 Cookie；authentication 模块接入前为 None。
- 匿名响应只暴露后端确认可用的媒体能力；前端不靠标签猜测 480P 限制。
- 登录要求映射 E005，页面可导航 `/login`；本模块不保存 Cookie。

## Pattern 决策

- Adapter：隔离 B 站 raw response 与稳定 DTO，必要。
- Port：认证上下文和 task draft handoff 是真实跨模块边界，使用小接口/稳定 store；不引入 service locator。
- 不使用 Repository：当前缓存是有 TTL 的解析结果/WBI key，不是领域集合持久化。
- 不使用事件总线或 command class；Pinia action 与直接 IPC service 足够。

## 测试策略

- 前端：输入校验、竞态、默认/全选/反选、模式条件、invalid combination、入队草稿、跳转与 a11y 组件测试。
- Rust：五类输入规范化、allowlist/重定向保护、指定 P、WBI TTL/单次重试、raw fixture adapter、错误映射。
- HTTP 使用本地 fixture/mock server，不让单测依赖公网；另设 opt-in smoke test 验证当前 B 站匿名解析。
- 静态检查页面/组件不导入 Tauri，raw DTO 不越过 infrastructure。

## 风险

- B 站接口/WBI 可能变化：raw fixture + adapter 集中吸收；稳定 command DTO 不随外部字段漂移。
- b23 重定向与封面 URL 是外部网络：超时、大小限制与 allowlist 必须在 Rust 层。
- 任务模块尚未完成：会话草稿边界只提供 handoff，不伪造下载执行或任务状态。
