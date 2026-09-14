# Execution Changelog

## 实现

- 新增 `ClipboardReader` 类型与 Tauri clipboard-manager adapter。
- 按既有 injection/provide 方式将 production reader 注入下载页，demo 使用空实现。
- 下载页在 mount/focus 读取，复用 `normalizeParseInput`，合法值走既有 `submit`。
- 页面用 raw text 去重、revision 抑制旧异步结果，卸载时移除 listener 并使读取失效。
- Tauri 注册官方插件，仅授予 `clipboard-manager:allow-read-text`。

## TDD 证据

- RED：三个剪贴板用例均失败，输入框保持空值；失败原因是页面无读取/聚焦处理。
- GREEN：下载页 12 个用例通过；组合运行最初暴露旧 wrapper 未卸载，补齐测试清理后通过，竞态用例单独与组合均稳定。

## Provenance Ledger

| Hunk | Product content delta | 来源 |
| --- | --- | --- |
| mount/focus 自动识别并解析 | 剪贴板行为 | CLIP-01/02，用户反馈，PRD |
| 合法性、去重、竞态、静默降级 | none（质量约束） | CLIP-03/04/05，spec |
| adapter/injection/plugin/capability | none（技术集成） | architecture ADR-CLIP-01 |
| 测试清理 | none（测试隔离） | 全局 listener 生命周期 |

## Pre-review Self-check

- 改动未触碰设置页修复文件，也未改变 paste/drop/手动解析。
- 无新增文案、样式、组件、全局 store、轮询、写剪贴板权限或日志内容。
- 所有新增文件与公开类型均在批准复杂度预算内；无 dead code 或调试输出。
- 下一步证据：全量 Vitest、typecheck、build、Cargo check/fmt、Tauri 实机流程。
