# 澄清：合集/番剧元数据解析

## Q1：合集结果的 `source` 用 `ugc` 还是 `bangumi`？

- 合集成员是普通稿件（不是 PGC 内容），只是被组织进 `ugc_season`。
- 决定：`source="ugc"`，`container.kind="season"`；番剧才是 `source="bangumi"`。
- 影响：任务语义、模块 3 取流路径（合集仍走 UGC 取流）。
- 锚定：source-derived（X1）。

## Q2：番剧解析为什么没有清晰度/音频选项？

- 能力来自 playurl，而 PGC 取流属于模块 3；本模块不提前请求。
- 决定：返回空能力（空数组 + 0/false）；前端显示“暂无可用选项”，不可入队直到模块 3 合入。
- 影响：模块 3 的 spec 必须补能力字段。
- 锚定：confirmed-decision（模块边界）。

## Q3：为什么要新增 `requestedCid`？

- `ep` 链接必须默认选中用户点进来的那一集；`page` 在番剧里恒为 1，无法区分。
- 决定：`ParseVideoResult.requestedCid: number | null`；前端优先级 cid > page > 首个 playable。
- 影响：前端默认选中；任务草稿不受影响。
- 锚定：source-derived（S3 用户指令）。

## Q4：`badge` 判定是否可靠？

- 接口实测只出现 `""`、`限免`、`预告`；会员/付费文案未实测。
- 决定：含“会员/大会员”判 `vip-required`、含“付费”判 `paid`、`allow_download=0` 判 `download-disabled`；其余乐观判 `playable`，由模块 3 取流时复核并降级。
- 影响：可能低估限制，但不会伪造可下载；下载阶段有兜底。
- 锚定：external-interface-pending。

## Q5：合集的能力字段用谁的？

- 逐成员请求 playurl 会产生 N 次请求，且合集成员能力通常一致。
- 决定：用入口稿件的 playurl；下载时每个单元各自重新取流。
- 锚定：source-derived + 性能约束。

## Q6：`durationSeconds` 是否要显示整季总时长？

- 聚合总时长是新的展示语义，来源未定义。
- 决定：保持“单元自身时长”；顶层字段沿用入口/首集时长；聚合展示留给模块 4 的显式决策（当前不做）。
- 锚定：out-of-scope（聚合语义）。

## Q7：`ep` 对应集已下架怎么办？

- 决定：不报错；返回其余集，`requestedCid=null`，前端选第一个 playable。
- 锚定：source-derived（容错）。

## Q8：`ss` 与 `ep` 是否都返回整季？

- 决定：是，两者都基于 season 接口返回整季；差异只在默认选中与顶层 `bvid/aid`。
- 锚定：source-derived。
