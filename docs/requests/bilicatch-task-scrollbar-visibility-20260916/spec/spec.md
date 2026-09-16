# 工程规格：任务列表滚动条可见性

## 范围

- 为 `.content-scroll` 预留稳定滚动槽。
- 为 WebKit/Chromium 滚动条声明固定宽度、透明轨道、可辨识滑块和悬停状态。
- 保留现有高度链、`overflow: auto` 和唯一滚动容器。

## 验收标准

- `AC-01`：`.content-scroll` 保持 `overflow: auto` 并包含 `scrollbar-gutter: stable`。
- `AC-02`：Chromium/WebView2 存在宽度明确的 `::-webkit-scrollbar` 规则。
- `AC-03`：滑块使用主题令牌，在明暗主题下均可辨识，并有 hover 状态。
- `AC-04`：不向 `.task-list` 增加纵向滚动或固定高度。
- `AC-05`：聚焦测试、全量测试、类型检查和构建通过；真实浏览器中溢出容器可滚动且滚动条可见。

## 风险

- 稳定滚动槽会在内容区右侧预留少量空间；这是避免滚动条出现时布局横跳的预期行为。
- 样式作用于所有路由的主内容滚动区，这是应用壳层统一行为。
