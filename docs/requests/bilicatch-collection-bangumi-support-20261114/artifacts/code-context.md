# 代码上下文：输入与单元契约

## 结构分析方式

- Code graph：本仓库未提供可用的 code graph 工具/索引（无 MCP、无索引产物）。
- Fallback：scoped 文本搜索 + 逐文件阅读，范围限定在请求链路与任务链路。
- 影响判定：以“谁构造 `VideoPart` / `ParseVideoResult` / `DownloadTaskDraft` / `DownloadTask`”为主线。

## 关键调用点

| 符号 | 定义 | 构造点 | 消费点 |
| --- | --- | --- | --- |
| `VideoId` | `infrastructure/bilibili/input.rs:5` | `normalize_input` | `client.rs` 请求参数、`parser.rs` 分支、`media_url` 校验 |
| `NormalizedInput` | `input.rs:13` | `normalize_input` | `parser.rs:110` |
| `ParseVideoResult` | `models/parse.rs:53` | `adapt_parse_result`（`adapter.rs:114`） | IPC `parse_video`、前端 `ParseVideoResult` |
| `VideoPart` | `models/parse.rs:28` | `adapter.rs:126` | store 选中、`PartSelector`、`buildDrafts` |
| `DownloadTaskDraft` | `models/task.rs:65` | 前端 `store.ts:322`、测试夹具 | `validate_create_request`、`manager.create` |
| `DownloadTask` | `models/task.rs:87` | `manager.rs:123` | 任务事件、前端任务页、持久化 |

## 既有约束

- `services/tasks/validation.rs:39` 要求 `part_count > 0` 且 `1 ≤ page ≤ part_count`；本模块改语义为“容器内”，约束表达不变。
- `manager.rs:123` 任务 id 为 `{request_id}:{index}`，与 bvid/cid 无关，新增字段不影响唯一性。
- `filename.rs:11/15` 依赖 `part_title`、`bvid`、`page`、`part_count`；本模块不改命名规则（模块 3）。
- `client.rs:143 append_video_id` 只处理 Bvid/Aid/ShortUrl；新增变体必须先由 `parser.rs` 分支拦截，否则需要改端口签名（模块 2）。

## 测试脆弱面

- 前端对象字面量构造 `ParseVideoResult`/`VideoPart`/`DownloadTaskDraft` 的测试与 demo：新增必填字段会触发类型错误，需同步。
- Rust `adapter.rs` fixture 测试同时断言 `parts` 数量与字段，新增字段后需补断言而非只改编译。
- `parser.rs` 测试使用 `view_fixture`/`play_fixture`，新增分支需补 ep/ss 用例。

## 不在本次影响范围

`raw.rs` 字段、`client.rs` 请求、`media_url.rs`、`audio_source.rs`、`video_source.rs`、页面组件与样式。
