# 代码上下文

- 高度链：`#app -> .app-provider -> .app-shell -> .workspace -> .content-scroll`。
- `.content-scroll` 已有 `min-height: 0` 与 `overflow: auto`，实测 478x810 视口下 `clientHeight=754`、`scrollHeight=1156`、最大滚动距离 `402`。
- 浏览器滚动后 `scrollTop=402`，证明滚动边界正确；剩余问题是 WebView2/系统覆盖式滚动条的常驻可见性。
- 正确所有者是 `src/styles/layout.css`，不应把 `overflow-y` 下放到 `.task-list`。
