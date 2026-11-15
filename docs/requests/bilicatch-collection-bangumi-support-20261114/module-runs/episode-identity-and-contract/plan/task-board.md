# 任务板：输入与单元契约

| ID | 任务 | 状态 | 依赖 | 主要文件 | 验证 |
| --- | --- | --- | --- | --- | --- |
| T1 | 变更链路审查与基线 | 完成 | — | `artifacts/code-context.md` | 基线输出 |
| T2 | Rust 输入识别扩展 | 完成 | T1 | `input.rs` | `cargo test input` |
| T3 | 解析 DTO 与 UGC 适配填充 | 完成 | T2 | `models/parse.rs`、`adapter.rs` | `cargo test adapter` |
| T4 | 任务契约与校验 | 完成 | T3 | `models/task.rs`、`manager.rs`、`validation.rs` | `cargo test tasks` |
| T5 | 剧集目标占位分支 | 完成 | T2 | `parser.rs` | `cargo test parser` |
| T6 | 前端契约与输入校验 | 完成 | T2 | `contracts.ts`、`input.ts` | `pnpm test`、`pnpm build` |
| T7 | 草稿构造与测试夹具适配 | 完成 | T6 | `store.ts`、前端测试/夹具 | `pnpm test` |
| T8 | 回归、清理与证据 | 完成 | T2–T7 | `changelog.md`、`verification.md` | 全量命令 |

## 关键检查点

- T2：裸 `ep733316` 必须被拒；`https://…/bangumi/play/ep733316` 必须被接受。
- T3：`partCount` 必须先于 `pages` move 计算。
- T4：旧 JSON 无新字段必须能反序列化。
- T5：`E004` 文案必须与 `E003` 可区分。
- T7：至少一条断言证明多单元草稿不共享顶层 `bvid`。

## 清理清单

- 模块 2 合入时删除 `parser.rs` 的 `Episode/Season → E004` 临时分支。
- 模块 3 合入时复核命名规则对新字段的使用。
