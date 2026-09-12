# 工程规格：下载任务管理

## 交付单元标识

`task-management`

## 背景与目标

解析模块已经能生成 session-only `DownloadTaskDraft[]` 并跳转 `/tasks`，但任务页仍为空态，Rust 也没有权威任务集合、状态机、事件、调度或恢复。本模块交付可操作的任务控制台与统一任务宿主，使后续音频/视频执行器只需接入既定端口。

目标是：任务创建与操作在 Rust 中一致、可持久恢复；前端能正确处理实时进度、筛选、批量命令和错误；队列 100 项、默认并发 3、网络重试与取消清理规则具备自动化证据。

## 范围内

- `/tasks` 四筛选、任务列表/行、状态/进度/速度/ETA/大小、行操作、批量操作、加载/空/错/确认状态。
- 从 `useTaskDraftsStore` 幂等创建任务，批量容量校验与跨平台文件名净化。
- Rust 权威任务状态机、默认并发 3 的 scheduler、执行器端口和 fake executor 验证。
- `download://progress` 与 `download://removed` 事件、前端订阅/缓冲/序号合并。
- 版本化 JSON 持久化、备份恢复、未完成任务启动恢复。
- 暂停/恢复/取消/重试/删除、暂停全部、清除已完成、打开文件/目录。
- 网络错误最多自动重试 3 次，退避 1s/2s/4s；取消确认后清理登记临时文件。
- 中英文文案、响应式和可访问性。

## 范围外

- 真实 HTTP Range 下载、分片连接、B 站流请求、音频转换、DASH 合并和 FFmpeg 子进程。
- 真实系统完成通知、托盘、通知点击唤起和退出确认，归 `system-release`。
- 并发数/连接数设置 UI 与持久设置来源，归 `settings`；本模块使用默认 3/8 并暴露受限接入点。
- 登录能力、受限内容、任务优先级、拖拽排序、搜索、分页、云同步、输出文件删除。

## 触发与开始条件

- 用户从下载中心成功生成一个或多个草稿并进入 `/tasks`。
- 用户通过侧栏直接打开 `/tasks`，页面加载已有持久任务。
- 应用启动时 Rust manager 加载任务文件并恢复未完成任务。
- 后续 executor 通过 `TaskExecutorPort` 回报进度、暂停确认、完成或失败。

前置条件：`foundation-shell-contracts` 与 `parse-download-center` review 已通过；当前 page-design 与 architecture-design 已存在。

## 需求拆分摘要与来源

- 模块边界来自 `requirements/modules/task-management.md`，顺序位于解析之后、认证/设置/执行器之前。
- 原始来源为 `biliCatch_PRD.md` 模块四约 340-422 行、3.2、5.1；本规格保留四筛选、全部行字段、状态操作矩阵、`download://progress`、队列/并发/恢复/重试/清理规则。
- 页面布局以当前模块 `design/page-design.md` 为上游；代码边界、类型与端口以 `design/architecture-design.md` 为上游，不在本规格改写。

## 用户流程

### 页面初始化与草稿交接

1. TasksPage 先订阅 `download://progress` 和 `download://removed`，订阅完成前显示恢复 loading。
2. 订阅完成后请求 `list_download_tasks`；在快照返回前缓冲两个事件源的事件。
3. store 应用快照，再按 sequence 升序重放 `sequence > snapshot.sequence` 的缓冲事件；旧序号不得覆盖新数据。
4. 若 session handoff 有草稿，使用稳定 `requestId` 调用 `create_download_tasks`。成功后才清空对应 handoff；失败保留并显示可重试错误。
5. 创建成功的任务进入 queued。当前生产执行器尚未接入时保持 queued；fake executor 用于证明 scheduler 行为，不在 UI 伪造下载。

### 任务执行与事件

1. 有 executor 且并发槽位可用时，scheduler 以 FIFO 选择 queued/resumeRequested，创建新 attempt 并进入 downloading。
2. executor 回报必须包含 taskId 与当前 attemptId；不匹配、非法 transition、终止态之后的迟到事件全部忽略。
3. 合法进度必须在同 attempt 内单调不减；Rust 形成完整 task 快照，持久化成功后 emit。
4. UI 至多每 250ms 合并显示一次同任务高频进度；状态变化不等待节流。普通进度最多每秒持久化一次，状态变化立即持久化。
5. 下载结束可进入 processing，再 completed；仅视频等无需处理的执行器允许 downloading 直接 completed。

### 自动与手动重试

- executor 将失败分类为 retryable network 或 terminal。retryable network 第 1/2/3 次自动重试分别等待 1/2/4 秒，保留 checkpoint/进度并创建新 attempt；等待期间任务回 queued 且有 `nextRetryAt`。
- 第 3 次重试仍失败后进入 failed，暴露最终 `AppError`；terminal 失败直接进入 failed。
- failed 的“重试”清空展示错误、瞬时速度/ETA、nextRetryAt 与自动重试计数，保留可用 checkpoint，重新 queued。

### 控制与清理

- 暂停：downloading -> pauseRequested；executor 保存 checkpoint 并确认后变 paused。重复点击禁用。
- 恢复：paused 设置 resumeRequested；有槽位时创建新 attempt 并直接变 downloading，等待槽位期间保持 paused 并显示“等待恢复”。
- 取消：queued/downloading/paused/processing 可请求；先使旧 attempt 失效，再终止 executor 并删除登记临时文件，全部成功后变 cancelled。清理失败进入 failed，不宣称已取消。
- 删除：仅 failed/completed/cancelled；移除记录并 emit removed。completed 输出文件保持原样。
- 打开：仅 completed 且可信 outputPath 存在；Rust 按 task id 查找路径后调用系统 opener。前端不能传任意路径。

## 页面与模块设计

- 复用 AppShell 和全宽内容容器；页面顶部为 tabs + 批量工具条，下方为无外层卡片的稳定列表。
- 桌面六列：文件/媒体、状态、进度、速度、剩余/大小、操作；中宽合并遥测列；极窄切为两至三行 grid。
- 文件名为主信息，次行展示模式、编码/格式与分 P；不可用数值显示短横线。
- 状态 badge 必须有文字与图标/形状差异；进度条固定 6px，百分比、速度和 ETA 更新不得改变列宽。
- 行操作使用 Lucide 图标按钮、tooltip、aria-label 和 36x36px 点击区；打开文件/目录放同一小菜单。
- 初始 loading 为 5 行 skeleton；全空提供“前往下载中心”；筛选空态使用对应短文案；已有任务时的读取/事件错误显示 banner 而不隐藏列表。

## Function-complete 行为拆解

### 筛选与排序

- tabs：全部、下载中、已完成、失败，各显示派生数量。
- “下载中”包含 queued/downloading/paused/processing；“已完成”仅 completed；“失败”仅 failed；cancelled 只在全部出现。
- 默认排序：活动任务在前，终止任务在后；组内按 createdAt 倒序。进度、速度和状态细节更新不改变组内顺序。
- 切换筛选只更新派生视图，不重复请求、复制任务集合或改变选择焦点。

### 列与格式

| 字段 | 展示规则 |
| --- | --- |
| fileName | 单行省略，tooltip/focus 展示完整值；次行显示 page、mode 与可用媒体参数 |
| status | 本地化 badge；queued/downloading/paused/processing/completed/failed/cancelled 全覆盖 |
| progress | 0-100 整数与 progressbar；processing 无确定值时使用 aria-valuetext |
| speed | downloading 且大于 0 时格式化 B/s 至 GiB/s；其他状态显示短横线 |
| ETA | downloading 且已知时格式化为 `mm:ss` 或 `h:mm:ss`；未知显示短横线 |
| size | 显示 downloaded/total；total 未知只显示 downloaded；completed 显示最终总大小 |
| error | failed 时在文件次行显示一行本地化摘要，完整 details 用 tooltip |

### 行操作矩阵

| 状态 | 可见操作 | 前置与结果 |
| --- | --- | --- |
| queued | 取消 | 确认后清理临时文件并 cancelled |
| downloading | 暂停、取消 | 行级 pending；等待 executor 确认 |
| paused | 恢复、取消 | 恢复可等待槽位；取消需确认 |
| processing | 取消 | 只有 executor 声明可中断时可用；确认后先终止进程再清理 |
| failed | 重试、删除 | 重试回 queued；删除只删记录 |
| completed | 打开文件、打开目录、删除 | 路径存在才允许打开；删除不删输出文件 |
| cancelled | 删除 | 只删记录 |

- 同一任务一次只允许一个 control command；按钮 pending 时保持稳定尺寸并禁用该行其他命令。
- 命令失败解除 pending，保留/恢复权威状态，显示行内错误标识和全局错误通知。
- 删除 failed/cancelled 可直接执行；取消、删除 completed、批量清除均需确认，初始焦点放取消按钮并在关闭后归还。

### 批量操作

- 暂停全部：只在存在 downloading 时启用，作用于全部 downloading 而非当前筛选；逐任务遵守相同 pause confirmation protocol，返回 affected ids 与逐项失败。
- 清除已完成：只在 completed 数量大于 0 时启用；确认文案包含数量；一次持久事务移除所有 completed 并逐条/批量 emit remove，输出文件不删除。
- 批量命令 pending 时禁止重复提交；部分暂停失败保留成功项并明确失败数量，清除已完成必须全成或全败。

### 容量与文件名

- 容量按全部未删除记录计算，最大 100；UI 在 90-99 显示接近上限提示，100 显示已满。
- `create_download_tasks` 对整个 drafts 批次先验证；`current + drafts > 100` 时零写入，返回 `E_INTERNAL` 且 `details=TASK_QUEUE_FULL`。
- requestId 必填，trim 后 1-80 个 ASCII 字母/数字/`-`/`_`；空白、换行、其他字符或超长在提交时拒绝。相同 requestId 且对应任务仍存在时返回既有任务，不重复计数。
- outputDir 去除首尾空白后必须非空；内部空白保留。媒体模式字段必须保持 parse 模块约定的互斥组合。
- 文件名从 partTitle 生成，移除控制字符和 `\\ / : * ? \" < > |`，trim 首尾空白/点；空结果 fallback 为 `<bvid>-P<page>`。保留内部连续空白；加扩展名后的完整名称按 Unicode 字符截断到 200，Windows 保留名追加 `_`。

### 持久化与恢复

- 文件位于 Tauri app data 的 `tasks.json`，schemaVersion=1；写入 `.next`，保留最近有效 `.bak` 后替换主文件。
- 主文件损坏时读取 `.bak` 并报告恢复 warning；两者均损坏或版本高于当前时返回错误，不能静默创建空队列覆盖原数据。
- 应用启动时 queued 保持 queued、paused 保持 paused；downloading/processing 恢复为 queued 并保留 checkpoint；completed/failed/cancelled 不变。瞬时 speed/ETA/controlRequest 清空。
- 每个状态变化、创建、删除和批量变更立即落盘；普通进度至多每秒一次，应用正常退出前执行最终 flush。

## 设计约束

### 责任与副作用

- Rust manager 是状态、transition、容量、幂等、retry 和持久化的唯一权威；前端 store 不推测成功状态。
- 网络/媒体执行只在 `TaskExecutorPort`；JSON I/O 只在 `JsonTaskStore`；emit 只在 `TaskEventSink`；opener 只在 validated task command。
- page 只编排生命周期/通知/路由；组件无 Pinia、invoke/listen 或集合规则。

### 规则与命名

- stable domain names：DownloadTask、TaskStatus、TaskAction、TaskProgressEvent、TaskRemovedEvent、TaskExecutionSpec、ExecutionUpdate、attemptId、sequence。
- Rust state machine 是 transition 权威；TS `action-policy` 是即时 UI 镜像，必须由同一矩阵用例证明对齐，不得在 TaskRow/templates 复制条件。
- bytes、速度、ETA 格式化和排序分别为纯函数；组件不做单位/时间运算。

### 复杂度护栏

- manager command 保持 validate -> derive -> persist -> emit；不持锁等待真实下载/FFmpeg。
- 文件超过约 250 行需审查职责，超过 350 行必须拆分；TasksPage 约 180 行内，TaskRow 不拥有列表/IPC。
- 禁止生产 `any`、字符串拼装事件、任意状态 fallback、组件直接 transport、整表响应式替换和未受控 timer/listener。
- 不引入多窗口同步、优先级、可排序队列、通用 Repository/Event Bus/DI container。

## 项目脚手架与允许偏离

- 复用已交付的官方 create-tauri-app Vue + TypeScript + Tauri 2 工程、Pinia、Router、i18n、Naive UI、Lucide、Vitest 和 Rust 分层，不重新脚手架。
- 允许新增 task-management feature、Rust task modules、Tauri event adapter、`tokio` 的 sync/time runtime 能力及测试用临时目录依赖。
- 不允许替换路由/store/UI 框架、引入另一套状态库或把持久任务迁到浏览器 localStorage。

## 变化轴与 Pattern 决策

- State：七状态和操作矩阵需要单点 transition；使用函数式表/guard，不用状态类层次。
- Adapter/Port：文件系统、Tauri event、Tauri transport、后续 audio/video executor 都有真实替换/测试需求，使用小接口；单实现且无替换压力的 formatter 直接函数实现。
- Observer：采用两个 Tauri event，必须支持 unsubscribe、sequence、缓冲重放和节流；不扩展为应用内通用 Event Bus。
- Command：控制请求使用 `TaskAction` 枚举 + 单一 command endpoint；不创建 command object/class。
- TaskStorePort 只暴露 load/save snapshot，不提供通用 CRUD Repository；拒绝 Manager 前端类、Redux reducer、每状态 Strategy 和 DI container。

## 代码上下文与影响假设

- 治理 TypeScript 的是根 `tsconfig.json`：strict、ES2020、DOM、ESNext、bundler、isolatedModules、noEmit、no alias；Node config 只覆盖 Vite 配置。
- 可见声明闭包：本模块 contracts、现有 `AppError/IpcTransport/DownloadTaskDraft`、`vite/client`、Vue/Pinia/Router、`@tauri-apps/api/event` 与现有 opener 包声明。禁止在 execute 再猜 alias/globals 或扫描无关 `.d.ts`。
- 直接影响：TasksPage、task-drafts store、main composition、IPC adapter、Rust lib/commands/models/services/infrastructure、Cargo 与 locale/styles/tests。
- 回归邻居：DownloadPage append+route、AppShell 任务导航/计数预留、AppError、existing ParserService state 注册。不得改变 parse_video 契约或下载中心 UI。
- code graph 不可用与 bootstrap 回退已记录在 `artifacts/code-context.md`；人工依赖追踪在本模块范围足够。

## API 与数据合同

### 权威来源与策略

- 仓库没有 backend-owned TS、protobuf/OpenAPI/IDL。Rust serde DTO 与本规格字段表是权威合同源；TS 以同名 camelCase 字段翻译并直接消费，不做展示性 rename。
- Tauri protocol 通过 `TaskService` adapter；UI 只消费归一化 `AppError` 与稳定 DTO。
- persistence/internal executor 字段不直接跨 IPC；manager 映射为 public DownloadTask。

### Commands

| Command | JS args | Result | 语义 |
| --- | --- | --- | --- |
| `list_download_tasks` | none | `TaskListSnapshot` | 当前完整快照 |
| `create_download_tasks` | `{ request: { requestId, drafts } }` | `CreateTasksResult` | 原子、幂等创建 |
| `control_download_task` | `{ request: { taskId, action } }` | `DownloadTask` | pause/resume/cancel/retry/delete；delete 返回前 emit remove，结果可用删除前快照 |
| `pause_all_download_tasks` | none | `BatchTaskResult` | 所有 downloading |
| `clear_completed_tasks` | none | `ClearTasksResult` | 原子删除 completed 记录 |
| `open_download_task` | `{ request: { taskId, target } }` | unit | target=`file|directory`，仅 completed |

```ts
type TaskStatus = "queued" | "downloading" | "paused" | "processing" | "completed" | "failed" | "cancelled";
type TaskControlRequest = "none" | "pauseRequested" | "resumeRequested" | "cancelRequested";
type TaskAction = "pause" | "resume" | "cancel" | "retry" | "delete";

interface DownloadTask {
  id: string;
  revision: number;
  createdAt: string;
  updatedAt: string;
  fileName: string;
  outputDir: string;
  outputPath: string | null;
  bvid: string;
  cid: number;
  page: number;
  partTitle: string;
  mode: "video-audio" | "video-only" | "audio-only";
  qualityId: string | null;
  codec: "avc" | "hevc" | "av1" | null;
  audioFormat: "mp3" | "m4a" | "flac" | null;
  audioBitrateId: string | null;
  status: TaskStatus;
  controlRequest: TaskControlRequest;
  progressPercent: number;
  bytesDownloaded: string;
  totalBytes: string | null;
  speedBytesPerSecond: string;
  etaSeconds: number | null;
  automaticRetryCount: number;
  nextRetryAt: string | null;
  error: AppError | null;
}

interface TaskListSnapshot { sequence: number; capacity: 100; tasks: DownloadTask[] }
interface CreateTasksResult { sequence: number; capacity: 100; tasks: DownloadTask[]; reused: boolean }
interface BatchTaskResult { sequence: number; affectedTaskIds: string[]; failures: Array<{ taskId: string; error: AppError }> }
interface ClearTasksResult { sequence: number; removedTaskIds: string[] }
interface TaskProgressEvent { sequence: number; task: DownloadTask }
interface TaskRemovedEvent { sequence: number; taskId: string }
```

- `revision` 与 `sequence` 为本次应用会话内单调非负安全整数；时间必须是 RFC3339；progress 0-100；eta 非负或 null。
- Rust `u64` bytes/速度以十进制字符串跨端，TS formatter 使用 BigInt/字符串计算，避免 JS 精度损失。
- 列表空是成功快照；loading 从请求开始到 hydration 完成；AppError 是失败。未知 reject 归 E_INTERNAL。

## 边界与异常

- 空草稿批次、非法 requestId、非法模式字段组合、空 outputDir、超容量均零写入。
- 快照返回晚于 event 时，buffer/replay 不丢更新；重复/倒序 sequence 忽略；未知 task 的 upsert 可加入，未知 task 的 remove 安全忽略。
- 重复 control 在 pending 中拒绝；不存在 task、非法状态 action、completed 缺路径或路径不存在返回错误，不改变记录。
- 进度小于当前、超过 100、bytes 回退、attempt 不匹配、completed 后进度都忽略并可记录诊断，不发送回退事件。
- 取消清理部分失败时任务进入 failed 并保留 cleanup details；用户可重试取消/清理或删除记录。
- event listen 失败时保留快照查询能力并显示 banner；卸载必须执行全部 unsubscribe、清除 timers。
- 页面卸载期间的 command response 不写入已销毁组件，但 Rust 事务继续保持一致。

## 验收标准

- AC-TASK-01：从一个 handoff 批次原子创建每个 draft 一个 queued 任务；相同 requestId 重试不重复；成功后才清空草稿。
- AC-TASK-02：总记录上限 100；99+2 整批拒绝且原集合不变；90/100 容量提示和按钮状态可观察。
- AC-TASK-03：文件名移除全部非法/控制字符、处理空名/保留名、含扩展名最多 200 Unicode 字符；outputDir 与模式互斥校验有表驱动测试。
- AC-TASK-04：Rust 状态机覆盖 queued/downloading/paused/processing/completed/failed/cancelled 的全部合法操作，并拒绝非法 transition。
- AC-TASK-05：scheduler 默认并发 3、FIFO；第 4 项等待；释放槽位后启动；resumeRequested 不越过更早 eligible 任务；production 无 executor 时保持 queued。
- AC-TASK-06：网络失败最多重试 3 次且退避 1/2/4 秒；terminal 直接失败；手动重试清空错误/计数并保留 checkpoint。
- AC-TASK-07：attempt 不匹配、倒序进度、终止态迟到事件不能回退任务；进度状态变化遵守持久化/emit 顺序。
- AC-TASK-08：`download://progress` payload 包含 task id、百分比、速度、ETA、已下载字节及 sequence；前端先订阅/缓冲/快照/重放，在乱序与重复事件下得到正确最终集合。
- AC-TASK-09：重启将 downloading/processing 恢复 queued、paused 保持 paused、终止态不变；主 JSON 损坏可从 backup 恢复，两者损坏不静默覆盖。
- AC-TASK-10：取消 queued/downloading/paused/processing 使旧 attempt 失效并删除登记临时文件；清理失败不得标 cancelled；completed 删除不删除输出文件。
- AC-TASK-11：全部/下载中/已完成/失败筛选、计数和稳定排序正确；全部七状态的列内容、空值格式与错误摘要可观察。
- AC-TASK-12：每种状态只显示 action matrix 允许的操作；行级 pending 防重复；取消/删除完成/清除完成确认与焦点归还正确。
- AC-TASK-13：暂停全部只作用所有 downloading 并报告部分失败；清除已完成为全成/全败且不删除输出文件。
- AC-TASK-14：打开命令只接受 task id + target，由 Rust 解析 completed 的可信存在路径；任意路径不能由前端注入。
- AC-TASK-15：初始 skeleton、全局/筛选空态、已有数据读取错误 banner、单项命令失败、事件监听失败及中英文文案均有组件测试。
- AC-TASK-16：1280、900、800px 视口无重叠/横向页面溢出；tabs、progress、图标按钮、dialog 符合键盘与 ARIA 约束，长中英文文件名不遮挡操作。
- AC-TASK-17：TS exact args/DTO、Rust serde、状态机、scheduler、persistence 与跨端事件 tests 通过；`npm test`/typecheck/build、cargo fmt/check/test 全部通过。
- AC-TASK-18：静态评审证明组件不导入 Tauri/Pinia，页面无状态矩阵，invoke/listen 只在 adapter，parse_video/DownloadPage 行为无回归，未实现范围外下载算法或系统通知。

## 人工评审与交接

- 规格获用户批准后才进入 plan；计划获批后才实现。
- verify 必须提供 AC-TASK-01..18 矩阵、自动化命令、持久化故障注入、事件乱序与多宽度截图证据。
- review 必须明确 clean-code、pattern、跨模块 handoff 和 API contract 结论；有 blocker 回流 execute。
- 完成本模块后，`audio-download`/`video-download` 消费 TaskExecutorPort，`settings` 消费并发/连接数 setter，`system-release` 消费完成/失败事件和窗口通知。

## 风险

- JSON 在 Windows 的替换语义需要 `.next/.bak` 故障路径测试，否则崩溃可造成数据丢失。
- 高频进度可能触发过度渲染/磁盘写入；250ms UI coalescing 与 1s progress persistence 是批准权衡，状态变更仍即时。
- 本模块独立时 production 任务保持 queued，真实执行取决于后续模块；验证不得把 demo/fake 进度描述为真实下载。
- progress/bytes 使用不同数值类型，契约测试必须防止 JS 精度丢失和格式化异常。
- opener、临时文件权限和路径存在性受平台影响；失败必须是可观察 AppError。

