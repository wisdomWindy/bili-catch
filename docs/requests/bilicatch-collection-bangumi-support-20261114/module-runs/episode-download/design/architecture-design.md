# 架构设计：取流与权限（episode-download）

## 目标

在不复制下载链路的前提下，让同一套 executor 能服务 UGC 与番剧：差别只在“取流来源解析”，下游（下载、合并、转码、进度、任务）不变。

## 决策

| ID | 问题 | 决策 | 拒绝的备选 | 验证 |
| --- | --- | --- | --- | --- |
| ADR-15 | 能力字段映射重复 | 从 `adapt_parse_result` 抽出 `capabilities_from_play(&PlayData) -> PlayCapabilities`，UGC/番剧共用 | 复制一份番剧版会双维护 | 既有 adapter 断言不变 + 番剧能力用例 |
| ADR-16 | 番剧取流 | 新增 `BilibiliPort::fetch_bangumi_playurl(ep_id, cid, auth) -> PlayData` | 复用 UGC playurl（X6 实测流被压缩/失败） | 端口单测 + parser 用例 |
| ADR-17 | 请求携带来源 | `VideoSourceRequest`/`AudioSourceRequest` 增加 `source`、`ep_id`；在 executor 边界从 task 填充 | 把整个 task 传进端口会扩大耦合 | 编译期 + executor 测试 |
| ADR-18 | 权限判定位置 | `ParserService` 在取流前用 season 可用性判定，`adapter` 暴露 `bangumi_episode_availability` | 放在 client 会混入产品语义 | 三态用例 |
| ADR-19 | 命名分流 | 草稿/任务增加 `container_kind`（默认 video）；`filename.rs` 增加 `sanitize_season_filename` | 用 `group_index>1` 推断会漏掉首个成员 | 命名单测 + manager 用例 |
| ADR-20 | 临时状态 | 删除模块 2 的“能力为空是常态”说明；解析仍允许降级 | 强制失败会因单集受限丢整季 | 降级用例 |

## 所有权

| 能力 | Owner |
| --- | --- |
| PGC 取流 URL/解码 | `client.rs` |
| 能力映射 | `adapter::capabilities_from_play` |
| 可用性判定 | `adapter::bangumi_episode_availability` |
| 权限错误映射 | `parser.rs` |
| 归属校验 | `parser.rs` |
| 命名 | `tasks::filename` |
| 字段填充 | executor 边界 |

## 文件结构

不新增文件。改动：`models/task.rs`、`infrastructure/bilibili/{adapter,client,mod}.rs`、`services/parser.rs`、`services/video/{source,executor,track}.rs`、`services/audio/{source,executor}.rs`、`services/tasks/{filename,manager}.rs`、前端 `contracts.ts`、`store.ts`。

## 依赖方向

`executor → port(trait) ← parser`；`parser → adapter/client`；`tasks::manager → filename`。无循环。前端只加字段。

## 状态与数据流

任务记录 → executor 构造请求（source/epId）→ parser 解析源 → 下载。无新增可写状态。

## 依赖规则

禁止：`filename` 依赖 bilibili 模块；`adapter` 发请求；`executor` 内联权限规则。

## 复用与 Anti-DRY

| 候选 | 结论 |
| --- | --- |
| 能力映射 | 抽取（2 个真实调用方，语义相同） |
| 权限判定 | 单点 `bangumi_episode_availability`，解析与下载共用 |
| 命名 | 复用 `normalize_stem`/`finish_filename`，仅新增序号前缀 |

## 复杂度预算

新增文件 0；新增公开函数 3（能力映射、可用性、season 命名）；新增端口方法 1；允许触碰 11 个生产文件。

## 迁移/兼容

- `container_kind` 默认 `video`；旧任务可读。
- 命名仅 season 变化，既有文件不动。
- 删除模块 2 遗留的“能力为空”作为最终状态的假设（代码路径保留降级）。

## 函数设计

- `capabilities_from_play(&PlayData) -> PlayCapabilities`（纯）
- `bangumi_episode_availability(&RawEpisode) -> (Availability, Option<UnavailableReason>)`
- `adapt_bangumi_result(season, requested_ep, play: Option<PlayData>)`
- `ParserService::fetch_bangumi_play(ep_id, cid, auth) -> Result<PlayData, AppError>`（私有）
- `sanitize_season_filename(group_index, part_title, bvid, page, ext)`

## 功能边界

纯函数：能力映射、可用性、命名。副作用：HTTP（client）、任务写盘（manager）。

## 模式决策

Level 0 direct code；分派点 2 个（parse 的 source 分支、resolve 的 source 分支）。拒绝 trait/策略（只有两来源）。

## 风险

| 风险 | 缓解 |
| --- | --- |
| 端口签名扩散导致测试大改 | 只加字段，不改既有方法 |
| 权限先判后取流仍可能被接口拒绝 | 取流失败按错误码兜底 |
| 命名变更影响老任务 | 仅新任务按新规则，旧任务沿用其已存 `file_name` |
