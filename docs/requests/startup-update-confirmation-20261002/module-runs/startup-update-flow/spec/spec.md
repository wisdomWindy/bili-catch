# 工程规格：启动更新确认流程

## 交付单元标识

`startup-update-flow`

## 背景与目标

在现有 Tauri updater、设置开关和手动检查功能上，增加一次受控的启动自动检查。只有发现新版时才显示全局确认弹窗，并由用户决定是否立即安装。

## 范围内

- 根级自动检查协调。
- 更新确认、安装中和安装失败弹窗状态。
- 中英文文案与响应式样式。
- 行为、组件、类型与构建验证。

## 范围外

- Rust、IPC、设置 schema、更新 endpoint、签名、发布清单与流水线。
- 强制更新、跳过版本、后台周期轮询、更新日志。
- 设置页手动更新 UI 改版。

## 触发与启动条件

- `useSettingsStore.status === "ready"`。
- `useSettingsStore.values?.autoCheckUpdates === true`。
- 当前根组件生命周期尚未启动过自动检查。
- 触发后立即置一次性闩锁，后续 store 更新不得重复检查。

## 需求拆分摘要

单模块交付，承接 `startup-update-flow` 全部行为；先完成的 page-design 规定紧凑全局模态、版本对照、安全默认焦点、安装锁定和错误重试。

## 用户流程

1. 应用启动，设置 store 初始化。
2. 设置 ready 后判断开关；关闭则结束，开启则调用 `checkForUpdates()` 一次。
3. `latest` 或检查异常时静默结束。
4. `available` 时保存 `currentVersion/latestVersion` 并打开弹窗。
5. “稍后”关闭弹窗，本次生命周期不再自动提示。
6. “立即更新”进入 installing 并调用 `installUpdate()`。
7. 成功路径由端口完成安装和 relaunch；失败则进入 error，保留版本信息并允许重试。

## 页面与模块设计

- `StartupUpdateBridge.vue`：根级业务组件，注入设置 store 与 release actions，拥有一次性闩锁、检查、版本信息、安装状态和错误状态；渲染 dialog。
- `UpdateConfirmDialog.vue`：展示组件，仅接收 `show`、`pending`、版本和错误，发出 `close/confirm`；负责焦点、Esc、遮罩和 Tab 循环。
- `App.vue`：在现有 providers 内挂载 bridge，不承载更新状态机。
- `release.ts`：端口和实现不变，继续作为 Tauri updater 唯一适配边界。

## 功能完整行为拆分

### 自动检查协调

- 监听设置 `status` 与 `autoCheckUpdates`。
- 只在 ready + true 首次成立时检查。
- 检查前记录闩锁，防止并发或响应式重复触发。
- available 保存两个非空版本字符串并显示弹窗。
- latest、异常、组件卸载后的迟到结果均不显示弹窗。

### 更新确认弹窗

- `role=alertdialog`、`aria-modal=true`，标题有稳定 ID。
- 显示当前版本与新版本，不对字符串做 trim 或重新格式化。
- 默认焦点为“稍后”，关闭后恢复原焦点。
- 非 pending 时允许稍后、Esc 和遮罩关闭。
- pending 时上述关闭路径全部失效，两个按钮禁用或仅主按钮显示稳定 loading 状态。
- Tab 在可用按钮间循环。

### 安装与重试

- confirm 将 pending 置 true，清除旧错误，并调用 `installUpdate()`。
- 同一 pending 周期重复 confirm 不产生第二次调用。
- install 抛错时 pending 复位、错误设为本地化“更新安装失败”。
- 错误态再次 confirm 重新调用 install。
- install 成功通常触发应用重启，不要求本地 success UI。

### 邻接流程

- 设置页手动检查与托盘事件不改签名、不改路由和不改状态文案。
- `autoCheckUpdates=false` 时启动不得调用 release actions。
- 设置加载失败时不得调用 release actions。

## 设计约束

- 职责：协调副作用与纯展示分离；根 App 不吸收状态机。
- 命名：稳定使用 startup update、available、installing、currentVersion、latestVersion。
- 规则所有权：一次性自动检查归 bridge；Tauri 更新对象与 relaunch 归 release adapter；焦点与关闭归 dialog。
- 副作用：插件调用不得进入 dialog 或 template。
- 复杂度：使用直接响应式状态与 guard，不引入通用状态机、事件总线或新 store。
- 可读性：检查与安装各有单一 async 入口，异常分支显式且不泄漏私有错误。

## 项目脚手架决策

不适用。现有 Vue 3 + Pinia + Naive UI + Vitest 工程直接扩展，不新增包或脚手架。

## 变化轴与模式决策

- 变化轴仅是更新流程状态（hidden/available/installing/error），直接状态与 guard 足够。
- 不引入命名设计模式；现有 `ReleaseActionsPort` 已承担 Adapter 边界。
- 不新增通用 modal manager、状态机库或 updater facade。

## 代码上下文与影响假设

- `App.vue` 已在 `NDialogProvider` 内，可挂载 Naive `NModal`。
- `main.ts` 为设置页和根级组件提供同一 release actions 实例。
- `settingsStore.initialize` 完成后原子地设置 values 和 ready。
- 影响文件限定在 app、settings components、locale、settings CSS 与测试。

## API 与数据合同

- 权威合同源：本地 TypeScript `ReleaseActionsPort` 和 `UpdateCheckResult`。
- 直接消费，无新 adapter 或字段映射。
- `checkForUpdates(): Promise<{status:"latest", currentVersion:string} | {status:"available", currentVersion:string, latestVersion:string}>`。
- `installUpdate(): Promise<void>`；副作用包含下载安装与 relaunch。
- 检查错误由 bridge 静默处理；安装错误由 bridge 转为本地化 UI 状态。
- 无后端 API、protobuf、OpenAPI 或新增 IPC。

## 上下文与依赖来源

- `tsconfig.json` governs 所有 `src/**/*.ts` 和 `src/**/*.vue`。
- 关键选项：`strict=true`、`noUnusedLocals=true`、`moduleResolution=bundler`、`isolatedModules=true`、Vue JSX preserve。
- 环境声明：`src/vite-env.d.ts` 的 Vite client 类型。
- 本地类型：settings contracts、release actions port；第三方组件类型来自 Vue、Naive UI 和 Lucide 包声明。

## 边界情况

- 设置关闭、加载失败、检查 latest、检查 reject：无弹窗。
- store 多次 ready 更新：仅检查一次。
- 检查 Promise 在组件卸载后 resolve：忽略。
- 用户关闭后：不重复自动提示。
- install 重复点击：pending guard 阻止并发。
- install reject：显示稳定错误，可重试。
- 窄视口和长英文版本/文案：换行但不溢出。

## 验收标准

- AC-1：ready + autoCheckUpdates=true 时自动检查一次；后续设置变化不重复检查。
- AC-2：autoCheckUpdates=false 或设置加载失败时不检查。
- AC-3：latest 和检查异常均不显示弹窗。
- AC-4：available 显示当前版本、新版本、“稍后”和“立即更新”。
- AC-5：稍后关闭；安装中不可关闭且重复确认不会重复调用。
- AC-6：确认调用 install；失败显示本地化错误并可重试。
- AC-7：弹窗默认聚焦安全操作、关闭恢复焦点、具有 alertdialog 语义。
- AC-8：现有手动检查、托盘更新、类型检查、前端测试和构建通过。

## 人工审查与交接

- 审查确认弹窗层级、焦点和安装锁定符合 page-design。
- 发布后使用高于当前版本的 updater manifest 完成真实安装重启冒烟。

## 风险

- Naive `NModal` teleport 使测试查询需挂载到 document.body。
- 安装成功会重启，自动测试只能验证端口调用，真实 relaunch 留给发布冒烟。
- 与手动检查共享 pending update 缓存；保持单实例注入和端口不变可降低风险。

