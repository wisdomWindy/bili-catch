# 执行记录：输入与单元契约

## 任务状态

| ID | 任务 | 状态 | 说明 |
| --- | --- | --- | --- |
| T1 | 变更链路审查与基线 | 完成 | 复核 `artifacts/code-context.md`；基线：`cargo test` 全绿（后因契约变更短暂失败，已修复）、`pnpm test` 57 文件 236 用例 |
| T2 | Rust 输入识别扩展 | 完成 | `VideoId` 新增 `Episode`/`Season`；URL 段解析；`?p=` 仅稿件生效 |
| T3 | 解析 DTO 与 UGC 适配填充 | 完成 | 新增 `VideoSource`/`ContainerKind`/`Availability`/`UnavailableReason`/`ParseContainer`；`VideoPart` 扩展；adapter 填充 |
| T4 | 任务契约与校验 | 完成 | 草稿/任务新增 `source`/`epId`/`groupIndex`/`partCount`（serde 默认）；manager 透传；一致性校验 |
| T5 | 剧集目标占位分支 | 完成 | `ParserService::parse` 对 `Episode`/`Season` 返回 `E004`，不发起元数据请求 |
| T6 | 前端契约与输入校验 | 完成 | TS 契约镜像；`isSupportedUrl` 接受 `ep`/`ss`，裸 `ep` 仍拒绝 |
| T7 | 草稿构造与测试夹具适配 | 完成 | `buildDrafts` 使用单元 `bvid`/`epId`/`groupIndex`/`partCount`；夹具与断言同步 |
| T8 | 回归、清理与证据 | 完成 | 见 `verification/verification.md` |

## 改动文件

Rust：

- `src-tauri/src/infrastructure/bilibili/input.rs`
- `src-tauri/src/infrastructure/bilibili/adapter.rs`
- `src-tauri/src/infrastructure/bilibili/client.rs`（新增变体的穷尽分支）
- `src-tauri/src/models/parse.rs`、`src-tauri/src/models/task.rs`、`src-tauri/src/models/mod.rs`
- `src-tauri/src/services/parser.rs`、`services/tasks/manager.rs`、`services/tasks/validation.rs`
- 测试：`tests/parse_contract.rs`、`task_contract.rs`、`task_rules.rs`、`task_store.rs`、`task_manager.rs`、`download_runtime.rs`、`audio_executor.rs`、`video_executor.rs`

前端：

- `src/features/download-center/contracts.ts`、`input.ts`、`store.ts`、`demo-service.ts`
- 测试：`input.test.ts`、`store.test.ts`、`DownloadPage.test.ts`、`task-drafts.test.ts`、`task-management/store.test.ts`

## 执行中修正

- 首轮 `cargo test` 集成测试因 5 处 `DownloadTask`/`DownloadTaskDraft` 字面量缺字段失败：补齐 `part_count`/`source`/`ep_id`/`group_index`。
- `validation` 测试初版断言错误码写死 `E001`，实际内部错误码不同：改为与 `AppError::internal` 比较。
- clippy 报 `filter(..).next_back()` 与“测试模块后仍有条目”：改为 `rfind` 并把 `valid_media_fields` 移到测试模块之前。
- `cargo fmt` 重排了追加的测试代码。

## 清理清单（跨模块）

- 模块 2 合入时删除 `src-tauri/src/services/parser.rs` 中 `Episode/Season → E004` 的临时分支。
- 模块 3 合入时复核 `filename.rs` 对 `part_count` 的新语义（容器内计数）。
- 模块 4 合入时处理 `VideoSummary` 对可空 `bvid` 的展示（当前为 null 时渲染空串）。

## 未改动

无新增依赖、无目录结构变化、无页面组件改动、无样式改动。
