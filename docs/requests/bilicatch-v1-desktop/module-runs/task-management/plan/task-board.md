# 任务板：下载任务管理

## 执行规则

- 顺序固定：TASK-01 -> TASK-02 -> TASK-03 -> TASK-04 -> TASK-05 -> TASK-06 -> TASK-07 -> TASK-08 -> TASK-09。
- 状态域：`pending | in_progress | completed | blocked`；同一时间仅一个任务可为 `in_progress`。
- 每项必须完成 red/green、定向门禁和执行记录后才改 `completed`。
- 执行说明：由主 agent 串行执行；不启用 workflow/subagent。仓库非 Git，不创建虚假 commit 检查点。

## 总览

| ID | 名称 | 状态 | 模式 | 前置 |
| --- | --- | --- | --- | --- |
| TASK-01 | 双端任务契约与 TS 上下文 | completed | 串行 | 计划批准 |
| TASK-02 | 状态机、策略、校验与文件名 | completed | 串行 | TASK-01 |
| TASK-03 | JSON 持久化与恢复 | completed | 串行 | TASK-02 |
| TASK-04 | Manager、Scheduler、重试与事件 | completed | 串行 | TASK-03 |
| TASK-05 | Tauri Commands/Event/Opener | completed | 串行 | TASK-04 |
| TASK-06 | TS Adapter 与草稿交接 | completed | 串行 | TASK-05 |
| TASK-07 | Pinia Hydration 与视图状态 | completed | 串行 | TASK-06 |
| TASK-08 | 任务页、组件、样式与 a11y | completed | 串行 | TASK-07 |
| TASK-09 | 集成验证与证据 | completed | 串行 | TASK-08 |

## TASK-01

- 任务名称：双端任务契约与 TypeScript 上下文
- 状态：completed
- 执行模式/说明：串行；先恢复 governing tsconfig/声明闭包，再 TDD 固定合同。
- 触发/前置：计划批准。
- 规格区域：API 与数据合同；AC-TASK-01/04/08/17。
- 功能单元：Rust serde DTO、TS contracts、request/response/event 字段与 enum/null/byte string。
- 页面/模块：`models/task.rs`、`task_contract.rs`、`features/task-management/contracts.ts`。
- 整洁性：字段同名 camelCase；internal 字段不进 public DTO。
- 模式/边界：Rust contract source -> TS direct translation；无 UI mapper。
- 上下文/影响：现有 AppError、DownloadTaskDraft、IpcTransport、strict/no alias。
- 交互/状态：无 UI；只冻结七状态和事件类型。
- API/类型：serde 为权威，无 backend TS/protobuf；TS 保留字段名。
- 测试切入：serde exact JSON、TS fixture、typecheck。
- 已批准假设：sequence/revision 为 session 安全整数；u64 byte/speed 为十进制字符串。

## TASK-02

- 任务名称：领域状态机、操作策略、校验与文件名
- 状态：completed
- 执行模式/说明：串行；纯函数 red/green。
- 触发/前置：TASK-01 completed。
- 规格区域：行操作、容量/文件名、边界；AC-TASK-02/03/04/07/10/12。
- 功能单元：transition、allowed actions、progress guard、request/draft validation、filename、TS policy/formatters/sort。
- 页面/模块：Rust `services/tasks/*`；TS action-policy/formatters。
- 整洁性：每类规则单一 owner；模板无重复 switch。
- 模式/边界：函数式 State；拒绝状态类/Strategy 层。
- 上下文/影响：复用 TASK-01 enums；不触及 I/O。
- 交互/状态：pause/resume 先 controlRequest；terminal/attempt/progress guard。
- API/类型：requestId/outputDir 提交时规范化；Unicode 200 字符含扩展名。
- 测试切入：全矩阵表、99+2 校验、非法字符/空名/保留名、BigInt formatter。
- 已批准假设：processing 仅 executor 可中断时 cancel；completed 删除不删输出。

## TASK-03

- 任务名称：版本化 JSON 持久化与恢复
- 状态：completed
- 执行模式/说明：串行；tempdir 故障注入。
- 触发/前置：TASK-02 completed。
- 规格区域：持久化与恢复；AC-TASK-07/09/10/17。
- 功能单元：schema v1 load/save、next/bak、损坏/未知版本、启动 transform、flush。
- 页面/模块：Rust infrastructure/tasks、fixtures、Cargo dev dependency。
- 整洁性：JSON store 只处理 I/O/schema，不判断 UI/action。
- 模式/边界：小 TaskStorePort；拒绝通用 Repository。
- 上下文/影响：app data path 在 TASK-05 注入；此任务只接收显式 PathBuf。
- 交互/状态：downloading/processing 恢复 queued；paused/terminal 保持；瞬时字段清空。
- API/类型：PersistedTaskFile internal，不跨 IPC。
- 测试切入：roundtrip、primary corrupt/backup valid、双 corrupt、unknown schema、replace failure。
- 已批准假设：崩溃前不足 1 秒进度可丢，状态变化不可丢。

## TASK-04

- 任务名称：TaskManager、Scheduler、重试与事件顺序
- 状态：completed
- 执行模式/说明：串行；in-memory ports + fake clock/executor。
- 触发/前置：TASK-03 completed。
- 规格区域：执行/控制流程；AC-TASK-01/02/04/05/06/07/08/10/13。
- 功能单元：事务、幂等、FIFO/并发、attempt、retry、pause/resume/cancel、batch、coalescing。
- 页面/模块：Rust manager/scheduler/ports。
- 整洁性：validate -> derive -> persist -> emit；锁内不 await executor。
- 模式/边界：Executor/Event/Cleaner Ports；Observer 只在固定 sink。
- 上下文/影响：后续 audio/video executor 的稳定宿主；无媒体算法。
- 交互/状态：initial + 3 retries，1/2/4 秒；save-before-emit；cleanup failure -> failed。
- API/类型：manager 产出 TASK-01 DTO；sourceRequestId internal。
- 测试切入：第 4 项等待、stale attempt、retry fake time、batch partial/all-or-none、cadence。
- 已批准假设：无 production executor 时 queued；默认 3/8。

## TASK-05

- 任务名称：Tauri Commands、事件与可信路径 Opener
- 状态：completed
- 执行模式/说明：串行；薄 adapter 集成。
- 触发/前置：TASK-04 completed。
- 规格区域：commands/events/opener；AC-TASK-08/09/13/14/17/18。
- 功能单元：六 commands、两个 event、setup/manage、app data、trusted open。
- 页面/模块：commands/tasks、Tauri event sink、lib.rs、capability only if required。
- 整洁性：command 不含业务；AppHandle 不进 domain。
- 模式/边界：Tauri Adapter；不接受 arbitrary path。
- 上下文/影响：保留 ParserService/parse_video/opener plugin/default capability。
- 交互/状态：setup load failure 可观察；event 在 persistence 后。
- API/类型：nested `{ request }`；exact `download://progress`/`download://removed`。
- 测试切入：registration、serde wrapper、completed/existing trusted path、invalid state/path。
- 已批准假设：opener 使用已安装 v2 API；无权限增量时不改 capability。

## TASK-06

- 任务名称：前端 Service/Event Adapter 与草稿幂等交接
- 状态：completed
- 执行模式/说明：串行；exact transport TDD。
- 触发/前置：TASK-05 completed。
- 规格区域：初始化/API/handoff；AC-TASK-01/08/14/15/17/18。
- 功能单元：Ipc/Event adapters、injection、DEV demo、handoffId。
- 页面/模块：event-client、feature service/injection/demo、task-drafts、main。
- 整洁性：invoke/listen 只在 adapter；main 仅 composition。
- 模式/边界：Tauri Adapter/Observer lifecycle；不建全局 Event Bus。
- 上下文/影响：保持 task-drafts `items/append/clear` 兼容 DownloadPage。
- 交互/状态：listener 部分创建失败回收；create 成功后才能清 handoff。
- API/类型：直接消费 TASK-01 contracts；normalizeIpcError。
- 测试切入：六 exact commands、两个 event、unsubscribe、stable id、clear mismatch、demo dev isolation。
- 已批准假设：requestId 1-80 安全字符；相关任务存在期间幂等。

## TASK-07

- 任务名称：Pinia Hydration、事件合并、筛选与命令状态
- 状态：completed
- 执行模式/说明：串行；fake promise/event TDD。
- 触发/前置：TASK-06 completed。
- 规格区域：初始化、筛选、错误、pending；AC-TASK-01/02/07/08/11/12/13/15。
- 功能单元：subscribe-buffer-snapshot-replay、byId/order、filter/count、commands/errors/capacity/dispose。
- 页面/模块：task-management/store.ts/tests。
- 整洁性：store 唯一页面状态源；upsert 单项，不整表深拷贝。
- 模式/边界：Observer sequence；不 optimistic 推测 status。
- 上下文/影响：service/event/handoff；页面只调用 actions/getters。
- 交互/状态：idle/subscribing/loading/ready/failed；pending finally 清理。
- API/类型：response/event 统一 merge；queue-full details 只映射展示 key。
- 测试切入：event 12 vs snapshot 10、duplicate 11、remove、subscribe/list failure、dispose、stable sort。
- 已批准假设：下载中为四类 active；cancelled 只在 all。

## TASK-08

- 任务名称：完整任务页、组件、样式与可访问交互
- 状态：completed
- 执行模式/说明：串行；component/page TDD + visual。
- 触发/前置：TASK-07 completed。
- 规格区域：page/function complete；AC-TASK-02/10/11/12/13/15/16/18。
- 功能单元：toolbar/tabs、list/row/status/progress、confirm/menu、loading/empty/error、locale/CSS。
- 页面/模块：TasksPage、task components/style、main CSS、zh/en locales。
- 整洁性：组件 props/emits；TaskRow 不导入 store/service；无卡片嵌套。
- 模式/边界：action policy 决定按钮；Lucide + tooltip；Naive dialog/menu 可复用。
- 上下文/影响：复用 AppShell/EmptyState/AppIconButton/tokens；不改 router。
- 交互/状态：确认初始取消焦点与归还；busy 稳定；错误不隐藏已有列表。
- API/类型：UI 只消费 store stable fields；无 raw event/invoke。
- 测试切入：七状态按钮、四 filter、列/空值、focus/ARIA、long text、1280/900/800 截图。
- 已批准假设：active-first/createdAt desc；打开两个命令收进小菜单。

## TASK-09

- 任务名称：跨端集成、验收矩阵与最终门禁
- 状态：completed
- 执行模式/说明：串行收口；失败回 owner task。
- 触发/前置：TASK-08 completed。
- 规格区域：AC-TASK-01..18 全部。
- 功能单元：full test/typecheck/build/fmt/check/test、定向风险、静态扫描、视觉、verification。
- 页面/模块：全 task scope + verification/evidence/changelog/state。
- 整洁性：证据与 claim 对应；warning/skip 不伪装 pass。
- 模式/边界：review 检查 State/Port/Observer/Command 是否仍最轻。
- 上下文/影响：回归 DownloadPage/AppShell/ParserService/parse contract。
- 交互/状态：只有全部 pass 才 verify -> review；失败立即 re-enter execute。
- API/类型：exact command/event/serde/TS contract 全链证据。
- 测试切入：前端全量、Rust 全量、高风险清单、静态 import、六视图截图/a11y。
- 已批准假设：demo 只证明 UI，Rust tests 证明 native；真实下载保持 queued 属范围声明。
