# 需求拆分：合集与番剧支持

## 请求摘要

`bilicatch-collection-bangumi-support-20261114`：把合集（`ugc_season`）与番剧（PGC `ep`/`ss`）纳入现有“解析 → 选集 → 批量入队 → 下载”链路。范围由 PRD §1.3“合集/番剧批量下载”＋用户直接指令界定，四项产品口径已于 2026-11-14 确认。

## analysis 承接（carry-forward）

| 项 | 内容 | 对本阶段的影响 |
| --- | --- | --- |
| confirmed scope | 合集成员与番剧免费/限免集可解析、可展开到分 P、可多选批量下载 | 拆分必须覆盖“输入→元数据→取流→UI”完整链路 |
| 非目标 | 收藏夹/播放列表、系列、直播、课程、会员/付费/DRM/地区限制内容、整季一键、重写 UGC 链路 | 任何模块都不得引入这些内容 |
| 前端 / 后端职责 | Rust 拥有标识归一、权限判定、取流、任务校验；前端拥有选集状态、草稿构造、展示与文案 | 模块 1/2/3 属后端与共享契约，模块 4 属前端 |
| 已确认决策 | H1 仅免费/限免集；H2 统一选集列表；H3 多选批量；H4 展开到分 P | H1→模块 1/3；H2/H4→模块 1/4；H3→模块 4 |
| 剩余开放项 | Q5 序号定义、Q6 预告/花絮、Q7 失效集、Q8 `?p=`、Q9 勾选联动；PGC 错误码全集 | 全部为非阻塞或外部接口待补，不改变模块边界 |
| 依赖 | 契约（模块 1）先于 UI（模块 4）；任务字段（模块 1）先于取流（模块 3） | 决定顺序 1→2→3→4 |
| 路由信号 | 模块 4 需要 page-design；模块 1/2/3 需要 architecture-design | 见模块表 |

## 拆分原则

- 按“契约 → 数据 → 取流 → 呈现”分层，而不是按页面切，因为所有层都改同一套标识模型。
- 共享契约只在一处定义（模块 1），其余模块只消费。
- 外部接口适配与权限判定分离：适配只描述“是什么”，权限判定在模块 3 决定“能不能下”。
- 既有 UGC 链路不拆入任何模块，作为全局回归约束保留在原处。

## 来源清单

- `biliCatch_PRD.md:32`、`:35`、`:44`、`:318`
- `docs/requests/bilicatch-v1-desktop/artifacts/prd-snapshot.md:9`
- `docs/requests/bilicatch-v1-desktop/analysis/requirement-analysis.md:151`、`:165`
- 本请求 `artifacts/prd-snapshot.md`（C1–C16 代码事实、X1–X7 接口事实、S1–S10 原文）
- 本请求 `analysis/requirement-analysis.md`（范围、职责拆分、决策 H1–H4）

## 来源锚定承接

| 模块 | source-backed | code-fact-backed | confirmed-decision | source-derived | allowlist | 待补/非阻塞 | 禁止进入 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 1 | S4 批量、S6 权限分层 | C1–C5、C13–C15 | H2、H3、H4 | X1–X3 单元标识 | P1、P4、P7、P8 | Q8 默认、Q9 联动 | 收藏夹、系列、课程 |
| 2 | S4 合集/番剧解析 | C6–C11 | H2、H4 | X1–X3 字段、毫秒 | P2、P8 | Q7 失效态、错误码全集 | 评分/追番/弹幕 |
| 3 | S4 批量、S6 登录策略、S7 命名 | C10–C12 | H1、H3 | X5/X6 取流差异 | P4、P5、P6、P7 | 地区/DRM 表现 | 付费/DRM/绕过 |
| 4 | S4 批量、S8 范围 | C13–C15 | H2、H3、H4 | — | P2、P3、P8 | Q9 联动默认 | 新筛选、新入口 |

## Markdown 归一化说明

不适用：上游 PRD 与既有工件均为 Markdown，表格与字段清单已可直接消费，本次未发生格式转换，因此不需要额外保留原始快照副本。

## 模块清单

| ID | 名称 | 类型 | 来源映射 | 范围一句话 | 顺序 | page-design | architecture-design |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `episode-identity-and-contract` | 输入与单元契约 | cross-module rule | S1–S4、C1–C5、C13–C15、H2/H3/H4 | 识别 ep/ss/合集链接，定义单元与任务契约 | 1 | 否 | 是 |
| `episode-metadata` | 合集/番剧元数据解析 | table/list workflow（数据） | S4、C6–C11、X1–X3 | 取合集成员与番剧集列表并归一 | 2 | 否 | 是 |
| `episode-download` | 取流与权限 | interaction flow（后端） | S6、S7、C10–C12、H1、X5/X6 | 按权限取流、校验归属、命名 | 3 | 否 | 是 |
| `episode-selection-ui` | 统一选集界面 | page | S4、S8、H2/H3/H4、P2/P3/P8 | 容器/单元两级选集与提示 | 4 | 是 | 否 |

## 模块依赖与顺序说明

- 模块 1 是全部下游的契约前置；模块 2 消费模块 1 的单元类型；模块 3 消费模块 1 的标识字段与模块 2 的可用性字段；模块 4 消费模块 1 的契约与模块 2 的数据。
- 严格依赖：模块 2 不得在模块 1 通过 `review` 前开始；模块 3 不得在模块 2 通过 `review` 前开始；模块 4 不得在模块 1 通过 `review` 前开始（因此顺延为 4 最后）。
- 模块 4 的前端选择状态与既有分 P store 共用，必须等模块 1 契约稳定后改造，否则会二次返工。

## 模块执行顺序

1. `episode-identity-and-contract` → 起始阶段 `spec`（无需 page-design）
2. `episode-metadata` → `spec`
3. `episode-download` → `spec`
4. `episode-selection-ui` → `page-design` → `spec`

每个模块在 `spec` 通过框架审批后，若 `architecture_design_required=true`，进入 `architecture-design` 再进 `plan`。

## page-design 候选

- 模块 4：唯一需要 page-design 的模块。重点：容器/单元两级呈现、P/EP 标签、置灰与原因、勾选联动、与现有分 P 区块合并后不产生第二个列表。

## spec 承接清单

| 模块 | spec 必须保留 |
| --- | --- |
| 1 | 五类输入识别规则（BV/av/ep/ss/b23）、`?p=` 语义、单元四段标识、任务字段与兼容策略、错误码边界、匿名与登录语义不变 |
| 2 | 合集/番剧端口契约、字段映射、毫秒→秒归一、section 过滤、失效成员容错、可用性枚举取值 |
| 3 | 权限矩阵（免费/限免 vs 登录专享/会员/付费/地区/禁止下载）、PGC 取流参数、任务归属校验、命名 `{序号}_{标题}`、E005/E006 文案语义 |
| 4 | 统一列表结构、P/EP 标签、不可选态与原因、全选/反选/计数的粒度、勾选联动默认、空/加载/失败态、与既有分 P 的合并方式 |

## 开放问题

- Q5 序号定义：spec 默认取接口集号，记录可逆。
- Q6 预告/花絮：spec 默认只展示可下载的正片 section。
- Q7 失效集：外部接口待补，模块 2 以实际枚举为准做容错。
- Q8 `?p=`：spec 默认忽略并选中第一个可下载单元。
- Q9 容器/单元勾选联动：spec 必须写死一种行为（建议父选中全部子），并在 page-design 体现。
- PGC 错误码全集：模块 3 只映射已实测与语义明确的码，其余归一为可读失败。
