# 任务看板：应用基础、壳层与共享契约

| ID | 任务 | 状态 | 模式 | 执行说明 | 前置条件 | 规格区域 | 功能单元/范围 | 整洁性与 Pattern 约束 | 上下文/影响面 | 关键交互与状态 | API/类型策略 | 测试切入点 | 待确认/批准假设 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| FND-01 | 官方脚手架与上下文恢复 | completed | 串行 | 主执行路径 | 计划批准 | Bootstrap、TS 上下文 | 根配置、src、src-tauri、code-context | 必须官方 scaffold；不手写替代 | 全新工程 | 无业务交互 | 读取真实 tsconfig/Cargo/Tauri | scaffold build、配置检查 | npm；兼容版本优先 |
| FND-02 | 契约、IPC Adapter 与 Rust 健康命令 | completed | 串行 | TDD | FND-01 | API、错误、边界 | contracts、IPC、models、commands | Adapter + composition；无 UI 副作用 | 双端共享契约 | health 成功/失败 | Rust DTO + TS contract，同字段 | 错误归一、transport、序列化 | E_INTERNAL 为系统兜底 |
| FND-03 | Router、状态、主题、i18n | completed | 串行 | TDD | FND-02 | 路由、主题、语言 | router、app store、locales | 单一来源；直接枚举分支 | 前端核心状态 | hash/404、三态主题、双语 | ThemePreference/AppLocale | 路由、matchMedia、locale | 本模块不持久化 |
| FND-04 | AppShell、Sidebar、TopBar | completed | 串行 | TDD + 视觉 | FND-03 | 页面/交互设计 | layout/shared/styles | 展示组件不 invoke；Lucide | 所有路由壳层 | 导航、返回、登录、设置、响应式 | 无新增后端契约 | 组件、ARIA、多宽度 | 使用占位品牌图标 |
| FND-05 | 占位页、健康与错误集成 | completed | 串行 | TDD | FND-04 | 启动/错误/页面 | pages、App、providers | 页面编排与 transport 分离 | 应用组合入口 | loading/healthy/failed/retry/404 | 只经 app IPC service | 集成测试、静态 invoke 扫描 | 占位页不伪造业务 |
| FND-06 | 门禁、运行与视觉验收 | completed | 串行 | 验证与修复 | FND-05 | AC-FND-01..12 | 全模块 | 扫描重复、any、隐藏副作用 | 全模块与执行产物 | 多路由/主题/宽度/IPC | 契约形状比对 | test/build/cargo/截图 | 环境缺失须如实记录 |

## 状态约定

- `pending`：未开始。
- `in_progress`：当前唯一活动任务。
- `completed`：实现及任务内测试完成。
- `blocked`：真实外部门禁阻塞，并写明原因。

执行阶段必须按 FND-01 至 FND-06 顺序逐项更新，不能一次性全部标记完成。
