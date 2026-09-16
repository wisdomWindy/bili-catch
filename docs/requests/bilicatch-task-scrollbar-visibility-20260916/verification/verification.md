# Verification

| 验收项 | 结果 | 证据 |
| --- | --- | --- |
| AC-01 | pass | `.content-scroll` 保留 `overflow: auto`，computed `scrollbar-gutter=stable` |
| AC-02 | pass | `::-webkit-scrollbar` 宽度为 `10px` |
| AC-03 | pass | 轨道、滑块、hover 使用现有主题令牌；暗色主题 computed color 为 `rgb(168, 173, 181) rgb(41, 44, 48)` |
| AC-04 | pass | `.task-list` 未修改，仍由应用壳层唯一滚动 |
| AC-05 | pass | 47 个测试文件、184 个测试通过；typecheck、build 通过 |

## 浏览器证据

- URL：`http://127.0.0.1:1420/#/tasks?demo=1`
- 视口：478x810，任务数 7。
- `.content-scroll`：`clientHeight=754`、`scrollHeight=1156`、`overflowY=auto`。
- `maxScrollTop=402`，实际滚动后 `scrollTop=402`；右侧滚动条可见。

## 已知警告

- Vite 报告现有主 JS chunk 为 515.33 kB，超过 500 kB 建议阈值；与本次 CSS 修复无关。
