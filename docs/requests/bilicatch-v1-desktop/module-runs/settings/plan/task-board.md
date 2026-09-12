# 任务板：设置管理

## 执行规则

- 固定顺序：SET-01 -> SET-02 -> SET-03 -> SET-04 -> SET-05 -> SET-06 -> SET-07。
- 状态域：`pending | in_progress | completed | blocked`；同一时间只允许一个 `in_progress`。
- 每项必须完成可信 red、最小 green、重构、定向门禁和 changelog 记录后才能 completed。
- 单 agent 串行，不启用 workflow/subagent。目录不是 Git 仓库，不创建虚假 commit 检查点。

## 总览

| ID | 名称 | 状态 | 模式 | 前置 |
| --- | --- | --- | --- | --- |
| SET-01 | 跨端合同、类型闭包与依赖 | completed | 串行 | 计划批准 |
| SET-02 | Rust 迁移、校验、事务存储与 Commands | completed | 串行 | SET-01 |
| SET-03 | 任务调度与下载中心设置消费 | completed | 串行 | SET-02 |
| SET-04 | 前端 Service、Store 与效果协调 | completed | 串行 | SET-03 |
| SET-05 | 目录、Release Port、Demo 与根 Composition | completed | 串行 | SET-04 |
| SET-06 | 四组设置页面与无障碍 | completed | 串行 | SET-05 |
| SET-07 | 全量回归、视觉、安全与验收证据 | completed | 串行 | SET-06 |

## SET-01

- 任务名称：跨端合同、类型闭包与最小依赖。
- 状态：completed。
- 执行模式/说明：串行，contract-first TDD。
- 触发/前置：用户批准计划。
- 规格映射：SET-AC-04..07、09、18、20；API/TypeScript/脚手架。
- 功能单元：V1 Settings DTO、typed patch、media shared types、store/dialog dependencies。
- 页面/模块范围：TS/Rust contracts、manifests/locks；无 UI。
- 整洁性：字段名同源，download contracts 兼容 re-export；不升级无关包。
- Pattern/边界：Rust serde source -> TS direct translation；无 mapper。
- 上下文影响：strict/no alias/isolatedModules；读取 dialog package declarations。
- 关键交互/状态：无运行时交互；冻结 default/range/enum/nullability。
- API/类型：11 required values、snapshot schemaVersion/revision、mapped union patch。
- 测试切入：TS defaults/type、Rust exact JSON、typecheck、dependency resolution。
- 已批准假设：CL-SET-01/02/07；无待确认项。

## SET-02

- 任务名称：Rust 设置迁移、校验、事务存储与 Commands。
- 状态：completed。
- 执行模式/说明：串行，fake store/fault injection TDD。
- 触发/前置：SET-01 completed。
- 规格映射：SET-AC-03..10、18、20；migration/validation/store/API。
- 功能单元：defaults、V1 migration、typed validation、candidate transaction、cache rollback、commands。
- 页面/模块范围：Rust settings model/service/infrastructure/command/lib。
- 整洁性：manager 无 AppHandle；command 只委派；message 不含路径/raw JSON。
- Pattern/边界：StorePort + candidate transaction；拒绝 auto-save/前端持久化。
- 上下文影响：新增 plugin registration；固定 settings.json/document。
- 关键交互/状态：load -> normalize -> explicit save -> snapshot；update no-op/commit/fail。
- API/类型：`get_settings_snapshot`、`update_setting` exact contract。
- 测试切入：migration matrix、range/path、revision、save/cache fault、fmt/check。
- 已批准假设：CL-SET-04/07/08；无待确认项。

## SET-03

- 任务名称：任务调度与下载中心设置消费。
- 状态：completed。
- 执行模式/说明：串行，高回归风险邻居接入。
- 触发/前置：SET-02 completed。
- 规格映射：SET-AC-05、11、12、17、18。
- 功能单元：TaskSettingsPort、dynamic claim limits、DownloadDefaults/configure/fallback。
- 页面/模块范围：Rust tasks manager/ports/commands/tests、download-center store/contracts/tests。
- 整洁性：consumer 只读窄子集；不 import store adapter/full settings DTO。
- Pattern/边界：provider port；拒绝第二 scheduler config/setter。
- 上下文影响：现有 task constructors/tests、parse selection/drafts/clear 回归。
- 关键交互/状态：新 limits 只影响后续 claim；当前选择优先于 settings default。
- API/类型：SchedulerLimits 1..10/1..32；DownloadDefaults 三字段。
- 测试切入：32 connections、lower active limit、quality/audio fallback、manual override。
- 已批准假设：CL-SET-02/03/09；无待确认项。

## SET-04

- 任务名称：前端 Service、Settings Store 与效果协调。
- 状态：completed。
- 执行模式/说明：串行，deferred promise/fake timer TDD。
- 触发/前置：SET-03 completed。
- 规格映射：SET-AC-06..10、16..18。
- 功能单元：exact IPC、hydrate/retry、revision merge、field status/drain、appearance rollback、effect sink。
- 页面/模块范围：features/settings service/store/effects、app settings-effects、app/download stores。
- 整洁性：Promise/worker 不进 Pinia state；单一 drain owner；raw error 不到 UI。
- Pattern/边界：per-field drain + explicit sink；拒绝 event bus/watch fan-out。
- 上下文影响：根 app theme/locale behavior、download defaults consumer。
- 关键交互/状态：loading/ready/error；preview/saving/saved/error；A->B->C latest。
- API/类型：direct SettingsSnapshot/Patch；effect methods exact。
- 测试切入：invoke args、lifecycle stale、coalescing、timer、rollback、typecheck。
- 已批准假设：CL-SET-05/09；无待确认项。

## SET-05

- 任务名称：目录、Release Port、Demo 与根 Composition。
- 状态：completed。
- 执行模式/说明：串行，adapter/composition TDD。
- 触发/前置：SET-04 completed。
- 规格映射：SET-AC-03、13、14、18..20。
- 功能单元：dialog exact options、release deferred、demo states、injection、root hydrate、capability。
- 页面/模块范围：settings native/runtime modules、main/App、Tauri capability/plugin registration。
- 整洁性：production/demo 明确分支；组件不直接 import plugin。
- Pattern/边界：Port/Adapter；release later replaced without page change。
- 上下文影响：App auth/backend lifecycle、existing opener permission。
- 关键交互/状态：select/cancel/error、latest/available/error、startup load。
- API/类型：DirectoryPickerPort、ReleaseActionsPort、unavailable injection defaults。
- 测试切入：exact open call、deferred details、hash demo、App lifecycle、permission scan。
- 已批准假设：CL-SET-04/06；真实 endpoint/license 明确 deferred。

## SET-06

- 任务名称：四组设置页面、即时反馈与无障碍。
- 状态：completed。
- 执行模式/说明：串行，component/page TDD + responsive styling。
- 触发/前置：SET-05 completed。
- 规格映射：SET-AC-01..08、13..17、19。
- 功能单元：14 项 UI、field controls/status、About、loading/error、notifications、a11y/i18n/CSS。
- 页面/模块范围：SettingsPage、settings components/tests/CSS、locales/pages tests。
- 整洁性：components props/emits；page only orchestration；无动态表单/页卡。
- Pattern/边界：复用 SettingRow/Section 的真实结构共性；不提升 global shared。
- 上下文影响：AppShell content width、Naive theme、existing locale parity。
- 关键交互/状态：slider preview/commit、picker focus、saving fade、update pending、theme/locale focus。
- API/类型：只消费 store 与 injected ports；AppInfo version read-only。
- 测试切入：14 项、全部状态、typed emits、aria/keyboard/focus、zh/en/typecheck。
- 已批准假设：CL-SET-01/04/05/06；无待确认项。

## SET-07

- 任务名称：全量回归、视觉、安全与验收证据。
- 状态：completed。
- 执行模式/说明：串行收口；失败回 owner task。
- 触发/前置：SET-06 completed。
- 规格映射：SET-AC-01..20 全部。
- 功能单元：full gates、static security/capability、visual matrix、evidence/changelog。
- 页面/模块范围：完整 settings + app/download/task 回归 + docs。
- 整洁性：证据与 AC 一一对应；deferred 不伪装 production pass。
- Pattern/边界：复核 Port/transaction/drain/sink 均保持轻量。
- 上下文影响：所有已完成模块回归；为 verify/review 提供原始证据。
- 关键交互/状态：只有 blocker 清零才移交 verify。
- API/类型：serde/TS/plugin/dialog/release/consumer 全链。
- 测试切入：npm/Cargo full gates、静态扫描、多状态多视口 DOM/截图。
- 已批准假设：CL-SET-06；真实 updater/license 留 system-release。
