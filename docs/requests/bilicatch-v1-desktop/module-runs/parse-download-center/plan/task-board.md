# 任务看板：解析与下载中心

| ID | 任务 | 状态 | 模式 | 执行说明 | 前置条件 | 规格区域 | 功能/页面范围 | 整洁性与 Pattern 约束 | 上下文/影响面 | 关键交互与状态 | API/类型策略 | 测试切入点 | 待确认/批准假设 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| PARSE-01 | 上下文、契约与失败测试 | completed | 串行 | TDD | 计划批准 | DTO/API/边界 | contracts、fixtures、models | 稳定 DTO；无 raw TS | strict TS + Rust serde | 无 UI | Rust DTO + TS contract | shape/command/draft red tests | task draft 不含任务状态 |
| PARSE-02 | 输入规范化与 SSRF 防护 | completed | 串行 | TDD | PARSE-01 | 输入/安全/错误 | input guard、Rust normalizer | 权威 Rust + 粗 guard；无 contains host | URL/redirect | 空白/粘贴/drop/指定P | `{input}`；E003/E004 | 五类输入/恶意 host/5跳 | trim 后提交；内部空白非法；不静默截断 |
| PARSE-03 | B 站 Adapter、WBI 与 command | completed | 串行 | TDD + 外部研究 | PARSE-02 | Rust/API/cache | bilibili infrastructure、service、command | Adapter + HttpPort/Clock；无 manager | Cargo/lib.rs/API 变化 | cache、单次 retry、匿名能力 | 私有 raw -> stable DTO | fixtures/TTL/error/cargo | 公网 smoke 可受外部状态影响 |
| PARSE-04 | 前端 service/store/选择规则 | completed | 串行 | TDD | PARSE-03 | 状态/选择/cache | feature store/service、draft store | 派生 getter；service 无 UI | Pinia/IPC | 竞态、默认/全选/反选、模式清理 | stable DTO only | state/token/cache/mode | 无可用 option 时不伪造 |
| PARSE-05 | 下载中心完整 UI | completed | 串行 | TDD + 视觉 | PARSE-04 | 页面/UI/a11y | DownloadPage + 5 components/styles | 展示组件无 invoke；token/Lucide | 复用 AppShell/Naive | input/loading/error/result/form | consume store only | component/ARIA/paste/drop | 目录更改明确禁用，非伪按钮 |
| PARSE-06 | 草稿交接与跳转 | completed | 串行 | TDD | PARSE-05 | enqueue/handoff | page orchestration、task-drafts、router | store 不是任务 Repository | TasksPage 邻接但不实现 | enqueueing/notify/tasks/login | DownloadTaskDraft | 字段互斥/去重/导航 | 下游 task module 拥有 id/status |
| PARSE-07 | 门禁、视觉与 smoke | completed | 串行 | 验证与修复 | PARSE-06 | AC-PARSE-01..14 | 全模块 | scan 边界/敏感信息/重复规则 | 全改动面 | 三宽度/键盘/所有状态 | DTO/fixture/smoke 对照 | npm/cargo/browser/smoke | 外部 smoke 失败单列，不伪造通过 |

## 状态约定

- `pending`：未开始。
- `in_progress`：当前唯一活动任务。
- `completed`：实现、任务内测试与边界检查完成。
- `blocked`：真实外部门禁阻塞并记录原因。

任务按 PARSE-01 至 PARSE-07 严格顺序推进；不得一次性批量完成或跨越批准边界。
