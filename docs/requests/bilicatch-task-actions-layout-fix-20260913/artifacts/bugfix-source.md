# 缺陷来源

## 来源信息

- source system：当前 Codex 对话中的直接用户缺陷报告
- project key / source scope：BiliCatch 本地工作区
- defect id：不适用，用户未提供外部缺陷编号
- defect title：任务列表操作列按钮越出列表

## 缺陷事实

- observed behavior：任务列表的操作列按钮显示在表格外面。
- expected behavior：操作列按钮显示在任务列表范围内。
- reproduction clues：打开存在任务行的任务列表；在侧栏占用宽度后的中等窗口宽度下检查最右侧操作列。
- affected module or page：任务列表页、任务行操作列响应式布局。
- screenshots / logs / comments：用户未提供截图；仓库已有任务列表多宽度视觉验证脚本与历史截图。

## Verbatim Defect Evidence Index

- `USER-DEFECT-01`："任务列表的操作列中的按钮显示在表格外面，调整样式，让操作列的按钮显示在列表范围内"

## Closed-World Fix Scope Allowlist

- `source-backed`：调整任务列表样式，使操作列按钮保持在列表范围内。
- `code-fact-backed`：保留既有按钮尺寸、操作矩阵、事件、文案和可访问名称。
- `out-of-scope`：任务状态逻辑、后端/IPC、其他页面布局、操作项增删、视觉重设计。

## Open Questions And Missing Context

- 无阻塞问题。用户描述已明确 observed、expected、范围和修复目标；具体断点由仓库尺寸事实约束。
