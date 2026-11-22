# 执行记录：合集/番剧元数据解析

## 任务状态

| ID | 任务 | 状态 | 说明 |
| --- | --- | --- | --- |
| T1 | raw 模型与 PGC envelope | 完成 | 新增 `PgcResponse`、`SeasonData`、`RawEpisode`、`RawUgcSeason` 等；`ViewData.ugc_season` |
| T2 | PGC 端口方法 | 完成 | `BilibiliPort::fetch_bangumi_season(ep_id, season_id)`；独立 `decode_pgc` |
| T3 | 两个适配函数 | 完成 | `adapt_collection_units`、`adapt_bangumi_units`、`duration_ms_to_seconds`；cid 去重 |
| T4 | 解析分派与 requestedCid | 完成 | 三路分派；删除模块 1 的 `E004` 占位；新增 `requestedCid` |
| T5 | 前端契约与默认选中 | 完成 | `requestedCid` 加入契约；选中优先级 cid → page → 首个 playable → 首个 |
| T6 | 测试夹具与回归 | 完成 | Parser 两条集成用例（番剧/合集）+ 时长单测 + 前端默认选中用例 |

## 改动文件

- Rust：`raw.rs`、`client.rs`、`adapter.rs`、`parser.rs`、`mod.rs`、`models/parse.rs`
- 前端：`contracts.ts`、`store.ts`、`demo-service.ts`
- 测试：`parser.rs` 内联用例、`adapter.rs` 单测、`parse_contract.rs`、`store.test.ts`、`DownloadPage.test.ts`

## 执行中修正

- `adapt_collection_result` 需要入口页码对应的 cid，改为在消费 `view` 前先取。
- `parse` 的分派改为借用匹配，避免移动 `normalized.video_id` 后无法再用于请求。
- 模块 1 占位分支删除后，原占位测试改为断言“season 端口失败时错误码原样传播”。

## 清理清单（跨模块）

- 模块 3 合入时补 PGC 能力字段（清晰度/音频/编码）与取流权限复核。
- 模块 4 合入时处理 `ownerName` 为空、`durationSeconds=0` 的展示。

## 未改动

无新增依赖、无新增文件、无页面组件与样式改动。

## 后续修复（2026-11-22）

- 问题：粘贴合集内某条视频（如 `BV1ZAWM61Evb`，合集第 8 集）解析后默认选中 EP1。
- 原因：`adapt_collection_result` 只在 URL 带 `?p=` 时设置 `requestedCid`，否则为 null，前端回退到首个可播放单元。
- 修复：无 `?p=` 时取入口稿件 P1 的 cid；带 `?p=` 时取对应 P。Vue 选集列表同时滚动到首个选中行，避免选中项在滚动区外不可见。
- 证据：`selects_the_pasted_collection_entry_instead_of_the_first_episode`、合集既有用例新增 `requested_cid` 断言。
