# Task Board

| ID | 任务名称 | 状态 | 执行模式 | 执行说明 | 触发/前置条件 | 规格区域 | 功能单元 | 页面/模块范围 | 整洁性约束 | 模式/抽象边界 | 代码上下文 | 状态约束 | API/类型策略 | 测试切入点 | 待确认项/假设 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| AUDCOV-01 | TDD 修复封面适配 | completed | 串行 | 主执行路径；先红后绿 | 计划批准 | AC-01~05 | HTTP/HTTPS cover、unsafe URL、source resolve | Rust Bilibili adapter/parser | 一个 helper、一个 rule owner | 复用 Adapter 与 validator；无新模式 | 单一 production caller 已确认 | 成功返回 bundle；不安全输入 E004 | crate-private `Result`；IPC/TS 不变 | adapter/parser focused tests | 已批准：只升级受信任 HTTP 封面 |
| AUDCOV-02 | 公网与全量验证评审 | completed | 串行 | AUDCOV-01 GREEN 后执行 | AUDCOV-01 completed | AC-04~07 | live smoke、全量 gate、合规与 review | client test + request docs | 测试不打印 URL/token | 不改生产 request/executor | 公网易变，fixture 权威 | verify fail/review blocker 回 execute | Bilibili raw fields 保持 | ignored smoke、全量 Rust/前端/typecheck/fmt | 已批准：公网失败单独记录 |

## 状态枚举

- `pending`：尚未开始。
- `in_progress`：正在按任务步骤执行。
- `completed`：任务完成且对应验证通过。
- `blocked`：存在需要外部输入的真实阻断。

## 执行顺序

`AUDCOV-01 -> AUDCOV-02`
