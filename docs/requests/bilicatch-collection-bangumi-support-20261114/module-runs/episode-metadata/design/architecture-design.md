# 架构设计：合集/番剧元数据解析（episode-metadata）

## 交付单元

`episode-metadata`（2 / 4）。依据：framework-approved spec 与 clarifications Q1–Q8、模块 1 契约。

## 架构目标

在现有 `raw → adapter → models` 管线上加两条“多单元来源”，不新建模块、不引入端口抽象层：合集复用现有 `view` 请求（只扩 raw 模型），番剧新增一个端口方法与一个独立 envelope。

## 范围与触发

触发：需要新增 HTTP 来源、扩展反序列化模型、新增纯函数适配、删除模块 1 的临时分支。
范围：`raw.rs`、`client.rs`、`adapter.rs`、`parser.rs`、`mod.rs`、前端 `contracts.ts`、`store.ts`。

## 上游输入与假设

spec AC-1…AC-9；X1–X4 实测字段；假设 `ep`/`ss` 共用 season 接口、PGC episodes 无内嵌 pages。

## 脚手架对齐

不适用。

## 专家架构决策记录

| ID | 问题 | 决策 | 备选/拒绝 | 迁移与验证 |
| --- | --- | --- | --- | --- |
| ADR-8 | PGC 响应 envelope | 新增 `PgcResponse<T> { code, message, result }` | 复用 `ApiResponse`（字段名不同：`data` vs `result`）会反序列化失败 | 单测反序列化 |
| ADR-9 | 端口形状 | 新增 `fetch_bangumi_season(ep_id, season_id)` 一个方法，参数用 `Option` 二选一 | 两个方法（ep/season）重复 URL 构造；trait 参数用枚举更重 | 单测断言 URL 选择 |
| ADR-10 | 合集是否需要新请求 | 不需要，扩 `ViewData.ugc_season` | 另请求 season_archives 会引入分页与额外失败面 | 复用 view fixture |
| ADR-11 | 两个适配器是否抽象 | 各自独立纯函数（`adapt_collection_units` / `adapt_bangumi_units`） | 抽象 shared "member→unit" 需要统一两种结构，变因 >3 | 行为等价测试各自覆盖 |
| ADR-12 | 时长归一 | 单点 `duration_ms_to_seconds(ms) = (ms + 500) / 1000` | 各自内联会漂移；floor 会偏短 | 单测含边界 499/500/1203160 |
| ADR-13 | 默认选中 | 新增 `requestedCid`，前端优先级 cid→page→首个 playable | 复用 `requestedPage` 会与 P 语义冲突 | 前端 store 测试 |
| ADR-14 | 模块 1 占位分支 | 删除；`parse` 改为按目标类型分派三条路径 | 保留会造成番剧永久 E004 | 覆盖测试断言不再返回 E004 |

## 能力与所有权

| 能力 | Owner |
| --- | --- |
| 合集展开 | `adapter::adapt_collection_units` |
| 番剧展开与过滤 | `adapter::adapt_bangumi_units` |
| PGC 请求 | `client::fetch_bangumi_season` |
| 时长归一 | `adapter::duration_ms_to_seconds` |
| 路径分派 | `ParserService::parse` |
| 默认选中 | 前端 `store.applyResult` |

## 模块边界与文件结构

不新增文件。`raw.rs` 增加 `PgcResponse`/`SeasonData`/`RawEpisode`/`RawUgcSeason`/`RawSeasonSection`/`RawSeasonEpisode`；`client.rs` 增加方法；`adapter.rs` 增加两个纯函数；`parser.rs` 改为三路分派。

## 依赖方向

`parser → client/adapter → raw → models`；`adapter` 不依赖 `client`；前端 `store → contracts`。无循环。

## 状态拓扑与数据流

无新增可写状态。数据流：HTTP JSON → `raw` → `adapter` 纯函数 → `ParseVideoResult` → IPC → store 默认选中（纯派生）。

## 依赖规则矩阵

允许：`adapter` 用 `raw` + `models`；`client` 用 `raw` + `models` + auth。
禁止：`raw` 依赖任何服务；`adapter` 发请求；前端判断可用性。

## 扩展缝与拒绝的过度设计

真实变化轴：来源（ugc 合集 / pgc 季）；已用两个纯函数 + 一个分支表达。拒绝：`SeasonSource` trait、通用 `UnitMapper`、分页器（当前数据一次返回）。

## 复杂度预算

新增文件 0；新增公开类型 2（`SeasonData`、`PgcResponse` 内部 crate 级）；新增端口方法 1；允许触碰 8 个生产文件。

## 迁移/兼容/清理

- 删除模块 1 的 `Episode/Season → E004` 分支（清理触发已满足）。
- `ParseVideoResult` 新增 `requestedCid`：TS 契约同步；旧数据无此字段（解析结果不持久化，无需兼容）。
- 前端默认选中新增优先级，不改变既有单视频行为（`requestedCid` 为 null 时回退 page）。

## 就近与文件抽取

| 逻辑 | 调用方 | 决策 |
| --- | --- | --- |
| `duration_ms_to_seconds` | 1 | 就近 `adapter.rs` 私有函数 |
| 可用性判定 | 1 | 内联在 `adapt_bangumi_units` |
| URL 构造 | 1 | 内联 `client.rs` |

## 复用候选与 Anti-DRY

| 候选 | 语义 | 分层 | 使用点 | 变因 | 结论 |
| --- | --- | --- | --- | --- | --- |
| 成员→单元构造 | 相似但来源不同 | adapter | 2 | 3（字段、时长单位、可用性） | 保持分离；仅共享时长换算 |
| 时长换算 | 相同 | adapter | 1（未来 PGC 复用） | 1 | 单函数，暂单项 |

## 职责切分 / 函数设计

- `fetch_bangumi_season(ep_id: Option<u64>, season_id: Option<u64>, auth)` → `Result<SeasonData, AppError>`
- `adapt_collection_units(&RawUgcSeason, entry: &ViewData) -> Vec<VideoPart>`
- `adapt_bangumi_units(&SeasonData) -> Vec<VideoPart>`
- `ParserService::parse` 三分支：合集（view+ugc_season）、番剧（season）、单稿件（原路径）。
- 前端 `applyResult` 的选中函数保持纯函数。

## 功能边界与副作用

纯函数：两个 adapter、时长换算、前端选中。副作用：仅 `client.rs` 的 HTTP。

## 数据结构策略

`SeasonData` 为内部 crate 类型（`pub(crate)`），不直接暴露 IPC；对外的仍是 `ParseVideoResult`。Pgc 字段全部 `#[serde(default)]` 以容错。

## 契约与适配边界

PGC 的 `result` → `SeasonData` → `VideoPart`；前端只见 `ParseVideoResult`。`Badge` 判定在 adapter 内完成，不泄漏字符串到前端。

## 模式决策

Level 0 direct code；两个纯函数 + 分支。拒绝 trait/strategy（只有两个来源、无独立生命周期）。

## 可读性护栏

- 两个 adapter 必须对称命名、同样的字段顺序。
- 不可用原因用枚举，禁止前端解析 badge 文案。
- 时长函数带注释说明取整规则。

## 架构风险

| 风险 | 缓解 |
| --- | --- |
| PGC 字段漂移 | `serde(default)` + 跳过坏条目 + fixture |
| 能力为空被误认为解析失败 | 由模块 4 显示“暂无可用选项”，不报错 |
| 合集单元过多 | 不做上限；任务容量 100 兜底 |
| 删除占位分支后回归 | parser 测试覆盖三条路径 |

## 开放问题

- 合集是否需要分页接口（成员超过内嵌上限）：当前不做，出现真实超长合集时再评估。
- `badge` 的会员/付费文案稳定性：模块 3 取流复核。
