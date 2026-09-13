# 请求：仅音频下载错误显示视频不可用

## Request Identifier

`bilicatch-audio-cover-url-fix-20260913`

## Source Link

- 来源：用户直接反馈
- 日期：2026-09-13
- 复现视频：`https://www.bilibili.com/video/BV1FNb366EH2/`

## Business Summary

用户在下载中心解析视频并选择“仅音频”后，任务失败且界面显示“视频不可用”。仅音频模式应使用可用音轨生成所选音频文件，不应因为 Bilibili 返回 HTTP 封面地址而失败。

## Goal Statement

让 Bilibili 受信任图片域名返回的 HTTP 封面地址在 adapter 边界被安全规范化为 HTTPS，使仅音频任务可以继续下载封面、音轨并进入处理阶段，同时不放宽媒体 URL 的 HTTPS 与域名白名单约束。

## Initial Done Signal

- `BV1FNb366EH2` 当前返回 HTTP 封面地址时，仅音频 source resolve 不再返回 `E004`。
- 规范化后的封面 URL 为 HTTPS，主机仍属于既有允许列表。
- 非 Bilibili 主机、带凭据、fragment 或其他不安全 URL 仍被拒绝。
- 仅视频与视频+音频路径不回归。

## Trigger Condition

用户在下载中心解析 Bilibili 视频、选择“仅音频”并加入下载队列。

## Initial Context Sources

- 用户直接缺陷描述。
- 目标视频的 Bilibili view/playurl 当前响应。
- `ParserService`、Bilibili audio adapter、媒体 URL 校验与 `AudioExecutor`。
- 既有回归修复与音频下载规格。

## Human Handoff Point

规格与计划需要用户批准；实现完成后交付测试证据和可运行验证结果。未经额外请求不推送代码或发布 Release。

## Affected Area

- 下载中心仅音频任务。
- Rust Bilibili adapter 的封面 URL 规范化。
- audio source resolve 与封面下载前置校验。

## Participating Modules

- `src-tauri/src/infrastructure/bilibili/audio_source.rs`
- `src-tauri/src/infrastructure/bilibili/media_url.rs`
- `src-tauri/src/services/parser.rs`
- `src-tauri/src/services/audio/executor.rs`
- 对应 Rust 单元与集成测试。
