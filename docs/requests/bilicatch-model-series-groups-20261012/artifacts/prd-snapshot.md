# 直接变更快照

## Verbatim Source Evidence Index

- `SRC-01`：“在模型管理弹窗中把模型按照系列分组，把同一个系列的模型分到同一个分组中”。

## Source Summary

用户要求改变模型管理弹窗的展示结构，分组维度是“模型系列”。

## Key Business Goal

降低模型列表的认知成本，使用户能先识别引擎/系列，再比较同系列模型。

## Greenfield Scaffold Sensitivity

不适用：本请求不创建新项目、应用、包或前端业务面。

## Explicit Behavior Constraints

- `source-backed`：弹窗中模型必须按系列分组。
- `source-backed`：同系列模型必须在同一分组。
- `code-fact-backed`：Whisper 系列 ID 为 `tiny/base/small/medium/large-v3`；FireRedASR2 系列 ID 为 `fire-red-asr2`。
- `code-fact-backed`：保留现有模型顺序、操作事件、下载进度和状态文案。

## Closed-World Product Scope Allowlist

- 新增模型系列分组容器。`SRC-01`
- 新增每个非空分组的系列标题。`SRC-01`
- 不授权新增筛选、折叠、排序、分组统计或新操作。

## Displays And Interactions

- 容器：现有“本地识别模型”弹窗。
- 展示：Whisper 与 FireRedASR2 系列标题，下方为现有模型行。
- 交互：现有下载、取消、删除与关闭行为不变。

## Workflow And State Rules

分组是 `models` props 的同步派生视图；props 更新时分组自动更新，不引入新的可写状态。

## Relevant Modules

- `src/features/transcription/components/ModelManager.vue`
- `src/features/transcription/components/ModelManager.test.ts`
- `src/features/transcription/transcription.css`
- `src/locales/zh-CN.ts`
- `src/locales/en-US.ts`

## Open Questions

无。现有模型 ID 足以唯一确定系列。

