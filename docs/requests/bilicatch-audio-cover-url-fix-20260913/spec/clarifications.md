# Clarifications

## Question 1

是否通过允许全局 HTTP 媒体 URL 来兼容 Bilibili 封面？

## Answer

否。音视频 CDN 与规范化后的封面都必须继续使用 HTTPS。

## Final Decision

只在 Bilibili audio metadata adapter 边界将 raw HTTP 封面升级为 HTTPS，再复用既有 validator。

## Affected Spec Area

In Scope、Design Constraints、AUDCOV-AC-01 至 AUDCOV-AC-03。

## Question 2

是否同时修改通用 `E004` 的中文文案？

## Answer

否。当前问题是错误触发，不是错误码合同设计；改文案会扩大前后端合同与所有错误路径范围。

## Final Decision

保持 `E004` 和现有本地化文案，消除受信任 HTTP 封面造成的错误 E004。

## Affected Spec Area

Out Of Scope、UI Outcome、Risks。
