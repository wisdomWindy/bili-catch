# 来源快照：合集与番剧支持

## 逐字来源证据索引

> 归纳摘要只用于导航，不能成为独立授权来源。下游引用必须以本索引的原文或代码位置为准。

| ID | 来源 | 位置 | 原文 |
| --- | --- | --- | --- |
| S1 | 用户直接指令 | 2026-11-14 会话 | “「番剧/合集支持」做成一个正式 request” |
| S2 | 用户直接指令 | 2026-11-14 会话 | “这个地址明明是一个合集中的一个” |
| S3 | 用户直接指令 | 2026-11-14 会话 | “为什么不能获取 https://www.bilibili.com/bangumi/play/ep733316/ 这个地址的视频” |
| S4 | 本地 PRD | `biliCatch_PRD.md:32`（§1.3 范围表） | “合集/番剧批量下载” |
| S5 | 本地 PRD | `biliCatch_PRD.md:35`（§1.3 范围外） | “播放列表/收藏夹导入（V1.2 规划）” |
| S6 | 本地 PRD | `biliCatch_PRD.md:44`（§1.4） | “应用采用‘匿名可用，登录解锁高清/无损’的策略。匿名状态可解析视频、下载 ≤480P 视频和 ≤64K 音频；登录后可下载 1080P 及以上、4K/8K/HDR、HiRes 无损音频，并访问账号有权观看的受限内容。” |
| S7 | 本地 PRD | `biliCatch_PRD.md:318`（模块二业务规则） | “文件命名规则：`{视频标题}.{扩展名}`，合集视频为 `{序号}_{标题}.{扩展名}`。” |
| S8 | 既有生命周期工件 | `docs/requests/bilicatch-v1-desktop/artifacts/prd-snapshot.md:9` | “V1.0 范围：单视频/音频、合集/番剧批量、独立音频、扫码登录、队列与断点续传、系统集成、自动更新” |
| S9 | 既有需求分析 | `docs/requests/bilicatch-v1-desktop/analysis/requirement-analysis.md:151` | “合集/番剧 \| 是否与普通多 P 共享模型 \| 外部接口待补 \| 解析适配器规范化为统一 part 列表” |
| S10 | 既有需求分析 | `docs/requests/bilicatch-v1-desktop/analysis/requirement-analysis.md:165` | “合集/番剧的受限内容与账号权限依赖真实账号和线上环境，自动化测试需要契约夹具或授权测试账号。” |

## 代码事实（仅约束实现，不授权产品范围）

| ID | 位置 | 事实 |
| --- | --- | --- |
| C1 | `src/features/download-center/input.ts:5` | 前端 BVID 规则为 `/^BV[0-9A-Za-z]{10}$/`，AVID 为 `/^av[1-9][0-9]*$/i` |
| C2 | `src/features/download-center/input.ts:12` | `isSupportedUrl()` 要求路径最后一段匹配 BV/av，否则判定输入无效；`b23.tv` 仅要求路径非空 |
| C3 | `src-tauri/src/infrastructure/bilibili/input.rs:26` | `parse_video_id()` 只识别 BV（长度 12）与 av，`VideoId` 枚举只有 `Bvid`/`Aid`/`ShortUrl` |
| C4 | `src-tauri/src/infrastructure/bilibili/input.rs:119` | 后端同样取 URL 最后一段做 `parse_video_id`，失败即 `E003` |
| C5 | `src-tauri/src/infrastructure/bilibili/input.rs:124` | URL 的 `?p=` 被解析为 `requested_page`，仅支持正整数 |
| C6 | `src-tauri/src/infrastructure/bilibili/raw.rs:12` | `ViewData` 只声明 `pages: Vec<RawPage>`，未声明 `ugc_season`；未知字段被 serde 忽略 |
| C7 | `src-tauri/src/infrastructure/bilibili/raw.rs:29` | `RawPage` 只有 `cid/page/part/duration` |
| C8 | `src-tauri/src/infrastructure/bilibili/adapter.rs:123` | `parts` 完全由 `view.pages` 一对一映射，`canonical_url` 为 `https://www.bilibili.com/video/{bvid}` |
| C9 | `src-tauri/src/infrastructure/bilibili/client.rs:218` | 元数据只请求 `/x/web-interface/view`，参数为 `bvid` 或 `aid` |
| C10 | `src-tauri/src/infrastructure/bilibili/client.rs:271` | 取流只请求 `/x/player/wbi/playurl`，参数为 `cid` + `bvid`/`avid`，`fnval=4048` |
| C11 | `src-tauri/src/infrastructure/bilibili/raw.rs:39` | `PlayData` 只声明 `accept_quality/accept_description/v_voucher/dash`，无 PGC 字段 |
| C12 | `src-tauri/src/services/parser.rs:214` | 视频取流前用 `view.pages.iter().any(|p| p.cid == request.cid)` 校验，cid 不在当前稿件 `pages` 内即 `E004` |
| C13 | `src/features/download-center/contracts.ts:14` | 前端 `VideoPart` 只有 `cid/page/title/durationSeconds`，无 `bvid`、无集号、无来源类型 |
| C14 | `src/features/download-center/contracts.ts:71` | `DownloadTaskDraft` 以 `bvid` + `cid` + `page` 标识一个下载单元 |
| C15 | `src/features/download-center/store.ts:322` | `buildDrafts()` 给所有选中 part 统一写入 `result.bvid`，任务标识不随 part 变化 |
| C16 | `src-tauri/src/models/app_error.rs:5` | 现有错误码 E001–E017；`E003` 输入无效、`E004` 不可用、`E005` 需登录、`E006` 无权限 |

## 外部接口实测证据（2026-11-14，仅作为接口事实）

| ID | 接口 | 观测结果 |
| --- | --- | --- |
| X1 | `/x/web-interface/view?bvid=BV1kw8b6bEeL` | `videos=1`；`pages` 长度 1；`ugc_season.ep_count=7`，`sections[0].episodes[]` 含 7 条，各自 `bvid`/`cid` 不同 |
| X2 | 同上，`episodes[]` 单条 | 含 `bvid`、`cid`、`title`、`arc.duration`、`page.duration`、`attribute`、`section_type` |
| X3 | `/pgc/view/web/season?ep_id=733316` | `code=0`；`result.episodes[]` 每集含 `ep_id`、`bvid`、`cid`、`aid`、`long_title`、`badge`、`badge_type`、`duration`（毫秒，如 1203160）、`rights.allow_download`、`section_type` |
| X4 | 同上 | 季信息含 `season_id`、`title`、`cover`、`ep_count`、`sections[]`；`rights` 内含 `allow_download`/`area_limit`/`allow_dm` |
| X5 | `/pgc/player/web/playurl?ep_id=733316&cid=1022370693` | `code=0`；`support_formats[]` 标注 `need_login`（1080P/720P）、`need_vip`（4K/1080P+）；`is_preview=0`、`is_drm=false`；匿名可取得 480P/360P dash 流 |
| X6 | `/x/player/playurl?bvid=BV1vT411d7QE&cid=1022370693` | 对该限免集同样 `code=0`，但 dash 仅含 480P/360P 码流；`accept_quality` 仍列出到 4K |
| X7 | `b23.tv/ep733316` | 短链重定向到 `https://www.bilibili.com/bangumi/play/ep733316` |

## 来源摘要

PRD 在 V1.0 范围内明确写入“合集/番剧批量下载”（S4/S8），但 `bilicatch-v1-desktop` 的解析模块只交付了 UGC 分 P（C6–C8）。番剧受限内容的账号权限被列为需要真实账号与契约夹具的风险项（S10）。本次请求由用户直接指令（S1–S3）发起，目标是把这两类内容纳入既有“解析 → 选集 → 批量入队 → 下载”链路。

## 关键业务目标

1. 合集内任一视频链接都能展开为整个合集的可见集。
2. 番剧 ep/ss 链接能展开为整季集列表。
3. 选集与批量入队沿用现有交互，不引入第二套下载模型。
4. 权限不足与不可下载以明确错误呈现，而不是失败在下载阶段。

## greenfield scaffold sensitivity

不适用：本请求是在既有 Tauri + Vue 3 + TypeScript 工程内增加解析能力，不创建项目、应用、包或新的前端业务面。

## 显式行为约束（来自来源）

- 范围内包含“合集/番剧批量下载”，因此合集与番剧都必须支持“多集选择 + 批量入队”（S4、S8）。
- 范围外明确包含“播放列表/收藏夹导入（V1.2 规划）”，本次不得顺手实现收藏夹/播放列表批量（S5）。
- 匿名可解析、≤480P；登录解锁更高清晰度与“账号有权观看的受限内容”（S6）。因此番剧的权限分层必须可表达，但匿名能力边界不变。
- 合集视频的下载文件名为 `{序号}_{标题}.{扩展名}`，单视频为 `{视频标题}.{扩展名}`（S7）。
- 既有分析已把“合集/番剧”标记为 `外部接口待补`，并声明“解析适配器规范化为统一 part 列表”（S9），本请求延续该口径，但“统一列表”是否等于“同一控件/同一模型”仍需人工确认。

## 封闭产品范围允许清单（intake 初版，待 requirement-analysis 固化）

| ID | 类别 | 授权来源 | 授权内容与边界 | 未授权邻近做法 |
| --- | --- | --- | --- | --- |
| A1 | 输入识别 | S1–S3 | 识别合集内 BV 链接、番剧 `ep`/`ss` 链接、对应 `b23.tv` 短链 | 不扩大到收藏夹、播放列表、UP 主投稿批量 |
| A2 | 解析结果 | S4 | 解析结果包含合集的可见集列表 | 不展示合集简介、UP 主其他合集、追番状态 |
| A3 | 解析结果 | S4 | 解析结果包含番剧整季的集列表 | 不展示评分、弹幕、字幕、追番/订阅入口 |
| A4 | 选集与批量 | S4、S8 | 多选、全选、反选，按选中集各生成一个任务 | 不默认自动下载整季，不新增“订阅更新” |
| A5 | 权限提示 | S6 | 对需登录、需大会员、付费、地区限制、禁止下载的集给出明确结果 | 不实现付费获取、不实现 DRM 绕过 |
| A6 | 文件命名 | S7 | 合集/番剧按 `{序号}_{标题}.{扩展名}` | 不改动单视频命名规则 |
| A7 | 兼容约束 | C6–C15 | 现有 UGC 分 P 的解析、选择、任务契约保持不变 | 不以“统一模型”为由重写现有 UGC 链路 |

## 从来源提取的表单、表格、展示与交互

- 输入区：一个链接/编号输入框 + 解析动作（现有实现，来源为既有解析模块规格）。
- 视频身份区：标题、UP 主、封面、时长、BV 号（现有实现；合集/番剧需补充季/合集标题与集列表，来源 S4）。
- 选集区：checkbox 列表 + 全选/反选 + 已选计数（现有实现；S4 要求扩展到合集与番剧集）。
- 下载设置区：模式、清晰度、编码、音频格式与码率、保存位置（现有实现，不在本请求改动范围）。
- 错误区：解析失败、权限不足、集不可用（S6 提供分级依据，具体文案与错误码映射尚未在来源中给出）。

## 从来源提取的流程与状态规则

1. 粘贴链接 → 输入校验 → 解析 → 展示身份与选集 → 选择 → 生成草稿 → 入队（现有主流程）。
2. 输入校验必须放行合集内 BV 与番剧 ep/ss；非法输入仍为 `E003`（C1–C4）。
3. 解析成功但某集不可播放时，不得整单失败；该集应被标记为不可选并给出原因（由 S6 权限分级推出，需在 spec 明确）。
4. 下载阶段再次校验集归属；校验失败不得静默下载错误内容（C12 现状：以 `pages` 校验 cid）。

## 相关模块或页面

- 页面：下载中心（`/download`）。
- 前端：`src/features/download-center/`（输入、store、选择器、草稿）。
- 后端：`src-tauri/src/infrastructure/bilibili/`、`src-tauri/src/services/parser.rs`、任务校验链路。

## 上游开放问题

1. 番剧支持是否覆盖“需登录可见集”“大会员集”“付费集”“DRM 集”？（PRD 只说范围内包含番剧批量，未给权限层级口径）
2. 合集/番剧集与现有分 P 是否共用同一选集模型和同一 UI 区块？（S9 只说“统一 part 列表”，未说控件形态）
3. “批量”是否指“多选手动勾选”，还是必须提供“整季/整合集一键加入”？（S4 用词为“批量下载”）
4. 合集/番剧的“序号”取接口返回的页码/集号，还是列表中的位置序号？（S7 只写“序号”）
5. 合集内某一集自身包含多个分 P 时，选集粒度是“集”还是“集的某个 P”？（来源未涉及，代码事实显示两层标识同时存在）
