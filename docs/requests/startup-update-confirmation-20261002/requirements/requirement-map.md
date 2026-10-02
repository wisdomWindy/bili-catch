# 需求拆分总览

## 请求摘要

为现有更新器增加受设置控制的启动检查，并在发现新版时显示全局确认弹窗。

## 分析承接

- 范围固定为前端启动编排、确认弹窗、locale 与测试。
- 不修改 updater 协议、设置 schema、IPC、Rust 后端或发布流水线。
- 自动检查错误静默；安装错误在弹窗内可重试。
- 使用既有 release actions 实例，保持手动更新流程兼容。

## 拆分原则

启动检查、确认和安装状态是同一个用户流程，拆开会导致状态所有权与验收割裂，因此保留为单模块顺序交付。

## 来源清单

- 用户要求与批准设计。
- `request.md`、`artifacts/prd-snapshot.md`、`analysis/requirement-analysis.md`。
- 仓库现有根组件、设置 store、release actions、模态框样式与 locale。

## Markdown 归一化说明

原始输入为纯文本对话，没有表格截图、富文本或不可表达格式，无需额外归一化；模块工件同时保留来源快照和结构化 Markdown 副本。

## 模块清单

| ID | 名称 | 类型 | 来源映射 | 范围 | 顺序 | page-design | architecture-design | 下游根目录 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `startup-update-flow` | 启动更新确认流程 | interaction flow | 请求与需求快照全部章节 | 启动检查、确认弹窗、安装状态、双语文案和测试 | 1 | 是 | 否 | `module-runs/startup-update-flow/` |

## 模块依赖与顺序

- 单模块，无并行依赖。
- 先完成 page-design，再进入 spec、plan、execute、verify、review。

## 模块执行顺序

1. `startup-update-flow`，首阶段 `page-design`。

## Page Design 候选

`startup-update-flow` 必须定义弹窗信息层级、安全默认焦点、安装锁定、错误重试和移动端尺寸。

## Spec 承接清单

- 设置 ready 后且开关开启才检查。
- 每个根生命周期最多自动检查一次。
- latest 和检查错误静默。
- available 显示当前版本、新版本及两个操作。
- 安装中不可关闭；失败保留弹窗并允许重试。
- 成功沿用自动重启。
- 手动与托盘检查不回归。

## 开放问题与未决歧义

无。

