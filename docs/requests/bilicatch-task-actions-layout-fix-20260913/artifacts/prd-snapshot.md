# 归一化缺陷快照

## Verbatim Source Evidence Index

- `USER-DEFECT-01`："任务列表的操作列中的按钮显示在表格外面，调整样式，让操作列的按钮显示在列表范围内"

## Source Summary

任务列表的最右侧操作按钮越过列表边界，需通过样式调整使其保持在列表范围内。

## Key Business Goals

- `source-backed`：用户在任务列表中可看到并操作完整的行级操作按钮。

## Greenfield Scaffold Sensitivity

不适用：not a project/app/package/frontend-surface creation request。

## Explicit Behavior Constraints

- `source-backed`：操作列按钮不能显示到任务列表外。
- `code-fact-backed`：不得通过隐藏操作、改变动作集合或缩小既有控件来规避布局问题。

## Closed-World Product Scope Allowlist

- 仅允许修改任务列表响应式布局，使操作列位于列表边界内。

## Forms, Tables, Displays, And Interactions

- 表格/列表：任务列表 `.task-list`。
- 显示：任务行 `.task-row` 的操作列 `.task-row__actions`。
- 交互：现有按钮点击、完成任务的打开菜单均保持不变。

## Workflow And State Rules

不适用：不改变任务状态或工作流。

## Relevant Modules Or Pages

- `src/features/task-management/task-management.css`
- `src/features/task-management/components/TaskRow.vue`
- `scripts/verify-task-visuals.cjs`

## Open Questions

无。
