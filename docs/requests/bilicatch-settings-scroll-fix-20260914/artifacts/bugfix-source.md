# 缺陷来源

## 来源信息

- source system：当前 Codex 对话中的直接用户缺陷报告
- project key / source scope：BiliCatch 本地工作区
- defect id：不适用，用户未提供外部缺陷编号
- defect title：设置页窗口缩小时没有滚动条且设置项显示不全

## 缺陷事实

- observed behavior：窗口高度缩小时，设置页底部内容被裁掉，主内容区域没有可用滚动条。
- expected behavior：设置内容超过可视高度时，主内容区域可以纵向滚动并访问全部设置项。
- reproduction clues：进入设置页后缩小窗口高度。
- affected module or page：设置页所在的全局应用壳层与根 Provider 高度链。
- screenshots / logs / comments：未提供截图；已通过源码追踪确认 `.content-scroll` 有 `overflow: auto`，但根 Provider 未传递确定高度，且 `body` 禁止滚动。

## Verbatim Defect Evidence Index

- `USER-DEFECT-01`："设置页面为什么没有滚动条，当窗口缩小之后，设置项没有显示完全"
- `USER-APPROVAL-01`："根据你的建议修改"

## Closed-World Fix Scope Allowlist

- source-backed：恢复设置页在小窗口下的纵向滚动能力。
- code-fact-backed：给应用根 Provider 添加稳定类和全高约束，保留 `.content-scroll` 的滚动所有权。
- out-of-scope：设置项业务行为、页面视觉重构、后端、IPC、路由和侧栏布局。

## Open Questions And Missing Context

- 无阻塞问题。用户已确认采用已说明的根 Provider 高度修复方案。
