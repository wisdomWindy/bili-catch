# 代码上下文

## 上下文需求

确认转写默认语言从持久化设置到表单的完整来源，并确定任务页首次绘制前执行的数据初始化链路。

## graph 可用性检查

仓库未包含 `.agents` 图索引或可直接查询的 code graph 工具。已通过 `npx --package=@sourcegraph/scip-typescript scip-typescript` 成功确认 SCIP 索引器可启动，但当前环境没有索引查询消费者，因此不能形成可用图查询闭环。

## 安装或 bootstrap 记录

执行了临时 `scip-typescript` bootstrap 能力检查，没有向项目写入新依赖。由于缺少图查询消费者，后续使用文本检索和导入链人工核对。

## fallback 记录

使用 `rg`、定向源码阅读、本机设置持久化文件和既有测试重建调用链。

## 相关入口点

- 应用设置加载：`SettingsManager::load`
- 旧设置迁移：`migrate_document`
- 转写表单：`TranscribePage.vue`
- 侧边栏导航：`Sidebar.vue` 中的 `RouterLink`
- 任务页挂载：`TasksPage.vue` 的 `onMounted`

## 关键符号与调用链

语言默认值：

`SettingsDefaults::values` -> `migrate_document` 缺省字段回退 -> `SettingsService` -> `useSettingsStore` -> `TranscribePage` watcher -> `form.language`

任务页加载：

`RouterLink` -> Vue Router 创建 `TasksPage` -> `onMounted` -> `taskStore.initialize` / `transcriptionStore.initialize` -> 5 个 Tauri `listen` 调用 -> 两类任务快照读取

## 根因证据

1. `SettingsDefaults::values` 和 `TranscribePage` 表单初始值均为 `auto`。
2. 本机旧版设置文件没有语言字段，`migrate_document` 使用上述默认值补齐，因此页面稳定显示“自动检测”。
3. `TasksPage` 在 `onMounted` 同一渲染周期立即启动两个 store 初始化；事件源依次等待 2 个下载事件和 3 个转写事件监听注册，浏览器在挂载回调让出执行权之前不能完成首次绘制。
4. 路由没有守卫、异步组件或过渡阻塞，侧边栏使用普通 `RouterLink`，因此延迟来源集中在目标页面挂载工作而不是路由解析。

## 依赖与副作用边界

- 默认语言仍允许用户在设置页和转写页改为其他合法选项。
- 已经持久化的 schema v2 合法语言值由反序列化直接保留。
- 延迟任务初始化必须能在页面快速离开时取消，避免离页后创建订阅。
- store 的订阅优先于快照读取规则保持不变，避免初始化期间遗漏事件。

## 影响范围

直接改动仅涉及默认值提供者、任务页加载调度和定向测试；不修改 IPC 合同、任务模型或界面布局。

## 开放后续检查项

- 用失败测试确认 schema v1 缺省语言回退为 `zh`。
- 用假定时器确认任务页初次渲染时尚未调用订阅，下一调度周期再开始初始化。
- 验证显式 `auto` 设置仍然可保存和恢复。

## 复测根因补充（2026-10-11）

`TasksPage` 虽然通过零延迟定时器推迟了 `initialize`，但 `allItems`、`counts` 和任务行仍会在首次渲染时读取两个 Pinia store 的既有缓存。转写 store 的 `tasks` getter 还会同步排序全部缓存任务，因此“延迟 IPC”并不等价于“首帧不处理数据”。

新调用时序应为：`RouterLink -> TasksPage 首帧结构/skeleton -> browser paint -> 开放缓存投影 -> store initialize -> snapshot/list render`。

部署证据同时显示正在运行的安装版与当前 release 哈希不同；必须使用新版本号生成安装器，避免 `0.1.18` 对 `0.1.18` 的同版本覆盖不确定性。

## Iteration 8 根因证据

- 当前安装程序报告版本为 `0.1.19`，因此本次复测问题不能再归因于旧版 `0.1.18`。
- `TranscribePage.onBeforeUnmount` 在目标页呈现前调用两个共享 store 的 `dispose`，合计触发 5 个 Tauri 事件监听卸载。
- `TasksPage` 随后又调用两个 `initialize`，重新注册同一批监听并重新读取两类快照。
- 上一轮首帧 gate 仅延迟了目标页初始化，没有移除源页面销毁订阅这一关键路径工作。
- 修正后的 owner：Pinia store 拥有共享运行时会话，页面只拥有视图调度，`App.vue` 拥有应用结束时的最终清理。
