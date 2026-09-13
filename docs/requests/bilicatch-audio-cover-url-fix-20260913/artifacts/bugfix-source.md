# Bugfix Source

## Source System

用户直接反馈，无外部缺陷系统。

## Project Key Or Source Scope

仓库 `wisdomWindy/bili-catch`，下载中心仅音频模式。

## Defect Or Work Item ID

`bilicatch-audio-cover-url-fix-20260913`

## Defect Title

下载中心选择仅音频后显示“视频不可用”。

## Observed Behavior

解析视频并选择仅音频后，任务返回 `E004`，前端按通用错误码显示“视频不可用”，音频下载未开始。

## Expected Behavior

当目标视频提供普通音轨且封面来自受信任 Bilibili 图片域名时，仅音频任务应完成 source resolve，下载封面与音轨，并按用户选择生成 M4A、MP3 或允许的 FLAC 输出。

## Reproduction Clues

1. 使用 `BV1FNb366EH2`。
2. Bilibili view 接口当前返回 `cid=41680179120`，封面主机为 `i2.hdslb.com`，scheme 为 `http`。
3. playurl 当前返回 3 条普通 `audio/mp4` 音轨，bandwidth 为 65,971 或 85,411 bps。
4. 用应用相同 User-Agent、Referer 和 Range 请求最高音轨，CDN 返回 `206`，证明音轨可用。
5. `ParserService::resolve(AudioSourceRequest)` 在返回 bundle 前调用 `validate_media_url(&metadata.cover_url)`；现有校验只允许 HTTPS，因此 HTTP 封面在音轨下载前被映射成 `E004`。
6. 将相同封面 URL 的 scheme 升级为 HTTPS 后，请求返回 `200 image/jpeg`。

## Affected Module Or Page

- 下载中心选择“仅音频”后创建的任务。
- Bilibili raw view 到 `AudioMetadata` 的 adapter。
- audio source resolve 的封面 URL 校验。

## Available Screenshots, Logs, Or Comments

- 用户可见文案：“视频不可用”。
- 官方接口诊断：`VIEW_CODE=0`、`PLAY_CODE=0`、`DASH_AUDIO_COUNT=3`。
- CDN 音轨诊断：`STATUS=206`、`CONTENT_TYPE=application/octet-stream`。
- 封面诊断：原始 HTTP 与升级后的 HTTPS 均返回 `200 image/jpeg`；本地安全校验拒绝原始 HTTP。

## Open Questions And Missing Context

- 无阻断问题。
- 当前证据覆盖匿名官方接口；登录态沿用相同 view 封面字段与 adapter 路径。
