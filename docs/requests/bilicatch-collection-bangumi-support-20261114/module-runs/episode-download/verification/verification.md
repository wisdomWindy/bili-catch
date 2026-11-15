# 验证：取流与权限

## 命令证据

| 命令 | 结果 |
| --- | --- |
| `cargo fmt -- --check` | 通过 |
| `cargo clippy --all-targets -j 1` | 通过（仅既有 warning：`filename.rs:27`、`track.rs` 参数数量） |
| `cargo test --lib --tests -j 1` | 通过（lib 111 passed；集成套件全绿） |
| `pnpm run typecheck` | 通过 |
| `pnpm run test` | 通过（57 files / 237 tests） |
| `pnpm run build` | 通过 |

## 验收标准映射

| AC | 结论 | 证据 |
| --- | --- | --- |
| AC-1 番剧解析能力非空 | 通过 | `parser.rs::parses_bangumi_episode_through_the_season_port` 断言 `qualities` 非空；能力来自代表集 PGC playurl |
| AC-2 番剧走 PGC、UGC 不走 | 通过 | `resolves_bangumi_sources_through_the_pgc_port` 通过 `StaticPort` 的 PGC 端口返回流；UGC 用例仍断言 `view_calls`/`play_calls` |
| AC-3 权限拒绝 | 通过 | 会员集用例 → `E006`；缺 `epId` 用例 → `E003`；cid 不匹配由 `fetch_bangumi_source` 返回 `E004`（代码路径覆盖） |
| AC-4 命名 | 通过 | `sanitize_season_filename` 单测（`2_第2话.mp4`、空标题回退 `BV…-P1.m4a`）；`season_drafts_are_named_with_their_index` 断言 manager 产出 `3_第3话.mp4` |
| AC-5 旧任务兼容 | 通过 | `container_kind` 带默认值；`models/task.rs` 旧 JSON 用例仍通过 |
| AC-6 UGC 不回归 | 通过 | 全量 Rust/前端测试通过；UGC 取流与命名测试未改动断言 |
| AC-7 四类命令 | 通过 | 见命令证据 |

## 行为验证要点

- 番剧权限在取流前判定：`bangumi_episode_availability` 同时被解析与下载使用，避免两套规则。
- 代表集取流失败降级为空能力，不使整季解析失败（`bangumi_capability_play().ok()`）。
- `container_kind=season` 命名对音频/视频同规则；`video` 完全沿用旧规则。
- `source`/`ep_id` 从任务透传到端口，执行器签名不变。

## 未覆盖与原因

- PGC 真实错误码全集：仅映射已确认 `-403`/`-10403`，其余走默认 `E004`；标注 `外部接口待补`。
- 地区限制/DRM 实测：无可用测试账号，按 `E006` 兜底。
- 端到端真实下载：需要真实网络与账号，未在自动化中执行。

## 残留风险

- 若 `badge` 未标注会员而接口实际需要权限，取流会以 `E006`/`E004` 失败，不会产生半成品文件。
- 代表集能力可能与其它集存在清晰度差异（例如某集受限）；播放列表仍可显示，下载会按各集实际能力校验。
