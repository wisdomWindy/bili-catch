# 任务看板

## T-01 默认语言回归测试（RED）

- 状态：complete
- execute-readiness：ready
- 执行模式：串行，先于 T-02
- 执行说明：先写并观察真实失败，不修改生产代码。
- 触发/前置条件：spec 已批准。
- 关联规格区域：3.1、AC-01 至 AC-04。
- 业务功能点：BF-01。
- 覆盖功能单元：新设置、schema v1 缺省迁移、schema v2 显式值、页面初值。
- 页面/模块：settings service、TranscribePage。
- 目标文件与符号：`settings_manager.rs` migration tests；`TranscribePage.test.ts` 页面表单。
- 整洁性约束：字面期望 `Zh`/`zh`，不复用生产默认常量计算 expected。
- 工程约束：测试真实 manager/真实页面，外部服务使用完整最小 double。
- 模式约束：现有测试形态，禁止新测试 helper 抽象。
- 来源：`[SRC-U1][DATA-D1]`，spec AC-01 至 AC-04。
- API/类型策略：复用 Rust 枚举和前端现有联合类型。
- 测试切入点：`cargo test --test settings_manager`；`npm test -- src/pages/TranscribePage.test.ts`。
- 假设：无产品假设。

## T-02 对齐默认语言实现（GREEN）

- 状态：complete
- execute-readiness：ready
- 执行模式：串行，依赖 T-01 RED。
- 执行说明：以最小值替换使测试通过。
- 触发/前置条件：T-01 已按预期因 `auto` 失败。
- 关联规格区域：3.1、4、6、7。
- 业务功能点：BF-01。
- 覆盖功能单元：后端权威默认、页面初值、demo、下载 store 缺省。
- 页面/模块：settings、transcription、download-center。
- 目标文件与符号：`SettingsDefaults::values`、`TranscribePage.form`、demo settings values、download store defaults。
- 整洁性约束：不改测试 fixtures 中用于验证显式 `auto` 的值，不做全局机械替换。
- 工程约束：持久化 owner 不变；schema v2 显式值保持。
- 生产代码质量：复用强类型枚举，不新增 assertion/any/兜底分支。
- 代码评审预检：确认所有启动缺省对齐且每处有明确 owner。
- 副作用边界：持久化仍仅由 SettingsManager 负责。
- 变更链审查：默认值 -> migration/service -> settings store -> page watcher；下载 store preset 仅为安全缺省。
- 模式约束：Level 0，保持分离，不抽跨语言常量。
- 来源：`[SRC-U1][CODE-C1][CODE-C2]`，spec 3.1。
- API/类型策略：合同形状不变，直接消费。
- 测试切入点：T-01 命令及相关 store tests。
- 假设：无产品假设。

## T-03 任务页初始化时序测试（RED）

- 状态：complete
- execute-readiness：ready
- 执行模式：串行，先于 T-04。
- 执行说明：用 fake timers 证明 mount 时可见但尚未订阅，并覆盖卸载取消。
- 触发/前置条件：T-02 已通过。
- 关联规格区域：3.2、5、AC-05、AC-06。
- 业务功能点：BF-02。
- 覆盖功能单元：首次呈现、延迟启动、快速离页。
- 页面/模块：TasksPage。
- 目标文件与符号：`TasksPage.test.ts::render` 与新增时序用例。
- 整洁性约束：断言真实页面 heading；对 spy 仅断言本应用副作用边界。
- 工程约束：mock 只替代外部 service/event source，真实组件与 store 保留。
- 副作用边界：测试结束恢复 real timers 并 unmount。
- 模式约束：复用现有 render helper，不新增测试专用生产 API。
- 来源：`[SRC-U2][CODE-C3]`，spec AC-05/AC-06。
- API/类型策略：double 满足现有 service/event interfaces。
- 测试切入点：`npm test -- src/pages/TasksPage.test.ts`。
- 假设：零延迟 timer 是已批准的非产品技术选择。

## T-04 延迟任务初始化（GREEN）

- 状态：complete
- execute-readiness：ready
- 执行模式：串行，依赖 T-03 RED。
- 执行说明：在 TasksPage 生命周期内添加可清理的后续任务调度。
- 触发/前置条件：T-03 已因订阅同步启动而失败。
- 关联规格区域：3.2、4 至 7。
- 业务功能点：BF-02。
- 覆盖功能单元：先绘制、后初始化、离页清理。
- 页面/模块：TasksPage + 既有 stores（仅调用，不修改）。
- 目标文件与符号：`TasksPage.vue` 的 `onMounted`、`onUnmounted`、局部 timer/启动函数。
- 整洁性约束：不触碰列表派生、排序、筛选、模板或样式。
- 前端专家约束：loading owner 不变；异步失败仍由 store 负责；用户入口/成功/失败态完整。
- 生产代码质量：timer handle 明确 nullable；无 magic ambient state；函数动词清楚。
- 代码评审预检：timer 在回调和卸载均释放；无离页后订阅。
- 副作用边界：全部留在 lifecycle；store 内订阅/快照顺序不变。
- 变更链审查：RouterLink -> route mount -> delayed callback -> store initialize -> dispose。
- 模式约束：Level 0 local direct code，禁止 composable/manager/factory。
- 来源：`[SRC-U2][CODE-C3][CODE-C4]`，spec 3.2。
- API/类型策略：无合同变化。
- 测试切入点：T-03 用例与既有 TasksPage 状态用例。
- 假设：无产品假设。

## T-05 验证、评审和交付

- 状态：complete
- execute-readiness：ready
- 执行模式：串行，依赖 T-01 至 T-04。
- 执行说明：运行门禁、浏览器走查、diff 自审并完成工件。
- 触发/前置条件：所有 RED/GREEN 循环完成。
- 关联规格区域：AC-07、AC-08、11。
- 业务功能点：BF-01、BF-02。
- 覆盖功能单元：完整回归。
- 页面/模块：前端全量、Rust settings、demo 浏览器。
- 目标文件与符号：不新增生产修改，除非验证发现本范围 blocker。
- 整洁性约束：changed hunks 映射任务；保留用户既有无关改动。
- 工程约束：记录所有适用 verdict 和剩余风险。
- 生产代码质量：typecheck/build/cargo compile 作为静态和集成证据。
- 代码评审预检：无 debug/dead/stale/unused/unrelated churn。
- 副作用边界：浏览器只读验证，不改持久化设置。
- 模式约束：确认未引入额外抽象。
- 来源：spec AC-07/AC-08。
- API/类型策略：确认 Settings JSON camelCase 合同未改变。
- 测试切入点：计划第 4 节全部命令和 demo 页面。
- 假设：仓库没有 lint script，因此 lint 为不适用。

## Iteration 7 状态

- [x] 记录用户复测与运行二进制哈希证据。
- [x] 新增缓存任务首帧失败测试。
- [x] 实现首帧 gate、绘制后初始化和双句柄 cleanup。
- [x] 升级到 `0.1.19` 并完成前端/Rust 回归。
- [x] 生成 MSI/NSIS、校验哈希并清理旧 `0.1.18` 安装器。
- [x] 完成复审与验证记录。

## Iteration 8 状态

- [x] T-06 RED：同一运行时的 store 初始化去重。
- [x] T-07 RED：`/transcribe` 到 `/tasks` 保持订阅连续。
- [x] T-08 GREEN：store 拥有可复用运行时会话，页面停止销毁共享订阅。
- [x] T-09 GREEN：应用根节点执行最终清理。
- [x] T-10 验证、复审并重新生成仅 NSIS 安装包。
