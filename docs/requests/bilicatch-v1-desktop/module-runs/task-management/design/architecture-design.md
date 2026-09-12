# 架构设计：下载任务管理

## 交付单元标识

`task-management`

## 架构目标

建立下载任务的唯一权威状态源和可扩展执行宿主：Rust 负责创建、状态转换、调度、持久化、事件序列和文件副作用；Vue 负责订阅、合并、筛选和呈现。该边界必须让后续 `audio-download` 与 `video-download` 只实现执行器，不重新定义任务模型或任务页规则。

## 架构范围与触发因素

- 当前范围：任务草稿消费、100 项原子入队、状态机、默认并发 3、任务列表/控制 IPC、事件推送、JSON 持久化恢复、文件名净化、临时文件取消清理契约、任务页与系统打开操作。
- 当前不实现：B 站媒体流下载、断点 HTTP 细节、FFmpeg、认证能力和设置 UI；它们由后续模块通过既定端口接入。
- 架构敏感点：实时事件与初始快照竞态、终止态被迟到事件回退、重启恢复、跨平台文件写入、批量命令和后续两类执行器共享同一调度器。

## 上游输入与假设

- 复用已通过 review 的 `DownloadTaskDraft`；task-management 可扩展 handoff 元数据，但不改变草稿媒体字段含义。
- Rust `AppError`、Tauri `Result<T, AppError>`、共享 `IpcTransport`、Pinia/i18n 和 AppShell 保持不变。
- page-design 固定四个筛选、六列桌面表、状态相关操作、窄屏行布局和“活动任务在前”的稳定排序。
- 删除 completed 默认只删除记录，不删除输出文件；取消任务立即清理任务登记的临时文件。
- 当前 `task-drafts` 是会话交接，不是任务事实源；创建成功后由 TasksPage 清空已确认草稿。

## 模块边界设计

### 前端

- `features/task-management/contracts.ts`：Rust 稳定 DTO 的 TypeScript 翻译，保留 camelCase 字段与枚举，不含 B 站 raw 或执行器内部字段。
- `features/task-management/service.ts`：唯一 IPC/Event adapter，封装 exact command、Tauri reject 归一化和 event unsubscribe。
- `features/task-management/store.ts`：唯一页面状态源，拥有 snapshot hydration、事件缓冲/序号合并、派生筛选/计数、行级 busy 和操作委派；不调用 Tauri 包。
- `features/task-management/action-policy.ts`：以任务状态返回合法操作，是 UI 和 store 的单一操作矩阵；不产生副作用。
- `features/task-management/formatters.ts`：纯函数格式化 bytes、速度、ETA、百分比与安全文件名显示。
- `features/task-management/components/*`：工具条、表头/列表、任务行、状态、进度与确认弹窗；只接收 props/emits，不持有集合或 IPC。
- `pages/TasksPage.vue`：注入 service、启动订阅与 hydration、消费 pending drafts、连接 store 与路由/通知；不实现状态转换条件。
- `stores/task-drafts.ts`：保留 `items`/`append`/`clear` 兼容面，增加稳定 `handoffId`，使创建请求可幂等重试。

### Rust

- `models/task.rs`：公开序列化 DTO、任务状态/操作/事件/请求响应类型。
- `services/tasks/state_machine.rs`：合法 transition 与 action policy 的唯一 Rust 权威实现。
- `services/tasks/filename.rs`：文件名净化、200 字符限制、扩展名和 fallback 纯函数。
- `services/tasks/scheduler.rs`：并发槽位、queued/resume-requested 选择和 `TaskExecutorPort` 协作；默认并发 3，允许后续 settings 在 1-10 内更新。
- `services/tasks/manager.rs`：用一个协调锁串行化命令与 worker update，执行校验 -> 变更 -> 持久化 -> emit；维护全局 sequence、attempt 与幂等 request id，并通过 `TaskEventSink` 隔离 Tauri emit。
- `infrastructure/tasks/json_store.rs`：应用数据目录中的版本化 JSON、备份式原子替换与损坏恢复。
- `commands/tasks.rs`：Tauri command 参数解包和 AppHandle event/open 适配，不承载业务规则。
- `lib.rs`：在 Tauri `setup` 中用 app data 路径与 `TauriTaskEventSink` 创建并 manage 一个 `TaskManager`，注册 commands；启动时加载快照并执行恢复判定。

## 文件与目录结构

```text
src/
  features/task-management/
    contracts.ts
    service.ts
    injection.ts
    store.ts
    action-policy.ts
    formatters.ts
    task-management.css
    components/
      TaskToolbar.vue
      TaskList.vue
      TaskRow.vue
      TaskStatus.vue
      TaskProgress.vue
      TaskConfirmDialog.vue
  pages/TasksPage.vue
  stores/task-drafts.ts
  services/ipc/event-client.ts

src-tauri/src/
  models/task.rs
  commands/tasks.rs
  services/tasks/
    mod.rs
    manager.rs
    scheduler.rs
    state_machine.rs
    filename.rs
  infrastructure/tasks/
    mod.rs
    json_store.rs
```

测试与实现同目录；Rust 跨端 serde 契约放 `src-tauri/tests/task_contract.rs`，持久化 fixture 放 `src-tauri/tests/fixtures/`。

## 代码关系与依赖方向

```text
main composition
  -> TasksPage
      -> TaskStore -> action-policy / formatters
      -> TaskService -> IpcTransport + EventTransport
      -> TaskRow / Toolbar / ConfirmDialog
      -> TaskDraftsStore (handoff only)

Tauri commands
  -> TaskManager
      -> state_machine / filename / scheduler
      -> TaskStorePort -> JsonTaskStore
      -> TaskExecutorPort -> downstream audio/video executors
      -> TaskEventSink -> Tauri AppHandle.emit
  -> opener (validated-path adapter side effect)
```

- 组件不得导入 service、Pinia 或 Tauri；page 不解析事件序号和状态合法性。
- Rust models 不依赖 Tauri、文件系统或 reqwest；services 不依赖 Vue/Tauri command 参数。
- 后续执行器只通过 `TaskExecutorPort` 接收不可变 `TaskExecutionSpec` 并回报 `ExecutionUpdate`，不能直接修改持久任务集合。

## 职责切分

| 层 | 拥有 | 禁止 |
| --- | --- | --- |
| Vue page | 生命周期、通知、路由、确认结果 | 状态机、直接 invoke/listen |
| Pinia store | 快照、筛选、序号合并、busy | 持久化、文件副作用、伪造权威状态 |
| UI components | 语义呈现、用户意图 emits | 全局数据与命令调用 |
| TS service | exact IPC/event transport、错误归一化 | 业务 transition、展示格式化 |
| Rust manager | 任务事实、命令事务、幂等、emit 顺序 | 媒体下载算法 |
| Scheduler | 并发槽位与执行器协作 | 持久化格式、UI 状态 |
| Executor port | start/pause/resume/cancel 的执行边界 | 直接写任务库或 emit Tauri 事件 |
| JSON store | 版本化读写/恢复 | 业务状态判断 |

## 函数设计与公开入口

### IPC commands

- `list_download_tasks() -> TaskListSnapshot`
- `create_download_tasks({ request }) -> CreateTasksResult`
- `control_download_task({ request }) -> DownloadTask`
- `pause_all_download_tasks() -> BatchTaskResult`
- `clear_completed_tasks() -> ClearTasksResult`
- `open_download_task({ request }) -> ()`

`CreateDownloadTasksRequest` 包含 `requestId` 与 `drafts`；同一 requestId 重试返回原 task ids，不重复创建。批量创建在容量、每条草稿和文件名全部验证后一次提交，任一失败则整批不写入。

### Event

- 保留 PRD 固定进度事件名 `download://progress`，payload 为 `TaskProgressEvent { sequence, task }`；完整 task 快照包含任务 ID、进度百分比、下载速度、剩余时间和已下载字节数，也承载状态变化。
- 删除记录使用 `download://removed`，payload 为 `TaskRemovedEvent { sequence, taskId }`。两个事件共享 manager 的单调 sequence。
- 事件总是在成功持久化之后发出；持久化失败不暴露未提交状态。

### 纯函数与服务入口

- `allowed_actions(status, control) -> &[TaskAction]`
- `transition(current, intent/update) -> Result<NextState, AppError>`
- `sanitize_filename(title, extension) -> String`
- `mergeSnapshot(snapshot)`、`acceptEvent(event)`、`visibleTasks(filter)` 为 Pinia 明确入口。
- `TaskManager::apply_executor_update(task_id, attempt_id, update)` 是 worker 唯一回写入口。

## 状态归属与数据流

1. `DownloadPage` 生成草稿并写入 session handoff；TasksPage 先订阅事件，再请求快照，同时缓冲事件。
2. 快照返回后 store 以快照为基线，只重放 `sequence > snapshot.sequence` 的缓冲事件，随后进入 live merge；解决订阅与读取竞态。
3. 若存在 drafts，TasksPage 使用稳定 handoffId 调 `create_download_tasks`；成功后清空对应 handoff，失败保留供重试。
4. Rust manager 校验总记录数不超过 100、净化文件名、创建 queued 任务、持久化，然后 emit upsert。
5. Scheduler 只在有已注册 executor 和空闲槽位时把 queued 交给执行器；未接入音视频执行器时 production 任务保持 queued，不伪造下载。
6. 每次 start/retry/resume 创建新 `attemptId`；executor update 必须匹配当前 attempt，状态机再验证 transition。取消、失败或完成后的迟到 update 被忽略。
7. progress update 在同一 attempt 内只允许 bytes/progress 单调增加；状态变化立即持久化/emit，普通进度最多每秒持久化一次、每 250ms emit 一次最终快照。

## 数据结构与类型策略

```text
DownloadTask
  id, revision, createdAt, updatedAt
  fileName, outputDir, outputPath?
  bvid, cid, page, partTitle, mode
  qualityId?, codec?, audioFormat?, audioBitrateId?
  status, controlRequest
  progressPercent, bytesDownloaded, totalBytes?
  speedBytesPerSecond, etaSeconds?
  error?

TaskListSnapshot
  sequence, capacity=100, tasks[]

PersistedTaskFile
  schemaVersion=1, sequence, tasks[] (internal sourceRequestId retained per task)
```

- `TaskStatus` 固定为 `queued | downloading | paused | processing | completed | failed | cancelled`。
- `TaskControlRequest` 为 `none | pauseRequested | resumeRequested | cancelRequested`，用于表达等待执行器确认，不创造伪状态。
- `TaskAction` 固定为 `pause | resume | cancel | retry | delete`；打开文件/目录使用独立 `OpenTarget` 与可信路径 command，不属于状态转换意图。
- 执行元数据包含 `automaticRetryCount` 与 `connectionCount`；connectionCount 当前默认 8、约束 1-32，后续 settings 提供来源。
- 时间为 RFC3339 字符串；byte/秒字段在 Rust 使用 `u64`，为避免 JS 精度风险，跨端序列化为十进制字符串，由 formatter 转为显示；百分比为 0-100 的整数。
- `attemptId`、temp paths、executor checkpoint 和 `sourceRequestId` 只在 Rust persisted/internal model，不暴露给 Vue；幂等查询复用仍存在的同 request batch。
- 任务文件 schema 带版本号；未知新版本拒绝覆盖并返回可观察错误，损坏主文件尝试 `.bak`，两者都坏时不静默清空。

## 状态机与调度规则

- 创建：draft -> queued。
- 调度：queued -> downloading；后续执行器可在下载流完成后回报 downloading -> processing -> completed，或 downloading -> completed（仅视频无需处理时）。
- 暂停：downloading -> pauseRequested -> paused，仅执行器确认后改变 status。
- 恢复：paused 保持状态并标记 resumeRequested；获得并发槽位并启动新 attempt 后直接 paused -> downloading。
- 取消：queued/downloading/paused/processing -> cancelled；manager 先使旧 attempt 失效，执行器/清理端口删除登记的临时文件，清理失败返回/记录错误且不得宣称取消完成。
- 失败：queued/downloading/processing 可进入 failed；retry 使 failed -> queued 并清空瞬时速度/ETA/error，保留已知可恢复进度/checkpoint。
- retryable 网络错误由 manager 最多自动重试 3 次，退避固定为 1s、2s、4s；每次自动重试创建新 attempt，耗尽后才进入 failed。非网络错误不套用该策略。
- 删除：只允许 failed/completed/cancelled；从持久记录移除并 emit remove。completed 的输出文件不删除。
- Scheduler 以创建时间 FIFO 选 queued 和 resumeRequested；默认最多 3 个 downloading/processing executor，占位设置接口限制 1-10。
- 100 上限按全部未删除记录计算；批量加入超出剩余容量时整批拒绝，`AppError.details=TASK_QUEUE_FULL`。

## 契约与 Adapter 边界

- Rust serde DTO/field table 是 task IPC 的权威来源；TS 保留同名 camelCase 字段，并由 `task_contract.rs`/TS exact request tests 双向证明。
- `TaskService` 是 Tauri invoke/listen adapter；组件和 store 不消费原始 event 或 reject。
- `TaskExecutorPort` 是后续音频/视频执行策略的唯一扩展边界；当前用 deferred production implementation 和 controllable fake tests。
- `TaskStorePort` 只为 JSON persistence 与内存测试替身隔离 I/O；不构建通用 CRUD repository。
- `TaskEventSink` 将 Tauri emit 与 manager 解耦，测试使用 recording sink 校验“持久化后发事件”和序号；它不扩展为应用内通用 Event Bus。
- `open_download_task` 由 Rust 根据 task id 找到受信 output path 再调用 opener，前端不能提交任意路径。

## Pattern 决策与拒绝项

- **State**（函数式状态表）：解决七状态与操作矩阵在 Rust/UI 多点分支会漂移的问题。Rust 表为权威，TS action-policy 镜像用于即时呈现并以契约测试对齐；若状态始终只有两三项，直接分支更简单，但当前不是该情形。
- **Adapter/Port**：`TaskStorePort` 隔离文件系统，`TaskExecutorPort` 隔离音频/视频实现，TS service 隔离 Tauri。真实变化轴分别是测试/文件系统、两类后续执行器、浏览器 demo/Tauri；比直接散落 side effect 更可替换。
- **Observer**：Tauri `download://progress`/`download://removed` 是必要的跨线程实时更新，必须通过 unsubscribe、全局 sequence、buffer/replay 和 250ms 合并控制生命周期与顺序。
- **Command（轻量枚举）**：`TaskAction` 把 pause/resume/cancel/retry/delete 作为明确意图传入同一控制入口，减少重复 commands；不创建 class hierarchy 或 command objects。
- 拒绝通用 Repository、Event Bus、Redux-style reducer、DI container、TaskManager 前端类和每状态 Strategy 类；这些不会减少当前复杂度。

## 可读性与维护护栏

- 状态 transition、action policy、进度归一化和 filename 净化各只有一个 Rust 权威函数；TS 镜像只服务同步 UI，必须由共享矩阵测试证明一致。
- manager 的每个命令按 validate -> derive -> persist -> emit 四段组织，不在持锁区等待 executor 网络/媒体操作。
- 文件建议 250 行进入拆分审查，超过 350 行必须拆分；Vue page 不超过约 180 行，TaskRow 不包含集合筛选或 IPC。
- 不允许 `any`、任意字符串状态、模板内复杂状态矩阵、组件直接 `listen/invoke`、事件到达即整表替换。
- 不为未来多窗口、云同步、优先级或可重排队列增加字段/抽象。

## 架构风险

- Windows JSON 替换不是天然 POSIX 原子 rename；实现必须使用 `.next` + `.bak` 恢复协议并测试中断点，不能先删主文件后无备份写入。
- 每秒持久化会丢失崩溃前不足一秒的进度，但不会丢失任务/状态转换；这是性能与恢复精度的可接受权衡。
- production deferred executor 意味着本模块独立交付时新任务保持 queued；必须在 UI/验证中诚实呈现，并由后续 audio/video 模块完成实际执行。
- 后续 settings 更新并发数时，降低上限不得强制中断已运行任务，只影响后续槽位分配。
- 临时文件清理失败不能把任务错误地标成 cancelled；需保留 failed 与可重试清理信息。

## 开放架构问题

- `processing` 取消：为满足取消清理语义，架构允许 `processing -> cancelled`，但后续 FFmpeg executor 必须先可靠终止子进程再确认；本模块 fake executor 证明协议，真实实现由 video/audio 模块验收。
- 断点 checkpoint 的具体字段由下载执行器私有化，task-management 只持久化 opaque JSON/value 或 executor-owned sidecar 引用；规格不得猜测 HTTP 分片结构。
- 当前并发固定为 3；settings 模块接入后通过 manager 的受限 setter 更新，持久设置的来源仍由 settings 拥有。
