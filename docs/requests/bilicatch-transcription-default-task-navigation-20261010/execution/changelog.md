# 执行记录

## 交付单元

`bilicatch-transcription-default-task-navigation-20261010`

## 批准输入

- Spec：`spec/spec.md`
- Plan：`plan/plan.md`
- Task board：`plan/task-board.md`

## 执行摘要

- 将 Rust 权威设置默认转写语言从 `Auto` 改为 `Zh`。
- 将转写页、demo settings 和下载 store 的启动安全缺省对齐为 `zh`。
- 将 TasksPage 的两个 store 初始化延迟到下一浏览器任务，并在卸载时取消未触发 timer。
- 增加默认值/迁移/显式 `auto` 保留、页面表单默认值、初始化延迟与卸载清理测试。
- 完成前端、Rust、构建、浏览器和安装器验证。

## Self-Healing Execution Notes

- loop iteration：3。
- 类型：initial implementation，未从 verify/review 回流。
- defect class：原始缺陷为 `logic-error`（默认值）与 `runtime-error`（首次绘制前副作用时序）。
- progress：RED 测试精确复现后完成最小修复；定向测试暴露三个旧页面测试依赖同步初始化，已按新时序推进一个真实 timer task 后断言。
- remaining verification risk：浏览器自动化点击会等待，不能用工具调用时长量化路由速度；由确定性 fake-timer 测试证明初始化不发生在首次 mount 周期。
- remaining blockers：none。

## Task Board Updates

- T-01：complete，观察到前端 `auto != zh` 和 Rust `Auto != Zh` 的预期失败。
- T-02：complete，默认值链对齐并通过。
- T-03：complete，观察到两个 store 在 mount 时被立即调用的预期失败。
- T-04：complete，延迟初始化和卸载取消通过。
- T-05：complete，完整门禁、浏览器走查和安装包生成完成。

## Changed Files And Owned Symbols

- `src-tauri/src/services/settings/mod.rs`：`SettingsDefaults::values`，拥有全新/缺省设置值。
- `src-tauri/tests/settings_manager.rs`：设置默认、schema v1 迁移、schema v2 显式值测试。
- `src/pages/TranscribePage.vue`：`form.language` 的设置加载前初值。
- `src/pages/TranscribePage.test.ts`：真实 select 默认值测试。
- `src/features/settings/demo.ts`：demo 设置缺省值。
- `src/features/download-center/store.ts`：自动转写 preset 的设置注入前缺省值。
- `src/pages/TasksPage.vue`：`initializeTaskStores`、`initializationTimer` 和生命周期 cleanup。
- `src/pages/TasksPage.test.ts`：初始化时序、卸载取消和旧状态测试的 timer 推进适配。

未新增生产文件、公共 API、组件、store 状态或依赖。

## Introduced-Content Provenance Ledger

| 位置 | 变更内容 | 计划/允许项 | 原始来源 | 结果 |
| --- | --- | --- | --- | --- |
| `SettingsDefaults::values` | 默认语言 `Auto -> Zh` | T-02 / BF-01 | 用户“默认简体中文” | authorized |
| `TranscribePage.form` | 页面初值 `auto -> zh` | T-02 / BF-01 | 用户“默认简体中文” | authorized |
| demo/download defaults | 前端安全缺省 `auto -> zh` | T-02 / BF-01 | 用户“默认简体中文” | authorized |
| `TasksPage` lifecycle | 首次渲染后启动任务初始化 | T-04 / BF-02 | 用户“先切页，数据之后加载” | authorized |
| `TasksPage` cleanup | 卸载取消 pending timer | T-04 cleanup | 已批准运行时安全约束 | authorized |
| 回归测试 | 默认、显式 auto、时序和取消证据 | T-01/T-03 | spec AC-01 至 AC-06 | authorized |

所有文档、测试时序推进、timer handle 和清理 hunk 均为 `product-content delta: none`；没有新增文案、控件、路由、IPC 字段或状态映射。

## Implementation Decisions

- 采用 Level 0 local direct code；`initializeTaskStores` 留在 owning page。
- 使用 `window.setTimeout(..., 0)` 让当前导航任务结束并提供渲染机会；不用 microtask，因为 microtask 仍会在渲染前执行。
- timer handle 使用 `number | null`；回调开始和卸载时都清空 owner 状态。
- 后端 default 是持久化权威值；前端三处值分别是页面、demo 和 store 的局部安全缺省，不抽取虚假共享常量。

## Expert Frontend Engineering Execution Notes

- 用户旅程：RouterLink 和路由不变；目标页先创建，后台任务随后初始化。
- 状态 owner：任务 loading/错误/数据仍由两个 store 拥有；页面只拥有调度 timer。
- 异步正确性：store 的订阅优先于快照规则、事件缓冲和 destroyed guard 均未改变。
- 生命周期：pending timer 在卸载清理，已触发初始化由既有 `dispose` 处理。
- UI：语言 select 文案/选项/交互态不变；任务页布局与反馈态不变。

## Frontend Architecture Quality Execution Notes

不适用：没有 architecture-design 工件，未改变模块边界、依赖方向、状态拓扑或公共 API。现有 owner 保持清晰。

## Production Code Quality Execution Notes

- type-first：复用 `TranscriptionLanguage`、service/event 接口和 DOM timer 类型。
- fail-fast：未新增 I/O 错误边界；现有 store 继续捕获 Promise 拒绝。
- strict null：`null` 唯一表示没有 pending timer；未用 `undefined` 混淆。
- naming：`initializationTimer` 和 `initializeTaskStores` 直接表达所有权/动作。
- no magic variables：依赖来自组件注入和本地闭包。
- performance：没有 memoization/cache/debounce/throttle；timer 只建立一次渲染时序边界。
- boundary states：现有 skeleton、空态和错误态完整保留。

## Human Review Readiness Notes

- 每个新增 hunk 均映射到 T-01 至 T-04；T-05 仅生成证据和安装包。
- 未对用户工作区中已有的大量 ASR/设置改动执行回退、格式化或重构。
- 无 debug log、commented code、TODO、dead code、unused import/export 或临时命名。
- `git diff --check`、`cargo fmt -- --check`、typecheck、测试与 build 均通过。
- 构建保留既有动态导入/chunk size、auth dead-code 和 bundle identifier 警告；本次未扩大范围处理。

## TDD Evidence

### 默认语言

- RED：`TranscribePage.test.ts` 收到 `auto` 而非 `zh`；`settings_manager` 两条测试收到 `Auto` 而非 `Zh`。
- GREEN：修改四个现有缺省 owner 后，前端定向 38/38、Rust settings 14/14 通过。

### 任务页时序

- RED：mount 后下载/转写 `initialize` 均已调用，卸载也无法取消。
- GREEN：局部 timer + cleanup 后，mount 断言调用为 0，推进 timer 后各 1 次，卸载分支保持 0。

## Change-Chain Review Evidence

- 变更前：`RouterLink -> TasksPage mount -> store.initialize -> event subscribe -> snapshot` 同一周期启动。
- 变更后：`RouterLink -> TasksPage mount/render -> timer -> store.initialize -> event subscribe -> snapshot`。
- 默认值链：`SettingsDefaults -> migration/service -> settings store -> TranscribePage watcher` 保持完整；仅 fallback 值改变。
- 下游任务创建仍消费相同枚举和 payload；无重复路径、孤立符号或邻近功能变更。

## Removal Cleanup Evidence

不适用：未移除控件、字段、调用、分支或 API；仅替换默认值并重排初始化开始时机。

## Functional-Programming Execution Notes

没有新增转换规则或可变共享数据。副作用局限于 Vue lifecycle 和既有 Pinia actions；未引入 class 或隐藏 I/O helper。

## Frontend Styling Execution Notes

不适用：没有样式改动。

## API Contract And Adapter Execution Notes

Settings JSON/IPC 的字段、camelCase 映射、枚举域和错误合同未改变；仅缺省枚举值改变。没有 adapter。

## TypeScript Context Recovery Notes

实施前读取根 `tsconfig.json`；遵守 `strict`、no-unused、DOM lib 和 bundler resolution。`npm run typecheck` 通过。

## Architecture-Design Conformance

不适用：spec 明确 `architecture-design_required=false`，执行未引入结构变化。

## Deviations, Blockers, And Rollback Triggers

- 无 spec/plan 偏差。
- 初次 `tauri build` 在安装器生成后因缺少 updater 私钥退出 1；使用仅对该构建关闭 `createUpdaterArtifacts` 的 CLI config override 重新构建，MSI/NSIS 成功退出 0，未修改仓库配置。
- 自动更新签名产物未生成；安装器可直接使用。要发布 updater release 仍需外部私钥，这是发布凭据条件，不是本次代码 blocker。

## Verification Handoff

- 前端：56 files / 211 tests passed。
- Rust：227 passed / 1 ignored。
- TypeScript、Vite build、cargo fmt 通过。
- 浏览器确认语言 option “简体中文” selected，并确认任务列表路由/页面可达。
- 新安装器位于 `src-tauri/target/release/bundle/{msi,nsis}`。

## Iteration 7：任务切换复测修复与 0.1.19 重新打包

- 用户复测时运行的 `D:\program\bilicatch\bilicatch.exe` 与当前 release 哈希不同，确认旧 `0.1.18` 二进制未包含上一轮修复。
- 新增 RED 测试：Pinia 预置缓存任务时，任务页首帧必须显示 skeleton 且不得出现任务行；修改前该测试按预期失败。
- `TasksPage` 新增 `contentReady`，首次渲染期间短路任务投影、批量能力、容量提示和任务错误提示。
- 初始化改为 `requestAnimationFrame -> setTimeout(0)`：先给浏览器一次绘制机会，再开放缓存投影并调用两个 store 的既有初始化；卸载同时清理 frame 和 timer。
- 应用版本由 `0.1.18` 升级至 `0.1.19`，同步更新 npm、Cargo 和 Tauri 版本来源，避免同版本安装器无法可靠覆盖旧程序。
- 删除 bundle 目录中的旧 `0.1.18` MSI/NSIS，仅保留 `0.1.19` 交付物。
- updater 私钥在本机不可用；最终使用 `tauri build --no-sign --bundles msi,nsis` 成功生成本地安装器，仓库中的 updater 配置保持不变。

## Iteration 8：跨页面共享任务运行时与 0.1.20 NSIS

- 根因修正：`TranscribePage` 和 `TasksPage` 不再在页面卸载时 dispose 共享任务 store，避免转写页到任务列表切换期间同步卸载 5 个 Tauri 监听。
- 两个 store 对相同 service/event source 复用进行中或已就绪的初始化 Promise，避免重复订阅和重复快照加载。
- 下载任务 store 在复用会话时仍处理新的 drafts handoff，保留下载页到任务页的任务创建流程。
- `App.vue` 成为最终 cleanup owner；若 subscribe 在 dispose 后才返回，store 会立即调用迟到的 unsubscribe，避免监听泄漏。
- TDD RED 证据：初始实现重复订阅 2 次、页面卸载调用 unsubscribe、根节点不清理；后续复审还捕获了复用路径跳过 handoff 与迟到订阅未释放两项边界。
- GREEN 证据：最终定向 25/25；全量 56 files / 218 tests；typecheck、Vite/Tauri production build、cargo fmt 与 diff check 均通过。
- 版本升级为 `0.1.20`。只执行 `tauri build --no-sign --bundles nsis`，未生成 MSI。
- 最终 NSIS：`src-tauri/target/release/bundle/nsis/BiliCatch_0.1.20_x64-setup.exe`，31,298,180 bytes，SHA-256 `3d7e272252c9b4e8ab650a377a48b5d264282454472a0a5af5a36d3ca60b4f91`。
- 清理旧 `0.1.19` NSIS 后 bundle 中仅保留上述一个文件；MSI 文件数为 0。

## v0.2.0 正式发布

- 将 npm、Cargo、Tauri 与应用版本测试统一升级到 `0.2.0`。
- 发布门禁：前端 56 files / 218 tests，Rust 227 passed / 1 ignored；typecheck、Vite build、cargo fmt 与 diff check 均通过。
- Windows 仅执行 `tauri build --no-sign --bundles nsis`，不构建 MSI。
- 发布文件：`BiliCatch_0.2.0_x64-setup.exe`，31,303,252 bytes，SHA-256 `28856127cd763d23f961892e1e93c4e656f8baa4007c12db1c6e9e4f17b34274`。
- bundle 目录最终文件总数为 1，MSI 文件数为 0。
