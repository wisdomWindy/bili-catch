# 验证：输入与单元契约

## 命令证据

| 命令 | 结果 |
| --- | --- |
| `cargo fmt -- --check`（`src-tauri`） | 通过（exit 0） |
| `cargo clippy --all-targets -j 1` | 通过（exit 0；剩余 9 条均为既有 warning，不在本模块改动行） |
| `cargo test --lib --tests -j 1` | 通过（exit 0；lib 105 passed，集成测试套件全部 ok） |
| `pnpm run typecheck` | 通过（exit 0） |
| `pnpm run test` | 通过（57 files / 236 tests） |
| `pnpm run build` | 通过（dist 产出） |

## 验收标准映射

| AC | 结论 | 证据 |
| --- | --- | --- |
| AC-1 输入矩阵同构 | 通过 | Rust `input.rs` 单测（BV/av/视频页/移动页/b23/合集 BV/ep/ss/带 p/非法域/http/空串/裸 ep/`ep0`/`ssabc`）；前端 `input.test.ts`（23 用例，同批矩阵） |
| AC-2 剧集目标返回 E004 | 通过 | `parser.rs::reserves_bangumi_targets_before_any_metadata_request`：断言 `E004` 且 `view_calls == 0` |
| AC-3 `1 ≤ page ≤ partCount` 与 cid 唯一 | 通过 | `adapter.rs` 断言 `part_count = 2`、`page` 1/2；`validation.rs` 断言越界拒绝 |
| AC-4 availability/原因可空关系 | 契约层通过，运行时延后 | 类型与序列化已固定（`adapter` 断言 `playable` + `null`）；`unavailable` 产出属模块 2，禁用逻辑属模块 4（spec 已注明） |
| AC-5 旧任务 JSON 兼容 | 通过 | `models/task.rs` 两条反序列化测试：缺字段回退 `ugc`/`null`/`1`/`1` |
| AC-6 草稿一致性 | 通过 | `validation.rs` 测试：`bangumi` 缺 `epId`、`epId=0`、`ugc` 带 `epId` 均拒绝 |
| AC-7 既有 UGC 不回归 | 通过 | 全量 Rust 与前端测试通过；`adapter` 既有断言保留并扩展 |
| AC-8 三类检查通过 | 通过 | 见命令证据（另加 fmt/clippy） |
| AC-9 UGC 适配填充 | 通过 | `adapter.rs` 断言 `bvid`、`part_count`、`group_index=1`、`availability=playable`、`unavailable_reason=None` |

## 行为验证要点

- 合集内不同 `bvid` 的单元不再共享顶层 bvid：`store.test.ts::keeps each unit bvid instead of the container bvid` 断言草稿分别取各自 bvid 与 `epId`。
- `ss` 季级结果允许 `bvid`/`aid` 为 `null`：TS 契约为 `string | null` / `number | null`，`typecheck` 通过。
- `ep`/`ss` 链接带 `?p=` 被忽略：Rust 与前端各有用例。

## 未覆盖与原因

- 真实 B 站 PGC 响应端到端：本模块不取数，属于模块 2；接口事实已记录在 `artifacts/prd-snapshot.md` X1–X7。
- `unavailable` 运行时产出与 UI 禁用：模块 2/4。
- 手工 UI 验证：本模块无布局改动；模块 4 统一验证。

## 残留风险

- `client.rs` 对 `Episode/Season` 的占位错误分支在模块 2 尚未移除前，若被其它入口直接调用会返回 `E004`（符合预期，已登记清理）。
- `VideoSummary` 目前对 `null` bvid 渲染为空，属模块 4 的展示决策，未越界实现。
