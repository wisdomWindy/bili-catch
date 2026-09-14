# 工程规格：设置页滚动高度链修复

## Delivery Unit Identifier

`bilicatch-settings-scroll-fix-20260914`

## Background And Goals

`NConfigProvider` 默认渲染一个 `div.n-config-provider`。该元素位于具有 `height: 100%` 的 `#app` 与 `.app-shell` 之间，却没有确定高度，导致 `.app-shell` 的百分比高度不能形成稳定的视口约束；设置内容把内部网格撑高后，又被 `body { overflow: hidden }` 裁剪。目标是补齐这一层高度约束，让已有 `.content-scroll { overflow: auto }` 正常工作。

## In Scope

- 给应用根 `NConfigProvider` 添加语义化类 `app-provider`。
- 在基础全局样式中让 `.app-provider` 占满父级高度并允许网格后代收缩。
- 添加应用根渲染回归测试，防止移除该高度链标记。

## Out Of Scope

- 不修改 `SettingsPage.vue`、设置项或设置 Store。
- 不启用 `body` 滚动，不创建第二个滚动容器。
- 不改变宽度断点、侧栏、顶部栏、路由、IPC 或后端行为。

## Trigger And Start Conditions

- 用户进入设置页。
- 窗口可视高度小于设置内容总高度。

## Requirement Split Summary

本缺陷为非拆分的单交付单元，根 Provider 类与对应全局样式共同构成一个不可分割的高度链修复。

## User Flow

1. 用户打开设置页。
2. 用户缩小窗口高度。
3. 主内容区域高度保持受窗口约束。
4. 超出可视区域的设置项通过 `.content-scroll` 纵向滚动访问。

## Page And Module Design

- 页面内容结构不变。
- `App.vue` 负责声明根 Provider 的布局身份。
- `base.css` 负责根级高度链；`layout.css` 继续负责应用壳层和主内容滚动。

## Function-complete Behavior Breakdown

- 根渲染：`NConfigProvider` 生成的真实 DOM 元素必须包含 `app-provider` 类。
- 高度传递：`.app-provider` 必须具有 `height: 100%`，以 `#app` 的确定高度作为包含块。
- 收缩能力：`.app-provider` 必须具有 `min-height: 0`，避免作为布局祖先阻止后代收缩。
- 滚动所有权：`.content-scroll` 继续使用既有 `overflow: auto`；不在设置页或 `body` 上增加滚动规则。
- 稳态：内容未溢出时布局与当前页面一致；内容溢出时仅主内容区域滚动。

## Design Constraints

- responsibility：`App.vue` 只声明结构类，根高度规则放在 `base.css`；设置页不承担应用级高度约束。
- naming：使用明确的 `app-provider`，避免耦合第三方内部类 `.n-config-provider`。
- duplication ownership：根高度规则只有一个所有者，不复制到各页面。
- side effects：无运行时副作用、Store 写入或事件变化。
- complexity：保持直接 CSS 修复，不新增包装组件或工具函数。

## Project Bootstrap And Scaffold Decision

不适用；在既有 Vue + Vite + Tauri 项目内做局部修复。

## Change Axes And Pattern Decision

无行为变化轴，也不需要设计模式。直接类名与 CSS 约束是最小且最清晰的实现，禁止为单一规则引入额外抽象。

## Code Context And Impact Assumptions

- 高度链：`html/body/#app -> .app-provider -> .app-shell -> .workspace -> .content-scroll`。
- `body` 的 `overflow: hidden` 保持不变，以确保只有主内容区滚动。
- Naive UI 当前 `ConfigProvider` 默认渲染 `div` 并转发 class；应用测试验证该本地契约。

## API And Data Contracts

无 API、数据结构、后端类型或持久化契约变化。

## TypeScript Context

- `src/app/App.vue` 由根 `tsconfig.json` 管理。
- 关键选项：`strict: true`、`moduleResolution: bundler`、`jsx: preserve`、DOM lib。
- 相关声明：`src/vite-env.d.ts`；本次模板类属性不新增 TypeScript 类型。

## Context And Dependency Sources

- 用户缺陷描述与方案批准。
- `src/app/App.vue`、`src/styles/base.css`、`src/styles/layout.css`。
- `node_modules/naive-ui/es/config-provider/src/ConfigProvider.mjs` 的真实渲染实现。

## Edge Cases

- 极小窗口高度：主内容区可以缩至网格允许范围并继续滚动。
- 设置加载态或错误态：使用相同壳层高度链，不需专用规则。
- 其他路由内容溢出：同样受益于正确的主内容滚动边界，不改变页面业务行为。

## Acceptance Criteria

- `AC-01`：应用根渲染测试能在真实 `NConfigProvider` DOM 上找到 `.app-provider`。
- `AC-02`：`.app-provider` 的生效样式包含 `height: 100%` 和 `min-height: 0`。
- `AC-03`：`.content-scroll` 仍是主内容唯一滚动容器，现有 `body { overflow: hidden }` 不变。
- `AC-04`：相关单测、全量前端测试、类型检查和生产构建通过。
- `AC-05`：小窗口验证中，设置页可视容器 `scrollHeight > clientHeight` 且滚动位置能够变化。

## Human Review And Handoff

用户以 `USER-APPROVAL-01` 明确批准按已给出的根 Provider 高度方案修改；实现完成后交付改动与验证证据。

## Risks

- 根 Provider 高度规则影响所有路由；通过保留现有 `.content-scroll` 所有权和全量测试降低回归风险。
- JSDOM 不执行真实布局，尺寸行为需额外使用浏览器级检查验证。
