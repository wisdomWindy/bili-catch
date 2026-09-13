# 代码上下文

## Context Requirement

- 需要理解任务页容器、任务行网格和操作控件之间的尺寸关系。
- 首次需要阶段：bugfix-intake。
- code graph：仅偏好，不要求；缺陷局限于已知 CSS 布局链。

## Graph Availability Check

- status：missing / not needed。
- detection method：仓库未暴露 code graph 配置；按 `code-graph.md` 的轻量样式缺陷例外使用定向 `rg` 与文件读取。
- iteration：bugfix-intake，2026-09-13。

## Installation Or Bootstrap Record

- attempted：no。
- result：not_needed。
- reason：一个已知样式文件内的局部响应式缺陷，依赖闭包可由直接 DOM/CSS 关系完整恢复。

## Fallback Record

- fallback used：yes。
- method：定向搜索、相关组件/样式/测试读取、Git blame 与浏览器尺寸检查。
- residual confidence：高；操作列无跨模块状态或副作用依赖。

## Relevant Entrypoints

- 用户入口：`/tasks`。
- 页面入口：`src/pages/TasksPage.vue`。
- 行组件：`src/features/task-management/components/TaskRow.vue`。
- 样式入口：`src/features/task-management/task-management.css`。

## Functional Chain Under Change

`TasksPage .task-list` -> `TaskRow .task-row` CSS Grid -> `.task-row__actions` -> `AppIconButton` / completed open menu。变更仅拥有 Grid/操作列布局；动作计算、emit、store 和服务调用均为邻接边界，不得修改。测试脚本属于适配目标，不是生产所有者。

## Key Symbols And Modules

- `.task-list__header`, `.task-row`, `.task-row__actions`
- `TaskRow`、`AppIconButton`

## Dependency And Side-Effect Boundaries

- CSS 负责视觉布局；Vue 组件保留现有 DOM、事件和状态派生。
- 无 API、store、IPC 或持久化副作用。

## Impact Scope

- 预计修改 `task-management.css` 和任务视觉验证脚本。
- 验证任务页多宽度、各状态操作控件边界、现有 TaskRow 测试与前端构建。

## Open Follow-Up Checks

- 在 1280、900、800px 以及断点邻近宽度验证操作控件相对 `.task-list` 的边界。
