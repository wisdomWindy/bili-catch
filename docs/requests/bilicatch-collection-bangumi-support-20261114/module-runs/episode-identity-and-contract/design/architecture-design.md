# 架构设计：输入与单元契约（episode-identity-and-contract）

## 交付单元

- request：`bilicatch-collection-bangumi-support-20261114`
- module：`episode-identity-and-contract`（顺序 1 / 4）
- 依据：framework-approved `spec`（v2）、`clarifications`（Q1–Q10）、模块需求工件。

## 架构目标

用**最小结构变化**把“目标标识”和“可下载单元”变成全链路一等概念：新增枚举变体、扩展现有 DTO 字段、补默认值兼容；不新增模块、不新增请求层、不引入第二套模型。

## 架构范围与触发条件

触发：spec 要求 DTO 与任务契约扩展、输入识别扩展、旧数据兼容；这些改动跨越 Rust 输入层、模型层、任务层与 TS 契约，属于架构敏感。
范围：仅本模块 4 个交付面（输入归一、解析 DTO、任务字段、前端契约与校验）。不含合集/番剧取数、取流与 UI。

## 上游输入与假设

- `spec` v2：字段、语义、不变式、AC-1…AC-9。
- 代码事实：`input.rs:26/97/119/124`、`adapter.rs:123`、`parser.rs:109`、`models/parse.rs:28`、`models/task.rs:65/87`、`manager.rs:123`、`validation.rs:35`、`input.ts:12`、`contracts.ts:14/71`、`store.ts:322`。
- 假设：任务持久化为 serde JSON；无外部迁移脚本（见 `artifacts/code-context.md`）。

## 脚手架对齐

不适用：既有工程内增量改造，无脚手架决策。

## 专家架构决策记录

| ID | 问题 | 决策 | 备选与被拒原因 | 依赖/所有权 | 迁移与验证 |
| --- | --- | --- | --- | --- | --- |
| ADR-1 | 番剧标识放在哪 | 扩展 `bilibili::input::VideoId`：新增 `Episode(u64)`、`Season(u64)` | 新建 `InputTarget` 枚举会导致两套标识并存，调用点需转换 | 归属 infrastructure，`pub use` 保持现状 | 穷尽 match 迫使所有消费点处理；单测覆盖 |
| ADR-2 | 契约权威 | Rust `models::*` 为唯一权威，TS 手写镜像 | 代码生成（ts-rs）引入构建依赖与产物同步成本，字段量小 | 无新依赖 | 双侧字段名/可空性逐个对齐；`pnpm build` 兜底 |
| ADR-3 | 是否新建 `EpisodeUnit` 类型 | 复用 `VideoPart` 扁平扩展（加 `bvid`/`epId`/`partCount`/`groupIndex`/`availability`） | 新建独立类型需在 store/selector/draft 间加转换层，破坏既有消费方 | `VideoPart` 仍是唯一单元类型 | 逐字段填充；旧字段语义不变 |
| ADR-4 | 可用性如何表达 | `availability`（playable/unavailable）+ `Option<UnavailableReason>` 两层 | 单一大枚举会把“状态”和“原因”耦合，新增原因要改状态判断 | 契约层定义，模块 2/3 填充 | 不变式测试：unavailable 必带原因 |
| ADR-5 | 旧任务兼容 | 新字段全部 `#[serde(default)]` + 自定义 default 函数 | 引入 `schemaVersion` 需要迁移器与双读路径，成本高于收益 | `models/task.rs` 拥有默认值 | 旧 JSON 反序列化用例 |
| ADR-6 | 模块 1 期间 ep/ss 怎么返回 | `ParserService::parse` 对 `Episode/Season` 返回 `E004`（“暂不支持该内容类型”），不伪造数据 | 直接返回空结果会违反“空即失败”；提前实现 PGC 取数越过模块 2 边界 | 临时路径 owner：`ParserService` | **清理触发**：模块 2 合入时删除该分支；验证：单测断言错误码 |
| ADR-7 | 两端校验如何保持一致 | 同一份输入矩阵分别落在两侧测试；Rust 为权威 | 共享 JSON 夹具需跨语言加载与路径依赖，收益低于成本 | 测试层各自拥有 | 矩阵行数一致；不一致以后端为准 |

## 业务能力与所有权图

| 能力 | Owner | 禁止的 owner |
| --- | --- | --- |
| URL → 目标标识 | Rust `bilibili::input` | 前端不得自行解析出 ep_id |
| 即时校验与反馈 | 前端 `download-center/input.ts` | 不得作为权威 |
| 解析 DTO 形状 | Rust `models::parse` | 前端不得扩字段 |
| 单元字段填充（UGC） | `bilibili::adapter` | 不得在 handler/command 里补字段 |
| 任务字段与默认值 | Rust `models::task` + `tasks::manager` | 前端不得省略必填语义 |
| 草稿构造 | 前端 `download-center/store` | 不得推导可用性/来源 |

## 模块边界

- `infrastructure/bilibili`：外部输入与 B 站数据 → 内部模型。
- `models`：跨层稳定契约（纯数据）。
- `services::parser`：流程编排。
- `services::tasks`：任务生命周期与校验。
- `features/download-center`：前端消费与交互。

## 文件与目录结构

不新增文件。改动落点：

```
src-tauri/src/infrastructure/bilibili/input.rs      (+ 目标识别、枚举变体)
src-tauri/src/infrastructure/bilibili/adapter.rs    (parts 字段填充)
src-tauri/src/models/parse.rs                       (+ 枚举/结构/字段)
src-tauri/src/models/task.rs                        (+ 字段/默认值)
src-tauri/src/services/parser.rs                    (+ 剧集分支占位)
src-tauri/src/services/tasks/manager.rs             (+ 字段透传)
src-tauri/src/services/tasks/validation.rs          (+ 一致性校验)
src/features/download-center/{input,contracts,store}.ts
```

## 代码关系与依赖方向

`commands → services → infrastructure → models`；`models` 不依赖任何上层。前端：`pages → features → contracts`，`features` 之间不互相 import。

## 状态拓扑与数据生命周期

- Writable：`tasks::manager` 的任务表；前端 store 的 `selectedCids`（模块 4 改为树形）。
- Derived：`VideoPart` 的 `partCount`/`groupIndex`/`availability` 由 Rust 派生，前端只读；草稿字段由 store 纯函数派生。
- Async：解析请求与任务事件，owner 不变。
- 生命周期：`ViewData → VideoPart → TS VideoPart → draft → task record`；旧任务缺字段 → serde 默认值补齐。

## 依赖规则矩阵

| 从 | 允许 | 禁止 |
| --- | --- | --- |
| `input.rs` | `models::AppError`、`url` | `raw`/`client`/`adapter` |
| `models::parse` | serde | infrastructure、services |
| `adapter.rs` | `models::parse`、`raw` | commands、task 层 |
| `parser.rs` | adapter、port、models | tasks 内部结构 |
| `tasks::manager` | `models::task`、validation | bilibili infrastructure 内部 |
| `contracts.ts` | 无 | store、组件 |
| `store.ts` | contracts、input、service | 组件、页面 |

公开 API：`VideoId`（已 pub）、`ParseVideoResult`/`VideoPart`/`ParseContainer`/`Source`/`Availability`、`DownloadTaskDraft`/`DownloadTask` 新字段。私有：`parse_input_target` 保持模块内。

## 扩展缝与拒绝的过度设计

- 真实变化轴：来源（ugc/bangumi）× 容器种类（video/season）。已用枚举表达，够用。
- 拒绝：`SourceStrategy` trait、`ParserRegistry`、`DtoVersion` 迁移器、`UnitAdapter` 泛型层——均只有 1–2 个实现，无独立变化节奏。

## 复杂度预算

- 新增文件 0；新增公开类型 4（`VideoSource`、`ContainerKind`、`ParseContainer`、`Availability` 及原因枚举）；间接层级 0；迁移脚本 0；允许触碰文件 ≤ 10（含测试）。
- 超出预算（例如为番剧新增独立 port 文件）→ 留给模块 2。

## 迁移、兼容与清理

- 临时路径：`parser.rs` 的 `Episode/Season → E004`。Owner `ParserService`，清理触发为模块 2 合入，回滚面为删除该 match 分支。
- 兼容：serde 默认值即迁移；无双重读路径。
- 旧前端契约：TS 新增字段后，所有构造点必须补齐（测试夹具同步）。

## 就近与文件抽取决策

| 逻辑 | 真实调用方 | 决策 |
| --- | --- | --- |
| `parse_input_target`（ep/ss 段解析） | 1（`normalize_input`） | 就近放 `input.rs` 私有函数 |
| `is_supported_url`（前端） | 1（`normalizeParseInput`） | 就近放 `input.ts` |
| 默认值函数 | 各自结构体 | 就近放 `models/task.rs` |
| `availability` 构造 | 1（adapter） | 直接内联常量，不抽 helper |

## 复用候选清单

| 候选 | 现有位置 | 语义等价 | 使用点 | 变因 | 决策 |
| --- | --- | --- | --- | --- | --- |
| 主机白名单校验 | `input.ts:8` / `input.rs:44` | 是（镜像） | 各 1（跨语言） | 各 1 | 保持分离（运行时不同） |
| 最后一段 ID 提取 | `input.ts:19` / `input.rs:121` | 是（镜像） | 各 1 | 各 1 | 保持分离，共享用例矩阵 |
| `VideoPart` 构造 | `adapter.rs` | — | 1（模块 2 复用） | 2 | 保持就近，模块 2 再评估 |

## Anti-DRY 矩阵

| 候选 | 业务语义 | 分层 | 真实生产使用点 | 变化稳定性 | 变因数 | 结论 |
| --- | --- | --- | --- | --- | --- | --- |
| 主机校验 | 相同 | 前端/后端两运行时 | 1+1 | 稳定 | 1 | 保留重复（跨语言无法共享） |
| ID 提取 | 相同 | 同上 | 1+1 | 稳定 | 1 | 保留重复 + 同源用例 |
| 默认值 | 各自结构 | 模型层 | 1 | 新引入 | 1 | 就近 |

共性分类：技术/基础设施（跨语言镜像）；不晋升 shared API。

## 共享抽象决策

不抽取。理由：两个候选都跨语言，抽取需要共享 schema/协议层，成本远超收益；风险由“同一输入矩阵两端都测”控制，不依赖代码共享。

## 依赖注入与策略边界

不适用：无共享抽象，无策略注入需求。

## 职责切分

- Rust 拥有：权威识别、字段语义、默认值、校验。
- 前端拥有：即时反馈、DTO 消费、草稿构造。
- 测试拥有：输入矩阵与反序列化兼容。

## 函数设计与公开入口

- `normalize_input(&str) -> Result<NormalizedInput, AppError>`（不变）
- 私有 `parse_input_target(&str) -> Option<VideoId>`（BV/av/ep/ss）
- `adapt_parse_result(ViewData, PlayData, Option<u32>) -> Result<ParseVideoResult, AppError>`（签名不变）
- `validate_create_request(...)`（不变，内部增加一致性检查）
- 前端 `normalizeParseInput(input) -> ParseInputResult`（不变）

## 功能边界与副作用模型

- 纯函数：目标识别、字段派生、默认值。
- 副作用：仅 IPC/文件/任务表写入，owner 不变。
- 不可变：`VideoPart` 一经产出不再修改；草稿为新建对象。

## 状态所有权与数据流

前端 `selectedCids: number[]` 仍是唯一可写选择状态（模块 4 扩展为树）；本模块不新增状态，只让草稿从单元字段派生。

## 数据结构与类型策略

- Rust：`#[serde(rename_all = "camelCase")]`；枚举 `kebab-case`。
- 可空用 `Option`；不用空字符串。
- TS：`string | null`、`number | null`；不使用 `any`。
- `VideoPart.bvid` 为 `String`（必填）：UGC 与番剧单元都有 bvid（X3）；季级无单元，因此不影响。

## 契约与适配边界

B 站 JSON → `raw` → `adapter` → `models` → IPC → TS。归一（时长、可用性）在 adapter；本模块只补字段，不做跨来源判断（模块 2 负责）。

## 模式决策与被拒替代

- 信号：按类型分派、契约适配、两级结构。
- 决策：Level 0 direct code，`enum` + 穷尽 `match`；TS 可辨识联合。
- 拒绝：Strategy/Factory（无独立变化节奏）、Visitor（层级只有两层且不扩展）、DTO 基类（serde 不友好）。
- 复审信号：出现第三种来源或容器层级变为三层时重新评估。

## 可读性与维护护栏

- 字段命名与既有 camelCase 一致；新字段必须写一行语义注释。
- 不变式用 debug 断言或校验函数表达，不靠注释约定。
- 禁止在多个位置重复构造 `availability` 常量（adapter 内一次性映射）。

## 架构风险

| 风险 | 影响 | 缓解 |
| --- | --- | --- |
| TS/Rust 字段漂移 | 编译过但运行缺字段 | 双侧测试 + `pnpm build` |
| 旧任务被默认值掩盖 | 数据错误静默 | 反序列化用例 + 任务创建校验 |
| 临时 E004 被遗忘 | 番剧永久不可用 | ADR-6 清理触发写入计划任务 |
| `VideoPart` 字段膨胀 | 契约变重 | 只加单元必需字段，展示字段留给模块 4 |

## 开放架构问题

- 模块 2 是否需要在 `parse_input_target` 基础上暴露 `Season` 解析结果给 PGC 端口：届时评估是否将其提升为 `pub(crate)`，本模块不预做。
- 任务持久化若改为自有格式（非 serde JSON），默认值策略需重评；当前按 serde 设计。
