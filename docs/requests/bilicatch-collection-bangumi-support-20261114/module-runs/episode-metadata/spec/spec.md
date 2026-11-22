# 规格：合集/番剧元数据解析（episode-metadata）

## 交付单元

- request：`bilicatch-collection-bangumi-support-20261114`
- module：`episode-metadata`（顺序 2 / 4）
- 依据：需求分析（S4/S9/S10、X1–X4、H1/H2/H4）、`requirements/modules/episode-metadata.md`、模块 1 已交付契约。

## 背景与目标

模块 1 已让单元契约能表达多容器、集级标识与可用性，但解析仍只读 `view.pages`，番剧目标返回 `E004`。本模块接入两类数据源，把“合集成员/番剧集”转成统一单元列表：合集读 `view.ugc_season`（无需额外请求），番剧读 `/pgc/view/web/season`。目标是让一次解析返回完整可见单元，并让用户粘 `ep` 链接时默认选中该集。

## 范围内

1. `ViewData` 扩展 `ugc_season`，把合集成员展开为单元（支持成员内多 P）。
2. 新增 PGC 季数据端口与适配（`ep_id` / `season_id` 两种入参）。
3. 时长归一（PGC 毫秒 → 秒）。
4. 番剧 section 过滤（只保留正片 `section_type=0`）。
5. 可用性初判（`badge`、`allow_download`、`section_type`）。
6. `requestedCid`：合集入口与 `ep` 输入默认选中对应单元（修复前合集无 `?p=` 时为 null，会落到 EP1）。
7. 缺失/异常字段容错。

## 范围外

- PGC 取流与权限二次校验（模块 3）。
- 选集 UI、置灰、文案（模块 4）。
- 系列（series）、收藏夹、直播、课程。
- 付费/DRM 内容获取。
- 变更模块 1 的契约字段语义。

## 触发与开始条件

用户在下载中心提交：合集内 BV 链接（`ugc_season` 存在）、`/bangumi/play/ep…`、`/bangumi/play/ss…`。

## 需求拆分承接

allowlist P2（集列表数据）、P8（分 P 层级数据）、H2/H4；`Q6` 默认只正片；`Q7` 失效成员容错；`H1` 仅免费/限免集。

## 来源锚定与可追溯性

| spec 项 | 标签 | 产品可见 | 来源 |
| --- | --- | --- | --- |
| 合集成员来自 `ugc_season.sections[].episodes[]` | source-derived（接口事实） | 是 | X1、X2 |
| 番剧集列表来自 `pgc/view/web/season` | source-derived | 是 | X3 |
| PGC `duration` 为毫秒需归一 | source-derived | 是 | X3 |
| 只保留正片 section | confirmed-decision（Q6 默认） | 是 | analysis Q6 |
| 仅免费/限免集可下载 | confirmed-decision（H1） | 是 | analysis H1 |
| `badge` 含“会员/付费/大会员”判不可用 | external-interface-pending | 是 | X3、H1 |
| `allow_download=0` 判不可下载 | source-derived | 是 | X3 |
| `ep` 输入默认选中该集 | source-derived（用户指令 S3） | 是 | S3 |
| 失效成员不补位、不整单失败 | external-interface-pending | 否 | Q7 |
| 系列/收藏夹 | out-of-scope | 否 | S5 |

## 工作流效率档位

`S3 broad`（延续请求级），full 密度；影响 Rust 端口、raw、adapter、parser 与前端默认选中。

## 自愈完成门槛

需求一致性、时长归一、section 过滤、可用性初判、容错、`requestedCid`、无回归；`cargo test`/`pnpm test`/`pnpm typecheck` 通过；无 debug/死代码；临时占位分支必须删除。

## 用户意图合同

- 字面：支持合集/番剧。
- 实际：粘一条链接就能看到该集合集，且默认落在用户点进来的那一集。
- 禁止：只返回当前一条视频；把季级 `ss` 当单集；把预告混进正片；毫秒当秒。
- 验收：集数、顺序、时长、默认选中、错误可区分。

## 用户流程

1. 输入归一（模块 1）→ 2. 解析：合集走 `view`，番剧走 `pgc season` → 3. 适配为容器 + 单元 → 4. 返回 DTO（含 `requestedCid`）→ 5. 前端默认选中。

## 页面与模块设计 / UI / 样式 / 展示语义

不适用：本模块不改页面；展示决策归模块 4。`durationSeconds` 语义固定为“单元自身时长”，不引入聚合总时长。

## 功能完整行为拆解

### 1. 合集解析

- 条件：`view.ugc_season` 存在且 `sections[].episodes[]` 非空。
- 顺序：按 `sections` 顺序，再按 `episodes` 顺序。
- 每个成员：`groupIndex` 从 1 递增；`bvid` 取成员 `bvid`；`epId = null`。
- 成员内 `pages[]` 非空：每个 P 一个单元，`page` = P 序号，`partCount` = `pages.len()`，`title` = `part`。
- 成员内 `pages[]` 为空：合成一个单元，`cid` = 成员 `cid`，`page=1`，`partCount=1`，`title` = 成员 `title`。
- 时长：优先 `pages[].duration`（秒）；缺省用 `arc.duration`（秒）。
- 结果：`source="ugc"`、`container.kind="season"`、`container.id=season_id`、`container.title=合集标题`、`title` = 合集标题、`cover` = 合集封面（缺失回退入口 `pic`）、`bvid/aid` = 入口稿件。
- 能力字段（清晰度/音频）：用入口稿件的 playurl 生成；不逐成员请求。
- `requestedCid`：入口稿件当前分 P 的 `cid`（带 `?p=` 取该 P，否则取 P1），保证用户粘贴的合集内视频默认被选中。

### 2. 番剧解析

- 入参：`ep_id` 或 `season_id`，均请求 `/pgc/view/web/season`。
- 过滤：`section_type != 0` 的条目直接丢弃（预告/花絮）。
- 顺序：接口顺序，`groupIndex` 从 1 递增。
- 每个集：`epId`、`bvid`、`cid`、`title` = `long_title` 非空则用之，否则 `title`；`page=1`、`partCount=1`。
- 时长：`duration`（毫秒）÷1000 后四舍五入（`(ms + 500) / 1000`）。
- 可用性：
  - `rights.allow_download == 0` → `unavailable` + `download-disabled`
  - `badge` 含“会员”或“大会员” → `unavailable` + `vip-required`
  - `badge` 含“付费” → `unavailable` + `paid`
  - 其余（含空、限免）→ `playable`，`unavailableReason=null`
- 结果：`source="bangumi"`、`container.kind="season"`、`container.id=season_id`、`container.title=季标题`、`title`=季标题、`cover`=季封面、`bvid/aid`：`ep` 输入取该集，`ss` 输入为 null。
- 能力字段：本模块不请求 PGC playurl，返回空能力（`qualities=[]`、`codecs=[]`、`videoVariants=[]`、`audioFormats=[]`、`audioBitrates=[]`、`audioCapability` 全空/false）。下载能力由模块 3 提供。
- `requestedCid`：`ep` 输入时 = 该集 `cid`；`ss` 输入为 null。

### 3. 默认选中

- `ParseVideoResult.requestedCid: number | null`（新增）。
- 前端 `applyResult` 优先选 `requestedCid` 命中的单元；其次 `requestedPage`；否则第一个 `playable` 单元。

### 4. 容错

- `sections` 为空或所有成员无有效 cid：`E004`（不得返回空成功）。
- 番剧过滤后无正片：`E004`。
- 单条成员字段缺失：跳过该条，不整单失败；计数以实际枚举为准。
- `duration` 缺失或为 0：保留 0，不猜测。

## 专家前端工程约束

- 前端只新增默认选中优先级，不新增展示；选择逻辑是纯函数式判断（先 cid、后 page、再首个 playable）。
- 不新增组件、不新增样式。

## 前端架构设计路由输入

`architecture-design_required: true`。不得改变：数据来源、过滤口径、可用性判定、时长单位、默认选中优先级。需架构决定：端口形状（两个方法 vs 一个带枚举参数）、raw 模型拆分、adapter 函数归属、能力字段为空时的表达。

## 生产代码质量约束

- 类型先行：`SeasonData`/`RawEpisode`/`RawUgcSeason` 显式建模；未知字段忽略。
- fail-fast：空集、无正片、无 cid 显式 `E004`。
- 严格空值：`Option` 表达缺失；不得用 0 冒充未知时长以外的语义。
- 纯函数：adapter 无副作用，输入不可变。
- 命名：`adapt_collection_units` / `adapt_bangumi_units`。
- 边界状态：能力为空由模块 4 处理，本模块不造默认选项。

## 人工评审就绪约束

允许改动：`raw.rs`、`client.rs`、`adapter.rs`、`parser.rs`、`mod.rs`、`contracts.ts`、`store.ts` 及相关测试/夹具。
不在范围：取流（`audio_source`/`video_source`/`media_url`）、页面组件、模块 1 已定字段。
review-sensitive：删除模块 1 的 `E004` 占位分支；新增 HTTP 请求；fixture 体积。
证据：`cargo test`、`pnpm test`、`pnpm typecheck`。

## 功能式约束

- 适配为纯函数：`adapt_collection_units(view) -> Vec<VideoPart>`、`adapt_bangumi_units(season) -> Vec<VideoPart>`。
- 不可变：解析入参只读。
- 副作用：仅 HTTP 请求（client）。
- 归一位置：adapter 内完成秒/毫秒、可用性。

## 架构复用与共享所有权约束

- 两个 adapter 共享“成员 → 单元”的构造规则（page/partCount/groupIndex/availability），但数据源不同；按 Anti-DRY，先各写一个纯函数，出现第三个来源再抽。
- 时长归一函数单点定义（`duration_ms_to_seconds`），两处可用时复用。
- 不引入 trait/策略层。

## 就近与文件抽取约束

- 新增解析函数放 `adapter.rs`；PGC raw 模型放 `raw.rs`；端口方法放 `client.rs`。
- 不新建文件。

## Vue 组件抽取约束

不适用：无 SFC 改动。

## 设计约束

- PGC 响应使用独立 envelope（`result` 而非 `data`），不复用 `ApiResponse`。
- 不改 `BilibiliPort` 已有方法签名；新增方法。
- 前端契约只加 `requestedCid` 一个字段。

## 脚手架决策

不适用。

## 变化轴与模式决策

- 信号：按来源分派（ugc/bangumi）、契约形状适配、两级结构。
- 决策：Level 0 direct code；两组独立纯函数 + 一个枚举分派；拒绝策略层与通用 mapper。
- 复审信号：出现第三个数据源（系列/收藏夹）时再抽。

## 代码上下文与影响假设

- 影响：`ViewData` 反序列化、`BilibiliPort` 实现、`ParserService::parse` 分支、`ParseVideoResult`、前端 store 默认选中、测试夹具。
- 假设：`ep` 与 `ss` 均可用同一个 season 接口；`episodes[]` 无内嵌 pages；缺失字段可跳过。

## API 与数据契约

| 接口 | 方法/路径 | 关键字段 |
| --- | --- | --- |
| 合集 | `GET /x/web-interface/view?bvid=` | `ugc_season{id,title,cover,sections[].episodes[].{bvid,cid,title,arc.duration,pages[]}}` |
| 番剧季 | `GET /pgc/view/web/season?ep_id=` / `?season_id=` | `result{season_id,title,cover,episodes[].{ep_id,bvid,cid,title,long_title,duration,badge,section_type,rights.allow_download}}` |
| 返回 DTO | `parse_video` | 新增 `requestedCid` |

错误：接口非 0 → `E004`；网络 → `E001`/`E002`。

## 前端 / 后端责任边界

- Rust：请求、反序列化、过滤、归一、可用性初判、`requestedCid`。
- 前端：默认选中优先级、DTO 消费。
- 外部接口待补：`badge` 文案与付费语义的稳定性；失效成员状态字段。
- 前端不得自行判定可用性。

## 上下文与依赖来源

`raw.rs`、`client.rs`、`adapter.rs`、`parser.rs`、`contracts.ts`、`store.ts`、`analysis`、`prd-snapshot` X1–X4。

## 边界情况

- 合集存在但 members 全失效 → `E004`。
- 合集成员多 P 且 `pages` 与 `page` 并存 → 以 `pages` 为准。
- 番剧 `episodes` 全为预告 → `E004`。
- `ss` 输入且 `season_id` 不匹配 → 以返回 `season_id` 为准。
- `ep` 输入但该集不在返回列表（下架）→ `requestedCid=null`，仍返回其余集。
- 时长毫秒缺失/0 → 0 秒。
- 同一个 `cid` 重复出现 → 保留首次，后续跳过。

## 验收标准

- AC-1：合集解析返回全部可见成员单元，`container.kind="season"`、`source="ugc"`，成员顺序与接口一致。
- AC-2：成员多 P 时逐个 P 成单元，`page/partCount` 正确；单 P 成员合成一个单元。
- AC-3：番剧解析返回正片单元，预告被过滤；`epId`/`bvid`/`cid` 与接口一致。
- AC-4：PGC 时长按 `(ms+500)/1000` 归一；UGC 秒不变。
- AC-5：`allow_download=0`、会员/付费 badge 判 `unavailable` 且原因对应；其余 `playable`。
- AC-6：合集入口（含带 `?p=`）与 `ep` 输入返回 `requestedCid` 且前端默认选中该单元；`ss` 输入为 null。
- AC-7：空集/无正片/无 cid 返回 `E004`，不返回空成功。
- AC-8：模块 1 的 `E004` 占位分支被删除；既有 UGC 分 P 解析不回归。
- AC-9：`cargo test`、`pnpm test`、`pnpm typecheck` 通过。

## 人工评审与交接

评审重点：过滤口径、可用性初判的乐观边界（下载时仍由模块 3 复核）、时长归一、默认选中优先级。
交接：模块 3 消费 `availability` 与 `epId`；模块 4 消费容器与单元结构。

## 风险

| 风险 | 影响 | 处理 |
| --- | --- | --- |
| badge 语义不稳定 | 误判可用性 | 初判 + 模块 3 取流时复核；记 external-pending |
| 合集成员大量多 P | 单元数暴涨 | 不做上限；由 UI 滚动与任务容量（100）兜底 |
| PGC 接口字段漂移 | 反序列化失败 | 全部字段 `#[serde(default)]` + 跳过坏数据 |
| 能力为空 | 番剧暂时不可入队 | 明确为模块 3 前状态，不伪造选项 |
