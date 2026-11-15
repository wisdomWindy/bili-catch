# 执行记录：统一选集界面

## 过程说明（偏差）

本模块实现前以会话内任务分解推进，`plan/plan.md` 在实现后补写以保证仓库可追溯；已在 `review/review.md` 记录为流程偏差（非阻塞）。其余阶段顺序正常：`page-design → spec → execute → verify → review`。

## 任务状态

| ID | 任务 | 状态 | 说明 |
| --- | --- | --- | --- |
| T1 | 视图模型与标签 | 完成 | 按 `groupIndex` 分组；season 多 P 出集头；`EP`/`P` 标签 |
| T2 | 集头与半选 | 完成 | checked/indeterminate/disabled；点击 emit `toggleGroup` |
| T3 | 选择动作 | 完成 | `togglePart`/`toggleGroup`/`selectAll`/`invertSelection` 均只作用于 `playable` |
| T4 | 摘要、文案与样式 | 完成 | 隐藏空 BV/0 时长；新增 `unavailableReasons` 双语键；新增集头/子行/禁用样式；`format.ts` 统一时长格式 |
| T5 | 测试与回归 | 完成 | 新增 `PartSelector.test.ts`（4 用例）+ store 混合可用性用例 |

## 改动文件

- `src/features/download-center/components/PartSelector.vue`（重写）
- `src/features/download-center/components/VideoSummary.vue`
- `src/features/download-center/store.ts`
- `src/features/download-center/format.ts`（新增；2 个真实调用方）
- `src/features/download-center/download-center.css`
- `src/pages/DownloadPage.vue`
- `src/locales/{zh-CN,en-US}.ts`
- 测试：`components/PartSelector.test.ts`（新增）、`store.test.ts`

## 执行中修正

- 测试 helper 显式 `cid` 与展开字段重复，TS2783；移除显式字段后通过。
- 原因文案键与既有 `download.unavailable`（字符串）冲突，改为 `download.unavailableReasons` 对象。

## 未改动

无 Rust 代码改动；无新增依赖。
