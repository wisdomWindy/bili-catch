# 代码上下文

## Context Requirement

- 需要理解 `#app -> NConfigProvider -> .app-shell -> .workspace -> .content-scroll` 的高度传递关系。
- 首次需要阶段：bugfix intake。
- 代码图仅为可选；该问题的相关文件与依赖链已知且范围很小。

## Graph Availability Check

- repository graph status：missing。
- detection method：在仓库内搜索 graph 配置或产物，未发现可用代码图。
- tool or runtime found：无。
- check context：2026-09-14，intake iteration 1。

## Installation Or Bootstrap Record

- attempted：no。
- method：不适用。
- result：not_needed。
- output summary：代码图政策允许对隔离样式和明确单文件行为使用轻量搜索；本次依赖链可直接完整追踪。
- next step：使用仓库文本搜索与依赖源码核对。

## Fallback Record

- fallback used：yes。
- fallback method：`rg` 搜索、完整读取 Vue/CSS 入口与 Naive UI `ConfigProvider` 渲染实现。
- why fallback was needed：仓库无代码图，且局部问题不值得引入新工具。
- residual confidence：高；高度链和裁剪边界已定位，无未解析调用链。

## Relevant Entrypoints

- 用户入口：设置路由 `/settings`。
- 应用入口：`src/main.ts` 挂载 `src/app/App.vue`。
- 布局入口：`src/components/layout/AppShell.vue`。

## Key Symbols And Modules

- `NConfigProvider`：默认渲染 `.n-config-provider` 的 `div`。
- `.app-shell`：声明 `height: 100%`。
- `.content-scroll`：声明 `overflow: auto`。
- `body`：声明 `overflow: hidden`。

## Dependency And Side-Effect Boundaries

- 根因仅位于 DOM/CSS 高度约束，不涉及设置 Store 或副作用服务。
- 根 Provider 缺少确定高度，使 `.app-shell` 的百分比高度无法相对视口解析，内容随自身高度扩张后被 `body` 裁剪。

## Impact Scope

- 预期修改：`src/app/App.vue`、`src/styles/base.css`、对应应用根测试。
- 验证范围：应用根渲染、前端全量测试、类型检查、生产构建、小窗口滚动尺寸检查。

## Open Follow-up Checks

- 验证根 Provider 类实际落在渲染 DOM 上。
- 验证修复未改变 `.content-scroll` 的滚动所有权。
