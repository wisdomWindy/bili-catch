# 执行记录：模型系列分组

## Delivery Unit

`bilicatch-model-series-groups-20261012`

## 实现结果

- 在 `ModelManager.vue` 内新增穷尽的 `AsrModelId -> ModelSeries` 映射，并用只读 `computed` 生成非空分组。
- 固定按 Whisper、FireRedASR2 的顺序展示系列；每个系列内部保留 `models` props 的原始相对顺序。
- 每个系列使用带 `aria-labelledby` 的语义 `section`，模型行及原有 `download/cancel/delete/close` 事件保持不变。
- 在中英文资源中新增系列标题，在现有 `transcription.css` 中补充分组间距和标题样式。
- 没有修改后端、IPC、模型下载流程、store 或模型合同。

## TDD 证据

- RED：`npm test -- src/features/transcription/components/ModelManager.test.ts`，新增的两个分组用例按预期失败，原因分别是不存在系列 section 与不存在系列标题。
- GREEN：同一命令随后通过，结果为 1 个测试文件、4 个测试全部通过。
- 新增覆盖：交错输入下的系列归属、固定系列顺序、组内相对顺序，以及空系列不渲染。

## 改动文件

- `src/features/transcription/components/ModelManager.vue`
- `src/features/transcription/components/ModelManager.test.ts`
- `src/features/transcription/transcription.css`
- `src/locales/zh-CN.ts`
- `src/locales/en-US.ts`

## 产品内容来源

- 系列分组与系列标题：`source-backed`，来自 `SRC-01` 用户请求。
- ID 归属、穷尽映射、只读派生、ARIA 关联与样式实现：`product-content delta: none`，仅用于实现并约束已批准行为。
- 没有新增筛选、折叠、排序、统计或下载语义。

## 已知环境限制

- 浏览器开发页可以打开模型管理弹窗，但没有 Tauri 桌面后端时模型清单为空。因此运行页只验证了弹窗可打开且空系列不渲染；含模型数据的分组层级由组件测试验证。

## 风险与回滚

- 新模型 ID 必须在穷尽映射中明确归属，TypeScript 会在遗漏时阻止构建。
- 回滚只需移除本次五个目标文件中的分组相关增量；后端和数据合同无需回滚。
