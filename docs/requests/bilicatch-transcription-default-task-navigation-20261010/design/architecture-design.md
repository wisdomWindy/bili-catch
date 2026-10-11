# 架构设计：共享任务运行时生命周期

## 决策

ADR-01 将下载任务与转写任务订阅的生命周期从路由页面提升到应用生命周期。两个页面消费同一组 Pinia store、service 与 event source，页面卸载时清理会产生无意义的 IPC 抖动并阻塞导航。

## 所有权与数据流

- `TaskManagementStore` 拥有订阅、初始化 Promise、快照、事件缓冲和命令状态。
- `TranscriptionStore` 拥有订阅、初始化 Promise、任务/模型快照、事件缓冲和命令状态。
- `TranscribePage` 与 `TasksPage` 触发懒初始化并渲染 store，不拥有共享订阅。
- `TasksPage` 继续拥有并取消 presentation frame 与零延迟 timer。
- `App.vue` 是最终清理 owner，在根节点卸载时 dispose 两个共享 store。

数据流保持为 `Tauri event/snapshot -> feature store -> page projection`。IPC 合同、DTO、路由和可见 UI 均不改变。

## 依赖规则

- 页面可依赖 feature store 与注入的 service 合同。
- 应用根节点只为生命周期清理依赖 feature store。
- Store 不得反向依赖页面或应用根节点。
- 初始化会话注册表留在各 store 模块内部，不新增公共 manager、facade 或跨 feature 抽象。

## 初始化合同

同一 store 实例、service 与 event source 组合在 hydration 期间返回已有 Promise；ready 后继续复用有效订阅。没有有效订阅的失败会话允许重试。`dispose` 使会话失效、移除注册项并调用当前 unsubscribe。

## 复用与 Anti-DRY 决策

两个 store 生命周期规则相似，但事件缓冲、快照结构、局部订阅失败语义与状态模型不同。真实生产 owner 仅两处且变因超过三个，因此各自保留小型私有注册表，不抽取泛型生命周期管理器。

## 复杂度与迁移

预算：不新增生产文件或公共 API；每个 store 一个私有 `WeakMap`；删除两处页面清理；增加两处根清理。无需持久化迁移或 feature flag，回滚面仅限上述生命周期代码。

## 验证

- Store 测试使用延迟快照覆盖 pending/ready 去重。
- 真实 router/page 集成测试覆盖 `/transcribe` 到 `/tasks` 的订阅连续性。
- App 生命周期测试覆盖根卸载清理。
