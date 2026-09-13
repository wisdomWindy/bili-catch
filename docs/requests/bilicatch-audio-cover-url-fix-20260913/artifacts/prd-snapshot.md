# 下游规格输入快照

## Source Summary

下载中心仅音频任务因 Bilibili view 返回 HTTP 封面 URL 而在 audio source resolve 阶段失败。音轨本身存在且可下载，错误发生在封面 URL 的 HTTPS 安全校验。

## Key Business Goals

- 仅音频模式在目标视频提供普通音轨时可以继续执行。
- 保留媒体与封面 URL 的 HTTPS、无凭据、无 fragment 和 Bilibili 域名白名单安全边界。
- 错误修复位于 Bilibili adapter 边界，不让下游 executor 感知 raw HTTP 兼容细节。

## Explicit Behavior Constraints

- 仅允许把 `http://` 且主机属于现有 Bilibili 图片/媒体允许域的封面地址规范化为 `https://`。
- 已是 HTTPS 的受信任封面保持等价。
- 不允许任意外部主机、用户名/密码、fragment、非 HTTP(S) scheme 或无主机地址通过。
- 不放宽音视频媒体源校验，也不将 E004 改成成功。
- 不改变下载模式、音频格式、码率或任务 IPC 字段。

## Forms, Tables, Displays, And Interactions Extracted From Source

- 页面：下载中心。
- 交互：解析视频 -> 选择“仅音频” -> 选择格式/码率 -> 加入队列。
- 成功结果：任务进入下载/处理，不显示“视频不可用”。
- 失败结果：真实无音轨或不安全 URL 仍显示稳定本地化错误。

## Workflow And State Rules Extracted From Source

- Parse 阶段返回能力与部件信息。
- Audio executor fresh resolve 当前账户下的 view/playurl。
- Adapter 输出规范化且可校验的 `AudioMetadata.cover_url`。
- Source resolve 通过后才准备 workspace 并下载封面与音轨。

## Relevant Modules Or Pages

- `src-tauri/src/infrastructure/bilibili/audio_source.rs`
- `src-tauri/src/infrastructure/bilibili/media_url.rs`
- `src-tauri/src/services/parser.rs`
- `src-tauri/src/services/audio/executor.rs`

## Notable Open Questions From The Upstream PRD

- 无。修复应采用 adapter 内受限的 HTTP -> HTTPS 规范化，而不是降低全局 URL 校验等级。
