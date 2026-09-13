# 规格：任务操作列布局修复

## Delivery Unit Identifier

`bilicatch-task-actions-layout-fix-20260913`

## Background And Goals

浏览器实测在 1100px 窗口出现 10 个操作控件越过 `.task-list`，851px 出现 6 个。根因是任务网格使用 viewport media query，而实际宽度同时受 208/72/60px 侧栏、内容 padding 和滚动条影响。目标是按任务列表自身可用宽度选择六列、五列或两列布局。

## In Scope

- 调整 `task-management.css` 中任务列表的响应式布局依据。
- 扩充任务视觉验证脚本，使其比较操作控件与列表边界。
- 覆盖 1280、1100、900、851、800px。

## Out Of Scope

任务状态、动作矩阵、按钮/菜单 DOM、IPC、store、文案、其他页面样式与视觉重设计。

## Trigger And Start Conditions

打开 `#/tasks?demo=1`，任务列表至少包含 downloading、completed 等拥有操作控件的任务。

## Requirement Split Summary

不适用：bugfix 非拆分交付单元。

## Source Grounding And Traceability

| Spec item | Label | Product-facing | Authority | Plan obligation |
| --- | --- | --- | --- | --- |
| 操作控件不得越过列表边界 | source-backed | yes | `USER-DEFECT-01` | 加入相对边界回归断言并修复 |
| 保留控件集合、尺寸与交互 | code-fact-backed | no new semantics | TaskRow/AppIconButton 现状 | 不改组件行为 |
| 用内容区宽度驱动响应式切换 | code-fact-backed | no | 1100/851px 复现及 Grid 最小宽度 | 修改既有 CSS |

Closed-world allowlist：只允许改变任务网格响应式布局。不得新增/删除字段、文案、状态、动作、导航或反馈。

## Workflow Efficiency Profile

`S1 local`，compact 工件、narrow context；若实现需要组件 DOM、跨模块布局或 JS 状态变化，升级到 S2 并回退规格。

## Self-Healing Completion Bar

需求一致性、操作控件边界、现有交互测试、CSS/DOM 引用链、构建与运行时浏览器检查全部通过；无无关 diff、死代码或调试输出；最终要求 `self-healing loop compliance: pass`、`self-healing loop assessment: pass`、blocker none。

## User Intent Contract

- literal：调整样式，让操作列按钮显示在列表范围内。
- practical goal：所有任务操作完整可见、可操作。
- success：每个操作控件的 left/right 均位于对应 `.task-list` 边界内，页面无水平溢出。
- forbidden：裁剪、隐藏、缩小按钮、改变动作集合。
- acceptable：基于容器宽度切换既有列布局。

## User Flow

进入任务页 -> 扫描任务 -> 在行尾选择操作。成功、失败、取消、重试和确认流程不变。

## Page And Module Design / UI UX Contract

遵循 `design/page-design.md`。宽表格保持现有字段顺序；中等容器隐藏速度；窄容器隐藏表头并使用两列任务行。视觉层级、颜色和交互状态不变。

## Frontend Styling Constraints

沿用模块唯一样式 owner `task-management.css`，只修改既有布局规则并加入必要的容器上下文；不新增语义类、scoped CSS、内联样式或 class 常量。仓库未配置 Tailwind，因此不得为单点缺陷引入新样式工具链。

## Field Display Semantics And Interaction Visual States

操作列表示当前任务允许执行的行级命令。按钮默认、hover、focus、active、disabled、菜单及危险操作语义全部保持既有实现；本次只保证空间归属，不改变显示精度、文本或强调。

## Function-Complete Behavior Breakdown

1. `.task-list` 建立 inline-size 容器上下文。
2. 列表内容宽度足够时使用既有六列 Grid。
3. 内容宽度低于六列最小占用时使用既有五列 Grid并隐藏速度。
4. 内容宽度低于五列最小占用时使用既有两列行布局并隐藏表头。
5. 所有状态下操作区可换行但不越过列表边界；36x36px 控件不缩小。
6. 验证脚本对列表边界、行 scrollWidth、viewport overflow 和控件尺寸做断言。

## Expert Frontend Engineering Constraints

不改变 state/data/async owner。交互使用原生 button/summary，焦点、ARIA 和 disabled 保持。列表最大 100 条的既有渲染策略不变；不引入 JS resize listener、watcher 或响应式状态，回滚面仅 CSS 与验证脚本。

## Frontend Architecture Design Routing Inputs

`architecture-design_required: false`。无模块边界、ownership、依赖方向、共享抽象或跨文件协作变化。

## Production Code Quality Constraints

不新增 TypeScript 合同、函数或状态。CSS 规则需直接、局部、无魔法 JS；断点值必须由现有 Grid 最小宽度解释。异常/空值/异步语义不适用。

## Human Review Readiness Constraints

允许改动：任务 CSS、视觉脚本、当前请求工件。组件、store、服务和后端不改。每个 hunk 必须映射到边界回归或内容区断点修复；提供 focused test、完整前端 test/typecheck/build 和浏览器指标。

## Functional-Programming / Architecture Reuse / Locality

不适用：不改转换、规则、状态派生或共享逻辑；不新增 helper、组件、utility 文件。

## Vue Component Extraction Constraints

保持 `TaskRow` 与 `AppIconButton` 现有边界；无抽取候选。

## Design Constraints

CSS 是布局唯一 owner；不能用 `overflow: hidden` 掩盖问题；不能让任务行出现水平滚动；表头与行使用同一列模板。

## Project Bootstrap And Scaffold Decision

不适用：既有 Vite/Vue/Tauri 项目。

## Change Axes And Pattern Decision

- concrete change：一个稳定的局部布局规则。
- depth：Level 0。
- decision：direct code；pattern：none；syntax shape：既有 CSS container query。
- rationale：无 selection、adaptation、creation、sequencing、副作用、状态分支或跨文件协作信号；新增 composable/observer/JS resize adapter 反而扩大状态与生命周期风险。

## Code Context And Impact Assumptions

链路为 `TasksPage.task-list -> TaskRow.task-row -> task-row__actions -> AppIconButton/open menu`。根 `tsconfig.json` governs Vue/TS，但本次不改 TS；Vite/Edge/WebView2 支持 container query。后置检查确保引用链和行为不变。

## API And Data Contracts / Frontend Server Boundary

不适用：无 API、DTO、IPC 或服务端职责变化。

## Context And Dependency Sources

`TasksPage.vue`、`TaskRow.vue`、`AppIconButton.vue`、`task-management.css`、`layout.css`、`verify-task-visuals.cjs`、既有 task-management 页面设计/验证。

## Edge Cases

- 1100px：宽侧栏仍存在，内容区不足以容纳六列。
- 851px：窄侧栏已触发但任务旧窄断点尚未触发。
- completed 行同时有菜单与删除按钮。
- downloading/failed 等两按钮状态。
- 垂直滚动条进一步减少内容宽度。

## Acceptance Criteria

- AC-01：1280、1100、900、851、800px 下 `buttonsOutsideList=0`。
- AC-02：所有任务行 `scrollWidth <= clientWidth + 1`，document width 不超过 viewport。
- AC-03：操作控件不小于 36x36px，现有 TaskRow 交互测试通过。
- AC-04：前端测试、typecheck 和 build 通过。

## Human Review And Handoff

交付 CSS/回归脚本 diff、浏览器指标、测试与构建结果；不提交、不改后端。

## Risks

容器断点若小于 Grid 实际最小宽度仍会产生窄区间溢出；用轨道+gap+padding 的 border-box 总宽度确定阈值并覆盖临界附近。
