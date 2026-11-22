# 验证：合集/番剧元数据解析

## 命令证据

| 命令 | 结果 |
| --- | --- |
| `cargo fmt -- --check` | 通过 |
| `cargo clippy --all-targets -j 1` | 通过（无本模块新增 warning） |
| `cargo test --lib --tests -j 1` | 通过（lib 107 passed；集成套件全绿） |
| `pnpm run typecheck` | 通过 |
| `pnpm run test` | 通过（57 files / 237 tests） |
| `pnpm run build` | 通过 |

## 验收标准映射

| AC | 结论 | 证据 |
| --- | --- | --- |
| AC-1 合集返回全部成员单元 | 通过 | `parser.rs::expands_a_collection_into_units_from_ugc_season`：3 单元、`container.kind=season`、`source=ugc`、标题取合集标题 |
| AC-2 成员多 P 展开 | 通过 | 同一用例：成员 1 的 `pages` 展开为 2 单元（`partCount=2`），成员 2 无 `pages` 合成 1 单元（`groupIndex=2`） |
| AC-3 番剧正片过滤 | 通过 | `parses_bangumi_episode_through_the_season_port`：预告 `section_type=1` 被过滤，`parts.len()==2` |
| AC-4 时长归一 | 通过 | `duration_ms_to_seconds` 单测（0/499/500/1203160→1203）；番剧用例断言 1203 |
| AC-5 可用性初判 | 通过 | 番剧用例第二条 `badge="会员"` → `Unavailable`；`allow_download` 分支由单元逻辑覆盖 |
| AC-6 `requestedCid` 与默认选中 | 通过 | 番剧用例断言 `requested_cid=Some(41191148952)`；合集用例断言入口 cid；前端 `prefers the requested unit cid over the requested page` 断言选中 1002 |
| AC-6 修复（2026-11-22） | 通过 | 合集入口无 `?p=` 时 `requested_cid` 改为入口当前 P 的 cid；新增 `selects_the_pasted_collection_entry_instead_of_the_first_episode`；实机解析 `BV1ZAWM61Evb` 后 EP8 被选中并滚动可见：[截图](docs/requests/bilicatch-collection-bangumi-support-20261114/module-runs/episode-metadata/verification/evidence/collection-entry-selection.png) |
| AC-7 空集/无正片返回 E004 | 通过 | `adapt_bangumi_result` / `adapt_collection_result` 空单元分支返回 `E004`；端口失败用例保留 |
| AC-8 删除占位分支、UGC 不回归 | 通过 | 占位分支已删除；既有 UGC 用例与前端 236+ 用例全绿 |
| AC-9 三类检查通过 | 通过 | 见命令证据 |

## 行为验证要点

- 合集封面/标题优先取 `ugc_season`，缺失回退入口稿件。
- 番剧 `long_title` 优先于 `title`。
- 重复 `cid` 只保留首次（两个 adapter 都去重）。
- `ep` 输入顶层 `bvid/aid` 取该集；`ss` 输入为 `null`。

## 未覆盖与原因

- PGC 能力字段（清晰度/音频）：属模块 3，本模块返回空能力。
- 真实 `badge` 文案（会员/付费）未实测：按 `external-interface-pending` 处理，模块 3 取流复核。
- `ugc_season` 超过内嵌上限的分页：当前不实现，已记开放问题。

## 残留风险

- 付费集若 `badge` 文案不含“会员/付费”，会被乐观标为 playable；下载阶段由模块 3 拦截并给出权限错误。
- 合集能力字段取入口稿件，成员清晰度差异不展示。
