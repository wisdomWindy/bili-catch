# 澄清记录：输入与单元契约

## Q1：`/bangumi/play/ss…` 是否属于本请求范围？

- 回答：属于。用户目标是“番剧链接能展开为集列表”；番剧的季入口就是 `ss`，若只支持 `ep`，粘 `ss` 链接仍会被判 `E003`，与目标冲突。
- 最终决定：支持 `ep` 与 `ss` 两种番剧入口。
- 影响区域：输入形态矩阵、规范化目标标识、`canonicalUrl`。
- 锚定：source-derived（用户指令 S1 的窄推导）；需人工复核但可作为已定实现口径。

## Q2：番剧链接上的 `?p=` 如何处理？

- 回答：`p` 是 UGC 分 P 概念，番剧集不使用该参数；若强行套用会选中不存在的 P。
- 最终决定：番剧链接忽略 `p`；稿件链接的 `p` 语义不变（正整数、≤ `partCount`）。
- 影响区域：`requested_page` 的产生条件、默认选中。
- 锚定：confirmed-decision（analysis Q8 默认口径）。

## Q3：`unavailableReason` 是否需要区分“会员”和“付费”？

- 回答：接口实测只稳定给出 `need_login`、`need_vip`、`is_preview`、`badge`、`rights.allow_download`、`area_limit`；“付费”与“会员”的稳定区分字段未确认。若强行区分会产生不可验证的枚举值。
- 最终决定：枚举保留 `vip-required` 与 `paid`，但允许映射层在无法区分时归为 `unknown`；用户可见文案对 `paid`/`unknown` 都表达“当前版本不支持该集”。
- 影响区域：可用性枚举、错误文案（模块 3/4）。
- 锚定：source-derived + 外部接口待补。

## Q4：结果顶层 `bvid`/`aid` 是否允许为空？

- 回答：`ep` 有 `bvid`/`aid`，`ss` 季级没有单一 `bvid`。若保持必填，只能用首集冒充，属于伪造。
- 最终决定：两者改为可空（`Option`/`null`）；展示层在为空时隐藏对应字段（模块 4）。
- 影响区域：`ParseVideoResult`、既有展示与测试。
- 锚定：source-derived（X3）+ code-fact-backed（序列化）。

## Q5：`partCount` 语义从“全稿件分 P 数”改为“容器内 P 数”是否破坏既有行为？

- 回答：单视频/多 P 场景下容器就是该稿件，两者相等，行为不变；合集/番剧场景下才出现差异。
- 最终决定：`partCount` 定义为容器内 P 数；草稿与任务沿用该字段，校验 `1 ≤ page ≤ partCount` 不变。
- 影响区域：草稿构造、任务校验、命名。
- 锚定：confirmed-decision（H4）+ code-fact-backed（C14、validation.rs）。

## Q6：是否需要限制输入 URL 长度？

- 回答：需要，避免超长字符串进入解析与日志。2048 是技术选择，可逆、不影响任何已批准语义。
- 最终决定：超过 2048 字符直接 `E003`。
- 影响区域：输入校验。
- 锚定：框架级技术假设（reversible, non-user-visible）。

## Q7：旧任务与旧草稿如何兼容？

- 回答：任务记录已在用户磁盘上，不能被新字段打断。
- 最终决定：新增字段全部带默认值（`source="ugc"`、`epId=null`、`groupIndex=1`、`partCount=1`）；反序列化失败视为数据损坏，沿用现有错误处理。
- 影响区域：任务反序列化、草稿校验、回归测试。
- 锚定：code-fact-backed。

## Q8：前端与 Rust 判定不一致时以谁为准？

- 回答：Rust 是权威；前端只做即时反馈，避免无意义请求。
- 最终决定：前端放行而后端拒绝时展示后端错误；前端拒绝时不请求后端。两侧用例矩阵共享同一批输入期望。
- 影响区域：`input.ts`、`input.rs`、错误展示。
- 锚定：code-fact-backed + 既有分层。

## Q9：解析结果 `parts` 为空时的语义？

- 回答：空列表不是“可下载的空合集”，而是解析失败。
- 最终决定：`parts` 为空 → `E004`，不得返回成功。
- 影响区域：解析结果校验、错误码。
- 锚定：code-fact-backed（现有 `select_part` 行为）+ 产品不变量 9。

## Q10：为什么模块 1 允许改 `adapter.rs` 与 `manager.rs`？

- 回答：DTO 新增字段必须有构造点与透传点，否则工程无法编译，也无法回归旧行为。把这两个文件纳入是“可构建性”要求，不是把模块 2/3 的工作提前。
- 边界：`adapter.rs` 只允许把现有 `ViewData` 映射成新单元形状（`bvid`、`partCount`、`groupIndex`、`availability=playable`、`epId=null`），不得新增请求或解析合集/番剧字段；`manager.rs` 只允许把草稿新字段透传到任务记录。
- 最终决定：纳入并限制边界；模块 2 在这两个文件上继续扩展。
- 影响区域：允许改动清单、AC-9。
- 锚定：code-fact-backed（编译闭合）。
