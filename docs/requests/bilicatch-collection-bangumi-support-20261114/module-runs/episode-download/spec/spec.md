# 规格：取流与权限（episode-download）

## 交付单元

- request：`bilicatch-collection-bangumi-support-20261114`
- module：`episode-download`（3 / 4）
- 依据：需求分析 H1/S6/S7、X5/X6、模块 1 契约、模块 2 元数据。

## 背景与目标

模块 2 能列出合集/番剧单元，但番剧没有可下载能力，任务取流仍走 UGC 路径（`view.pages` 校验 + `/x/player/wbi/playurl`）。本模块让单元真正可下载：番剧走 PGC 取流、按权限矩阵拒绝不可下载内容、按单元标识校验归属、按 `{序号}_{标题}` 命名。

## 范围内

1. PGC 取流端口（`/pgc/player/web/playurl`）。
2. 番剧解析结果补齐能力字段（清晰度/编码/音频）。
3. 下载源解析按 `source` 分派：UGC 原路径、番剧 PGC 路径。
4. 权限矩阵：免费/限免可下；登录专享、会员、付费、地区限制、禁止下载拒绝并给对应错误码。
5. 任务归属校验：以单元自己的 `bvid`/`epId`/`cid` 重新解析校验。
6. 命名 `{序号}_{标题}`（合集/番剧），单视频不变。
7. `containerKind` 进入任务契约。

## 范围外

- 付费/DRM 内容获取、会员购买引导。
- 选集 UI 与文案（模块 4）。
- 收起/展开、批量入口。
- 改变模块 1/2 的字段语义。

## 触发

任务执行到取流阶段（`VideoSourcePort::resolve` / `AudioSourcePort::resolve`），或解析番剧后需要能力字段。

## 来源锚定

| 项 | 标签 | 来源 |
| --- | --- | --- |
| 番剧走 PGC 取流 | source-derived（X5/X6 实测差异） | X5、X6 |
| 仅免费/限免可下载 | confirmed-decision（H1） | analysis H1 |
| 登录只提升清晰度 | source-backed（S6） | S6 |
| `allow_download=0` 拒绝 | source-derived（X3） | X3 |
| 命名 `{序号}_{标题}` | source-backed（S7） | S7 |
| 下载前按单元校验 | code-fact-backed（C12 缺陷） | C12 |
| `containerKind` 字段 | source-derived（模块 1 契约的窄扩展） | 模块 1 |
| 错误码复用 E005/E006 | source-derived（既有语义） | C16 |

## 工作流效率档位

S3 broad / full。

## 自愈完成门槛

权限矩阵、取流路径、归属校验、命名、能力字段、无回归；四类命令通过；清理模块 2 登记的“能力为空”状态。

## 用户意图合同

- 字面：番剧/合集能下载。
- 实际：免费集能下、受限集在下载前就说清不能下、不会下错集。
- 禁止：用 UGC 接口取番剧流；把 `epId` 丢掉只传 bvid；把受限集留到下载中失败。
- 验收：免费集产出文件、受限集错误码正确、命名含序号。

## 行为拆解

### 1. 番剧能力字段（解析阶段）

- 代表集：`ep` 输入用该集；`ss` 用第一个 `playable` 单元。
- 请求 `fetch_bangumi_playurl(ep_id, cid)`；成功则用其 dash 生成清晰度/编码/音频能力。
- 失败或无可播放流：能力保持为空，不使整次解析失败（列表仍可展示）。
- 不可下载单元不参与代表集选择（若全部不可下载，能力为空）。

### 2. 取流分派

- `source=ugc`：保持现有逻辑（`view` + `pages` 校验 + `x/player/wbi/playurl`）。
- `source=bangumi`：必须携带 `epId`；先按 `epId` 取 season，校验 `ep_id` 与 `cid` 同时匹配；命中后用该集可用性判定；通过后请求 PGC playurl。
- `epId` 缺失：`E003`（任务数据非法）。
- `ep_id`/`cid` 不匹配：`E004`（该集不可用）。

### 3. 权限矩阵

| 情况 | 判定 | 错误码 |
| --- | --- | --- |
| `badge` 含会员/大会员 | vip-required | E006 |
| `badge` 含付费 | paid | E006 |
| `allow_download=0` | download-disabled | E006 |
| 地区限制（实测前） | 按 E006 兜底 | E006 |
| PGC 接口返回 -403/-10403 | 权限不足 | E006 |
| 需要登录但当前匿名 | 由 playurl 失败或 badge 判定 | E005 |
| 其他不可用 | E004 | E004 |

### 4. 命名

- `containerKind="season"`：`{序号}_{标题}.{ext}`，序号 = `groupIndex`（1 起），标题 = 单元标题；音频/视频同规则。
- `containerKind="video"`：保持现状（单 P = 标题；多 P 音频 = `{page}_{part}`，视频 = 标题）。
- 标题清洗、Windows 保留字、200 字符截断沿用现有实现。

### 5. 任务契约

- `containerKind: "video" | "season"`，默认 `video`。
- `DownloadTask` 与草稿同步；旧任务默认 `video`。
- 前端 `buildDrafts` 取 `result.container?.kind ?? "video"`。

## 前端 / 后端职责

- Rust：取流、权限判定、命名、校验。
- 前端：传 `containerKind`；展示后端错误文案（模块 4 细化）。
- 外部接口待补：PGC 错误码全集、地区限制表现。

## 边界

- 番剧 `epId` 存在但 season 无该集 → `E004`。
- PGC playurl 返回 null dash → `E004`。
- 匿名选择 >480P → 既有 `E003` 不变。
- 登录态变化后已入队任务：按当前登录态重新取流，失败给权限错误，不回退到其它集。
- 合集成员仍走 UGC 取流（其 `source=ugc`）。

## 验收标准

- AC-1：番剧解析返回非空 `qualities`/`videoVariants`（代表集可取流时）。
- AC-2：番剧下载走 PGC 端口；UGC 不经过 PGC。
- AC-3：会员/付费/禁止下载集在取流前被拒绝，错误码 E006；`epId` 缺失 E003；集不匹配 E004。
- AC-4：`containerKind=season` 命名 `{序号}_{标题}`；`video` 保持既有命名。
- AC-5：旧任务缺 `containerKind` 反序列化为 `video`。
- AC-6：既有 UGC 取流、命名、任务用例不回归。
- AC-7：`cargo test`、`pnpm typecheck`、`pnpm test`、`cargo fmt/clippy` 通过。

## 风险

| 风险 | 处理 |
| --- | --- |
| PGC 接口字段/错误码不稳定 | 只映射已确认码；其余 E004 |
| 代表集取流失败导致解析失败 | 降级为空能力，不失败 |
| 命名改变影响既有用户文件 | 仅 season 生效，video 不变 |
