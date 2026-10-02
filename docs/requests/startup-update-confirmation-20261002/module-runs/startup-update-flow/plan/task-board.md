# 任务看板

| 任务 ID | 任务名称 | 状态 | 执行模式 | 执行说明 | 前置条件 | 规格区域 | 功能单元 | 范围 | 整洁性约束 | 模式边界 | 影响面 | 关键状态 | 类型策略 | 测试切入点 | 假设 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| T1 | 建立红灯测试 | completed | 串行 | 先测真实组件行为 | spec 批准 | AC-1~7 | 检查、弹窗、安装 | 新测试 | 不测私有变量 | 仅 mock 外部 port | settings/app | once、silent、pending、retry | 复用 ReleaseActionsPort | 聚焦 Vitest 红灯 | 无 |
| T2 | 实现启动更新确认 | completed | 串行 | 最小实现转绿 | T1 红灯 | 模块设计与 AC-1~7 | bridge、dialog、locale、style | app/settings | 协调与展示分离 | 保留 Adapter，无新模式 | App 与设置 UI | available、installing、error | strict TS、直接合同 | 聚焦测试 + typecheck | 无 |
| T3 | 完整回归与交付 | completed | 串行 | 跑全量门禁并审查 | T2 转绿 | AC-8 | 回归与证据 | 全前端 | 不掩盖失败 | 无新增抽象 | 手动/托盘邻接 | 不改变现有行为 | 合同不变 | test/typecheck/build/diff | 真实 relaunch 由发布冒烟 |
