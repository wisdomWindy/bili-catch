# 模块：合集/番剧元数据解析

## 模块标识

- module id：`episode-metadata`
- 类型：table/list workflow（数据获取与归一）
- 执行顺序：2
- 下游工件根：`docs/requests/bilicatch-collection-bangumi-support-20261114/module-runs/episode-metadata/`

## 来源快照（原文）

> “合集/番剧批量下载”（`biliCatch_PRD.md:32`）
>
> “合集/番剧 | 是否与普通多 P 共享模型 | 外部接口待补 | 解析适配器规范化为统一 part 列表”（`bilicatch-v1-desktop/analysis/requirement-analysis.md:151`）
>
> “合集/番剧的受限内容与账号权限依赖真实账号和线上环境，自动化测试需要契约夹具或授权测试账号。”（`bilicatch-v1-desktop/analysis/requirement-analysis.md:165`）
>
> 用户确认（2026-11-14）：“统一选集列表”“展开到 P”（H2、H4）

接口事实（见 `artifacts/prd-snapshot.md` X1–X4）：`ugc_season.sections[].episodes[]` 每条含 `bvid`/`cid`/`title`/`page`；`pgc/view/web/season` 的 `result.episodes[]` 含 `ep_id`/`bvid`/`cid`/`long_title`/`badge`/`duration`（毫秒）/`rights`/`section_type`。

## Markdown 归一化快照

不适用：上游为 Markdown 与 JSON 接口样例，已在 `artifacts/prd-snapshot.md` 以表格承载，无需额外转换。

## 来源追踪

- S4、S9、S10、C6–C11、X1–X4、H2、H4
- 现有实现：`raw.rs:12/29/39`、`adapter.rs:23/123`、`client.rs:218`

## 来源锚定

| 项目 | 标签 | 引用 |
| --- | --- | --- |
| 合集成员可作为集列表返回 | source-backed + source-derived | S4、X1 |
| 番剧集列表来自季接口 | source-backed + source-derived | S4、X3 |
| 时长需归一（毫秒→秒） | source-derived（接口事实） | X3 |
| 排除预告/花絮 section | missing-source（默认口径 Q6） | Q6 |
| 失效成员容错 | external-interface-pending | Q7 |
| 不展示简介/评分/追番 | out-of-scope | PRD §1.3 其余行 |

本模块允许的产品内容：P2（集列表展示数据）、P8（分 P 层级数据）。不新增任何展示字段。

## 业务意图

把 B 站两种结构的“多集”数据统一成一种可消费的列表：每个单元携带自身标识、标题、时长、可用性输入，供契约层与 UI 直接使用；同时不让接口字段差异（秒/毫秒、sections、缺失成员）漏到下游。

## 用户可见范围

- 解析结果中出现合集成员列表或番剧集列表，顺序与接口返回顺序一致。
- 每项显示集标题与时长；不显示简介、评分、追番状态、UP 主其他合集。
- 单 P 集与多 P 集以同构数据表达，由模块 4 决定呈现细节。

## 表单、表格、展示与交互

- 无表单。
- 列表数据：序号/集号、标题、时长（秒）、可用性输入（`need_login`/`need_vip`/`badge`/`allow_download`/`is_preview`）、分区类型、所属稿件标识。
- 排序：保持接口顺序，不新增自定义排序。
- 空态：接口返回空集列表时按“无可用集”处理，由模块 4 展示。

## 流程与状态规则

1. 合集：`view` 返回 `ugc_season` 时，展开 `sections[].episodes[]`；无 `ugc_season` 时退化为单稿件行为（模块 1 的标识类型决定）。
2. 番剧：按 `ep_id` 或 `season_id` 请求季接口，取 `episodes[]`；同时保留季标题用于展示。
3. section 过滤：默认只保留可下载的正片 section，预告/花絮记录但不进入集列表（Q6）。
4. 缺失成员：接口列表为准；`ep_count` 与实际条数不一致时不补位、不猜测，缺员在可用性上体现（Q7）。
5. 时长：统一转为秒（向上取整或四舍五入需在 spec 固定一种），下游只消费秒。
6. 失败：接口非 0 码或结构缺失时返回 `E004`，不得返回“空列表成功”。

## 依赖与影响面

- 前置：模块 1 的标识与单元类型。
- 影响：`raw.rs`、`adapter.rs`、`client.rs`、解析结果 DTO。
- 相邻回归：普通 UGC 解析路径必须完全不变。

## 前端 / 后端责任边界

- Rust：请求、反序列化、映射、容错、归一（本模块全部在 Rust）。
- 前端：只消费 DTO，不做字段换算、不判断 section。
- 共享契约：集列表 DTO 字段与可空性。

## page-design 路由

不需要：本模块不直接产出布局；列表呈现归模块 4。

## 下游 spec 义务

- 明确两种接口的字段映射表与可空字段默认值。
- 明确 duration 单位换算规则与取整方式。
- 明确 section 过滤口径与 `section_type` 取值处理。
- 明确失效/缺失成员的表达方式（隐藏还是占位）。
- 明确“不得因单条异常导致整单失败”的容错边界。
- 明确契约夹具来源（X1–X3 实测响应）与脱敏要求。

## 开放问题

- Q6：预告/花絮默认排除，spec 记录默认并可改。
- Q7：失效成员的状态字段未实测，spec 先按“以枚举为准 + 缺员容错”落地，验证阶段补实测。
