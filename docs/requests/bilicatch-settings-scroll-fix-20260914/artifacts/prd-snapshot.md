# 下游问题快照

## Source Summary

用户要求修复设置页在窗口高度不足时无法滚动、设置项显示不全的问题，并明确同意采用根 Provider 高度链修复建议。

## Key Business Goals

- 确保桌面窗口缩小时仍能访问全部设置项。
- 保持当前应用壳层作为主滚动边界。

## Explicit Behavior Constraints

- 内容未超过可视高度时不应出现无意义滚动。
- 内容超过可视高度时 `.content-scroll` 应产生纵向滚动。
- `body` 继续保持 `overflow: hidden`，避免双滚动条。
- 不改变设置表单、设置持久化或页面响应式宽度规则。

## Displays And Interactions Extracted From Source

- 页面：设置页。
- 交互：通过鼠标滚轮、触控板或滚动条访问超出可视区域的设置项。

## Workflow And State Rules

- 初始状态：设置内容高度大于主内容可视高度。
- 预期状态：主内容容器的 `scrollHeight` 大于 `clientHeight`，且 `overflow-y` 可滚动。

## Relevant Modules Or Pages

- `src/app/App.vue`
- `src/styles/base.css`
- `src/styles/layout.css`

## Open Questions

- 无。
