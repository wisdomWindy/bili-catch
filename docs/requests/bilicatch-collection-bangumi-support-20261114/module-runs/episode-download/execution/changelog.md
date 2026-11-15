# 执行记录：取流与权限

## 任务状态

| ID | 任务 | 状态 | 说明 |
| --- | --- | --- | --- |
| T1 | 能力映射抽取 | 完成 | 新增 `PlayCapabilities` + `capabilities_from_play`；UGC/番剧共用 |
| T2 | PGC 取流端口 | 完成 | `fetch_bangumi_playurl(ep_id, cid)`；`PgcResponse<PlayData>`；`ensure_playable` |
| T3 | 请求与执行器字段 | 完成 | 两个请求结构加 `source`/`ep_id`；4 处构造点填充 |
| T4 | 解析取流与权限 | 完成 | `fetch_bangumi_source` 校验归属与可用性；`permission_error` 映射 E005/E006；番剧解析补能力 |
| T5 | 命名与任务字段 | 完成 | `container_kind`（默认 video）、`sanitize_season_filename`、manager 分流 |
| T6 | 回归与证据 | 完成 | 见 verification |

## 改动文件

Rust：`adapter.rs`、`client.rs`、`mod.rs`、`parser.rs`、`models/task.rs`、`services/video/{source,executor,track}.rs`、`services/audio/{source,executor}.rs`、`services/tasks/{filename,manager,mod}.rs`
前端：`contracts.ts`、`store.ts`
测试：`parser.rs` 内联、`task_rules.rs`、`task_manager.rs`、`task_contract.rs`、`task_store.rs`、`audio_executor.rs`、`video_executor.rs`、`download_runtime.rs`、`task-drafts.test.ts`、`task-management/store.test.ts`

## 执行中修正

- `adapt_parse_result` 的能力计算抽出后，用一次性整体替换而非局部编辑，避免两份重复块；既有断言未改动。
- `fetch_wbi_key` 编辑时误删 `let response = self`，编译期立即发现并修复。
- 番剧能力用例暴露 `AudioSourceBundle` 无 `Debug`，把 `expect_err` 改为 `match`。
- 新增 `containerKind` 触发 6 处测试字面量缺字段，逐处补齐。

## 清理清单

- 模块 4 合入时处理 `ownerName` 为空、`durationSeconds=0`、可空 `bvid` 的展示。
- PGC 错误码全集仍为 `外部接口待补`，后续实测补充。

## 未改动

无新增依赖、无新增文件、无 UI 组件/样式改动；UGC 取流路径未改变行为。
