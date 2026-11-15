# 规格：输入与单元契约（episode-identity-and-contract）

## 交付单元

- request：`bilicatch-collection-bangumi-support-20261114`
- module：`episode-identity-and-contract`（顺序 1 / 4）
- 交付物边界：输入识别、规范化标识、解析结果 DTO、任务草稿/任务字段、前后端校验同构。不含 B 站取数、取流与 UI。

## 背景与目标

现有链路只把 UGC 稿件的 `pages` 当作可选单元，且输入层只识别 BV/av（C1–C4）。因此合集内链接只能得到当前一条视频，番剧链接直接报 `E003`。本模块建立“单元”这一等一概念：一次解析结果由若干容器（稿件/季）与若干可下载单元（分 P）组成，单元自带可持久化标识，后续模块只消费不再改契约。

目标：任何受支持的输入都能被规范化为稳定的目标标识；任何选中单元都能被唯一表达进任务，且旧任务仍可读取。

## 范围内

1. 输入形态识别（前端即时校验与 Rust 权威校验同构）。
2. 规范化目标标识（稿件 / 季 / 单集）。
3. 解析结果 DTO 扩展：容器、单元、可用性、来源。
4. 任务草稿与任务字段扩展：单元级标识与可缺省兼容。
5. 错误语义边界：`E003` / `E004` 的归属不变。
6. 既有任务的读取兼容。
7. UGC 路径上的单元字段最小填充（`adapter.rs` 的 parts 映射）与任务创建字段透传（`manager.rs`），保证交付单元可构建、可回归；不放宽为合集/番剧取数。

## 范围外

- 合集/季数据获取（请求、反序列化、字段适配规则，模块 2）；本模块只允许把现有 `ViewData` 映射成新单元形状，不新增 B 站请求。
- PGC 取流、权限判定、命名（模块 3）。
- 选集列表布局、置灰呈现、文案（模块 4）。
- 收藏夹、播放列表、系列、直播、课程（请求级 out-of-scope）。
- 修改 WBI 签名、下载器、任务队列调度。

## 触发与开始条件

用户粘贴链接/编号并点击解析；或任务创建请求携带草稿；或已持久化任务在启动时被读取。

## 需求拆分承接

本模块承接 allowlist P1（输入识别）、P4（任务单元）、P7（兼容）、P8（层级），以及 H2（统一选集列表）、H3（多选批量，字段层面）、H4（展开到 P，标识层面）。不承接 P2/P3/P5/P6。

## 来源锚定与可追溯性

| spec 项 | 标签 | 是否产品可见 | 来源 | plan 义务 |
| --- | --- | --- | --- | --- |
| 识别 `/bangumi/play/ep…` | source-backed（用户指令 S1/S3） | 是 | S1、S3；`analysis`（H1） | 前后端识别与用例 |
| 识别 `/bangumi/play/ss…` | source-derived | 是 | S1；理由见 clarifications Q2 | 前后端识别与用例 |
| 识别合集内 BV 链接 | source-backed | 是 | S2、S4 | 保持 BV 行为 |
| `?p=` 仅对 BV/av 生效 | confirmed-decision（Q8 默认） | 是 | `analysis` Q8 | 显式测试 |
| 单元需携带自身 bvid/cid | source-derived（接口事实 X1） | 否（内部） | X1、X2；C15 | 字段扩展 |
| 番剧单元需携带 ep_id | source-derived（接口事实 X3/X5） | 否（内部） | X3、X5 | 字段扩展 |
| `page` = 容器内 P 序号 | confirmed-decision | 否（内部） | H4；C5 | 语义测试 |
| `partCount` = 容器内 P 数量 | confirmed-decision | 否（内部） | H4；C14 | 语义测试 |
| 结果顶层 bvid/aid 可为空 | source-derived（X3：季无 bvid） | 是（展示方决定隐藏） | X3 | 可空契约 |
| 旧任务默认 UGC 来源 | code-fact-backed（兼容） | 否 | C14、C15 | 反序列化默认值 |
| 错误码不改语义 | confirmed-decision | 是 | `analysis` invariants 6 | 错误映射测试 |
| 匿名/登录能力边界不变 | source-backed | 是 | S6 | 不新增权限分支 |

封闭产品内容允许清单：本模块只允许新增“把 ep/ss 链接判为可解析”“解析结果可表达容器与单元”“任务可表达番剧单元”三类内容；其余字段、文案、控件一律禁止。

## 工作流效率档位

`S3 broad`，full 密度。本模块横跨前端校验、Rust 输入层、共享 DTO 与任务模型，上下文必须覆盖两侧定义与既有测试。

## 自愈完成门槛

- 需求一致性：四类输入行为与既有五类输入不回归。
- 业务逻辑：标识唯一、`page`/`partCount` 语义、可空字段、旧任务兼容。
- 语法/类型：`cargo test`、`pnpm test`、`pnpm build`、`cargo clippy`（若可用）通过。
- 运行时安全：空输入、超长输入、非法 ep/ss、序列化缺失字段均显式失败或显式默认。
- 代码组织：无重复标识解析逻辑；导出与调用方同步；测试夹具更新。
- 可维护性：命名一致、无魔法值、无死代码、无过期注释。
- 技术债：不允许留下“两端规则不一致”的临时实现。
- 完成条件：`self-healing loop compliance: pass`、`self-healing loop assessment: pass`、剩余 blocker `none`。

## 用户意图合同

- 字面：支持番剧/合集。
- 实际目标：粘一条合集/番剧链接就能拿到该集合集，且不会下错视频。
- 成功标准：识别、标识、任务三者一致；错误只出现在“确实不支持”时。
- 禁止解释：只把 ep 映射成 bvid；用顶层 bvid 生成所有任务；把不可识别输入吞掉返回空列表。
- 可接受方案：扩展标识与契约，保留既有链路。
- 校验点：`page`/`partCount` 语义测试、旧任务反序列化、两端输入用例同构。

## 用户流程

1. 粘贴 → 前端 `normalizeParseInput` 判定形态 → 合法则调用 `parse_video`。
2. Rust `normalize_input` 独立判定 → 输出目标标识与 `?p=`。
3. 解析结果返回容器与单元 → 前端 store 消费（本模块只保证 DTO 可消费）。
4. 用户选择 → 前端构造草稿（单元字段）→ Rust 校验 → 任务记录。
5. 重启读取旧任务 → 缺省字段回退 `ugc`。

## 页面与模块设计

不适用：本模块不改页面结构；仅改变输入合法性与 DTO 形状。页面行为承接见模块 4。

## UI / UX 设计合同

不适用：无 `page-design`；本模块不新增可视区域。唯一的用户可感知变化是“ep/ss 输入不再报无效”，其错误文案沿用现有 `E003` 展示。

## 前端样式约束

不适用：无样式改动。

## 字段展示语义与交互视觉态

不适用：本模块不定义展示；`bvid` 可空后的隐藏规则归模块 4。此处只固定：可空字段在 DTO 中为 `null`，不用空字符串伪造。

## 功能完整行为拆解

### 1. 输入形态识别

支持的输入（前后端同构）：

| 形态 | 示例 | 结果 |
| --- | --- | --- |
| BV 编号 | `BV1kw8b6bEeL` | 稿件 |
| av 编号 | `av170001`、`AV170001` | 稿件 |
| 视频页 | `https://www.bilibili.com/video/BV…?p=2` | 稿件 + `p` |
| 移动视频页 | `https://m.bilibili.com/video/av170001` | 稿件 |
| 短链 | `https://b23.tv/xxxx` | 短链，解析后归一 |
| 番剧单集 | `https://www.bilibili.com/bangumi/play/ep733316` | 番剧单集 |
| 番剧季 | `https://www.bilibili.com/bangumi/play/ss28747` | 番剧季 |
| 番剧短链 | `https://b23.tv/ep733316` | 短链 → 番剧单集 |

规则：

- 前后端都必须接受上述形态；后端为权威，前端仅做即时反馈，判定不一致时以后端结果为准。
- 允许结尾 `/`；允许任意 query（`spm_id_from`、`vd_source`、`share_source`）。
- `p` 参数仅对稿件生效：正整数有效，`0`/非数字 → `E003`。番剧链接上的 `p` 被忽略，不参与选中（Q8）。
- 空白处理沿用现状：先 `trim`，内部含空白或控制字符 → `E003`。
- 反斜杠转义（`spm\_id_from`）继续被还原后解析，不得因新增形态回归。
- 纯空格、空串 → 前端 `empty`（不请求），后端 `E003`。

### 2. 规范化目标标识

- 目标类型：`Video`（bvid/aid）、`Season`（season_id）、`Episode`（ep_id）。
- 短链必须先解析为 `Video` 或 `Episode`/`Season`；解析不出 → `E003`。
- 目标必须可序列化；`Episode` 必须能反查其所属季（供模块 2 取集列表）。
- 非 bilibili 域名、非 https、`bilibili.com.evil.example` 之类继续拒绝。

### 3. 解析结果契约

顶层结果新增：

- `source`：`"ugc" | "bangumi"`，必填。
- `container`：容器信息 `{ kind, id, title, unitCount }`，可空（旧形态可空）。`kind` ∈ `video | season`。
- `bvid`/`aid`：改为可空；季级结果为空。
- `canonicalUrl`：稿件为 `https://www.bilibili.com/video/{bvid}`；单集为 `https://www.bilibili.com/bangumi/play/ep{ep_id}`；季为 `https://www.bilibili.com/bangumi/play/ss{season_id}`。

单元列表 `parts` 语义扩展（仍为“可下载单元的扁平列表”，保持既有消费方可用）：

| 字段 | 类型 | 语义 |
| --- | --- | --- |
| `cid` | number | 该单元流 id |
| `bvid` | string | 该单元所属稿件的 bvid（番剧集也有，X3） |
| `epId` | number \| null | 番剧单集 id；UGC 为 null |
| `page` | number | **容器内** P 序号，从 1 开始 |
| `partCount` | number | **容器内** 可下载 P 总数（≥1） |
| `groupIndex` | number | 容器在结果中的 1 起序号，用于命名与排序 |
| `title` | string | 单元标题（P 标题或集标题） |
| `durationSeconds` | number | 秒，已归一（模块 2 负责换算） |
| `availability` | enum | `playable` 或 `unavailable` |
| `unavailableReason` | enum \| null | `login-required`/`vip-required`/`paid`/`region-locked`/`download-disabled`/`preview`/`unknown` |

不变式：

- `1 ≤ page ≤ partCount`。
- 同一容器内 `cid` 唯一；全局 `(bvid, cid)` 唯一。
- `availability = unavailable` 时 `unavailableReason` 非空；反之必须为 null。
- `parts` 为空时解析视为失败（`E004`），不得返回空成功。

### 4. 任务契约

草稿与任务新增字段：

| 字段 | 类型 | 默认 | 语义 |
| --- | --- | --- | --- |
| `source` | `"ugc" \| "bangumi"` | `"ugc"` | 任务来源 |
| `epId` | number \| null | null | 番剧单集 id |
| `groupIndex` | number | 1 | 容器序号，命名用 |
| `partCount` | number | 1 | 容器内 P 总数 |

既有字段语义收窄但不反转：

- `bvid` = 该单元自己的 bvid（不再允许“顶层 bvid + 他人 cid”）。
- `page` = 容器内 P 序号。
- `partTitle` = 单元标题。
- 命名所需“序号”取值在本模块只保证字段可得，具体格式由模块 3 决定。

兼容：

- 反序列化缺省：`source="ugc"`、`epId=null`、`groupIndex=1`、`partCount=1`。
- 旧任务列表、旧草稿、旧夹具不得因新字段失败。
- 任务 id 生成规则不变（`{request_id}:{index}`）。

### 5. 校验

- 草稿校验保留 `page ≥ 1`、`page ≤ partCount`、`partCount ≥ 1`、标题非空、媒体字段匹配模式。
- 新增：`source="bangumi"` 时 `epId` 必须存在；`source="ugc"` 时 `epId` 必须为 null。
- 校验失败仍为 `E004`（草稿非法用内部错误语义，沿用现状）。

## 专家前端工程约束

- 输入校验是纯函数，位于 `input.ts`，不得依赖 DOM/时间/随机。
- 判定结果类型用 discriminated union（`{ok:true,value}` / `{ok:false,reason}`），不新增布尔旗标堆叠。
- store 只消费 DTO；不得在 store 里二次推导来源类型。
- 异步：解析仍由现有 token 机制防竞态，本模块不新增请求。
- 可测试性：输入矩阵与 DTO 反序列化必须可单测，无需网络。

## 前端架构设计路由输入

`architecture-design_required: true`。不得在 architecture-design 改变的行为合同：输入形态集合、`page`/`partCount` 语义、`bvid` 可空、`availability` 取值与不变式、错误码语义、兼容默认值。

需要 architecture-design 决定：标识类型的归属与命名、UGC 与 PGC 两条归一化路径是否共享结构、DTO 扩展的落点（Rust models 与 TS 契约如何同源）、任务字段扩展对下载链路的影响边界、既有 `VideoPart` 消费方的迁移策略、测试夹具的组织方式。

## 生产代码质量约束

- 先类型后实现：Rust 侧新增标识与单元类型；TS 侧同步接口；禁止 `any`/`interface{}` 兜底。
- fail-fast：不认识的目标类型直接 `E003`，不返回半成品。
- 严格空值：`Option`/`null` 语义明确；不得用空字符串代替 null。
- 纯函数优先：URL 判定、ID 解析、校验均为纯函数。
- 命名：布尔 `is/has/can`，处理函数 `parse/normalize/validate`。
- 无魔法值：`ep`/`ss` 前缀、路径片段提为命名常量并就近归属。
- 边界 UI 状态：本模块不加 UI 状态。

## 人工评审就绪约束

- 允许改动：`src/features/download-center/input.ts`、`src/features/download-center/contracts.ts`、`src/features/download-center/store.ts`（仅草稿构造）、`src-tauri/src/infrastructure/bilibili/input.rs`、`src-tauri/src/infrastructure/bilibili/adapter.rs`（仅 parts 映射字段）、`src-tauri/src/models/parse.rs`、`src-tauri/src/models/task.rs`、`src-tauri/src/services/tasks/manager.rs`（字段透传）、`src-tauri/src/services/tasks/validation.rs`、`src-tauri/src/services/parser.rs`（仅剧集目标的分支占位）、相关测试与夹具。
- 不在范围：`raw.rs`/`adapter.rs`/`client.rs`（模块 2）、`parser.rs` 取流分支（模块 3）、页面组件（模块 4）。
- review-sensitive：DTO 字段扩散会触碰任务持久化与既有测试，必须逐个调用方核对。
- 证据：`cargo test`、`pnpm test`、`pnpm build` 输出；未运行项需说明原因。
- 清理：无临时字段、无重复常量、无过期注释、无未使用导入。

## 功能式约束

- 纯函数：`normalizeParseInput`/`parse_video_id`/草稿字段派生。
- 不可变：DTO、store 快照、props 不原地修改。
- 副作用边界：仅 IPC 调用与 store action。
- 归一位置：单位换算、可用性判定在 Rust adapter（模块 2/3），不在前端 computed。
- 不引入函数式工具库。

## 架构复用与共享所有权约束

- 已发现等价逻辑：前端 `input.ts` 与 Rust `input.rs` 各自实现“最后一段是 BV/av”的判定，语义重复但跨语言，属于必要镜像。
- 结论：语言内不重复；跨语言允许镜像，但必须共用同一份用例矩阵（同一批输入期望同一结果）。
- 若模块 2/3 需要同样的“目标类型”判断，必须复用本模块导出的类型与谓词，不得复制。
- 单调用方 helper 默认就近放置。

## 就近与文件抽取约束

- 新增 `parse_bangumi_target` 类解析函数：归属 `input.rs`，不新建模块文件。
- 前端在 `input.ts` 内扩展 `isSupportedUrl`，不新增 `url.ts`。
- 若出现第二个真实调用方（例如模块 2 需要从 URL 取 ep_id），再评估提取。

## Vue 组件抽取约束

不适用：本模块不新增或拆分 SFC。

## 设计约束

- 不引入 trait/抽象层来描述“来源”；用枚举 + 穷尽 match。
- 不引入通用 `Repository`/`Manager` 概念。
- DTO 字段命名沿用 camelCase 序列化，Rust 字段 snake_case。
- 契约以 Rust 为单一权威，TS 类型为镜像。

## 脚手架决策

不适用：既有工程内增量改造。

## 变化轴与模式决策

- 变化轴：目标类型（video/season/episode）× 单元类型（UGC P / 番剧集）。
- 触发信号：按类型分派（selection by type）、契约形状适配（adapter 语义）、树形结构（容器→单元）。
- 决策：Level 0 `direct code` + 一个类型化枚举；拒绝 Strategy/Factory 层。
- 理由：分派点只有两处（输入归一、适配产出），且分支逻辑短；引入策略对象会增加跨语言契约负担。
- 前端语法形态：可辨识联合 + 穷尽 `switch`；Rust 用 `enum` + `match`。
- 复审信号：若出现第三种来源（如课程/直播）或来源专属参数超过两个，重新评估。

## 代码上下文与影响假设

- 前端：`input.ts`（校验）、`contracts.ts`（DTO）、`store.ts`（草稿构造）、`DownloadPage`/`VideoSummary`/`PartSelector`（消费，本模块只保证类型可编译）。
- Rust：`input.rs`（VideoId/NormalizedInput）、`models/parse.rs`、`models/task.rs`、`commands/mod.rs`、任务管理反序列化、下载源解析（字段消费，模块 3 改行为）。
- 测试：`input.test.ts`、`store.test.ts`、`service.test.ts`、`input.rs` 单测、`adapter.rs` fixture 测试（模块 2 更新）。
- 假设：任务持久化使用 serde 默认值即可兼容；如使用自定义存储需在 plan 确认。

## API 与数据契约

- 命令：`parse_video(input: string)` 入参不变。
- 返回：`ParseVideoResult` 扩展字段（见上），新增字段必须可缺省反序列化。
- 任务命令：`create_download_tasks` 入参 `drafts[]` 扩展字段，旧草稿缺省可用。
- 契约来源：Rust 结构体为权威；TS 接口镜像，无生成器。
- 错误：不新增错误码；`E003` 输入非法、`E004` 目标不存在/无可用单元。

## 前端 / 后端责任边界

- Rust：权威归一、类型、默认值、校验。
- 前端：即时校验提示、DTO 消费、草稿字段填充。
- 共享：字段名、可空性、默认值、不变式。
- 外部接口待补：无（本模块不取数）。
- 前端不得自行把“不可识别”降级成“默认 UGC”。

## 上下文与依赖来源

`input.ts`、`input.rs`、`models/parse.rs`、`models/task.rs`、`contracts.ts`、`store.ts`、`services/tasks/validation.rs`、`services/tasks/manager.rs`、`services/tasks/filename.rs`、`analysis`、`prd-snapshot`。

## 边界情况

- 空串、纯空格、含换行 → 拒绝。
- 超长 URL（> 2048 字符）→ 拒绝（新增显式上限，属技术选择）。
- `ep0`、`ss0`、`epabc`、`ep` 无数字 → `E003`。
- `b23.tv` 短链解析到 HTML 而非目标 → `E003`。
- 番剧链接带 `?p=2` → 忽略 p。
- 稿件链接 `?p=` 超出 `partCount` → `E003`（沿用现状语义）。
- 结果 `parts` 为空 → `E004`。
- 旧任务缺 `source`/`epId` → 默认 `ugc`/null。
- 重复 `cid` → 适配层去重或失败，不得产生两个同 id 单元。

## 验收标准

- AC-1：前后端对 12 条输入矩阵（BV/av/视频页/移动页/b23/合集 BV/ep/ss/b23ep/非法域名/http/空串）给出同构判定，单测覆盖。
- AC-2：`ep733316` / `ss…` 链接通过输入层校验（不再 `E003`），被归一化为 `Episode`/`Season` 目标；在数据获取能力接入前（模块 2 之前）返回 `E004`，且错误信息可区分“输入非法”与“暂不支持该内容类型”。
- AC-3：单元满足 `1 ≤ page ≤ partCount`，同容器 `cid` 唯一。
- AC-4：契约层固定 `availability` 与 `unavailableReason` 的可空关系（`unavailable` 必带原因、`playable` 必须为空）；运行时产出由模块 2 适配层交付，禁用交互由模块 4 交付，本模块只保证契约与既有 UGC 路径全部为 `playable`。
- AC-5：旧任务 JSON 反序列化成功，`source="ugc"`、`epId=null`。
- AC-6：番剧草稿缺 `epId` 时校验失败；UGC 草稿带 `epId` 时校验失败。
- AC-7：既有 UGC 解析、多 P 选择、草稿字段与任务创建行为不回归。
- AC-8：`cargo test`、`pnpm test`、`pnpm build` 通过。
- AC-9：UGC 适配对单元填充 `bvid = view.bvid`、`epId = null`、`groupIndex = 1`、`partCount = pages.len()`、`availability = playable`；既有分 P 的 `page` 语义不变，回归用例覆盖。

## 人工评审与交接

- 评审重点：契约字段扩散是否最小；`page`/`partCount` 语义是否被所有调用方理解；两端用例矩阵是否同源。
- 交接：模块 2 消费 `EpisodeUnit`/`container`；模块 3 消费 `source`/`epId`/`bvid`/`page`；模块 4 消费 `availability`。

## 风险

| 风险 | 触发 | 影响 | 处理 |
| --- | --- | --- | --- |
| 语义漂移 | 调用方仍把 `page` 当全局序号 | 命名/计数错误 | 类型注释 + 用例 + 模块 3 复核 |
| 契约扩散 | 一次性加过多字段 | 迁移成本 | 只加本模块必需字段 |
| 前端落后 | TS 接口未同步 | 编译失败 | `pnpm build` 门禁 |
| 旧任务不兼容 | serde 无 default | 恢复失败 | `#[serde(default)]` + 回归用例 |
| 短链误判 | b23 解析异常 | 误报 E003 | 显式失败路径与用例 |

## 修订记录

- v1：初始规格，framework 自动审批通过。
- v2（architecture-design 前置检查）：发现两处不可执行问题——(a) AC-2 要求 `ep` 解析成功，但数据获取属于模块 2，v1 自相矛盾；(b) v1 未允许 `adapter.rs` / `manager.rs` 的最小字段填充，导致新增 DTO 字段无人构造、工程不可构建。修正：AC-2 改为“识别成功、数据层接入前返回 `E004`”，新增 AC-9，允许文件补充 `adapter.rs`（仅 parts 映射）与 `manager.rs`（字段透传）。行为合同、范围边界与产品语义未扩大。
