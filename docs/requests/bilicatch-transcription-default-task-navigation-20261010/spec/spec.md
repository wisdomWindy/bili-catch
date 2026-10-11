# 工程规范：转写默认语言与任务页切换时序

## 1. 范围与来源

### 来源标签

- `[SRC-U1]` 用户报告：转写语言默认应为简体中文。
- `[SRC-U2]` 用户报告：点击任务列表应立即切页，任务数据可在切页后加载。
- `[SRC-I1]` 用户截图：转写页语言控件当前显示“自动检测”。
- `[DATA-D1]` 本机 `settings.json`：schema v1 且不存在 `defaultTranscriptionLanguage`。
- `[CODE-C1]` `SettingsDefaults::values` 与 schema v1 迁移缺省字段回退链。
- `[CODE-C2]` `TranscribePage` 的本地表单初始值与设置 watcher。
- `[CODE-C3]` `TasksPage` 的 `onMounted` 同步启动两类 store 初始化；事件源共注册 5 个 Tauri 监听后读取快照。
- `[CODE-C4]` 侧边栏为普通 `RouterLink`，路由无守卫、异步组件或过渡阻塞。

### 范围内

- 将权威设置默认转写语言改为 `zh`，并让 schema v1 缺字段迁移沿用该默认值。`[SRC-U1][DATA-D1][CODE-C1]`
- 将转写页及前端运行时缺省值对齐为 `zh`。`[SRC-U1][CODE-C2]`
- 让任务页先呈现页面结构和 loading 状态，再开始任务订阅与快照读取。`[SRC-U2][CODE-C3]`
- 增加覆盖默认值、迁移、显式设置保留和初始化调度的回归测试。

### 范围外

- 不移除“自动检测”选项，不改语言选项文案。`[SRC-U1]`
- 不覆盖 schema v2 中已显式保存的合法语言值，包括 `auto`。`[SRC-U1][CODE-C1]`
- 不改变任务 IPC、排序、筛选、批量操作、错误态或列表视觉布局。`[SRC-U2]`
- 不修改路由结构，也不预加载任务数据。`[SRC-U2][CODE-C4]`

## 2. 用户意图合同

- 字面请求：默认简体中文；点击任务列表即切页，数据随后加载。`[SRC-U1][SRC-U2]`
- 实际目标：避免用户每次手动选择中文，同时让慢 IPC 不再表现为导航失灵。`[SRC-U1][SRC-U2]`
- 成功标准：语言控件首次稳定值为简体中文；任务 IPC 未完成时，任务页标题与 loading 状态已经出现。`[SRC-U1][SRC-U2]`
- 禁止解释：不能通过删除自动检测、隐藏 loading、提前全局加载任务或忽略用户显式设置来达成表象结果。`[SRC-U1][SRC-U2]`
- 可接受方案：修改现有默认值和兼容迁移回退；在任务页本地生命周期中延迟初始化到首次渲染后的浏览器任务。`[CODE-C1][CODE-C3]`

## 3. 功能完整行为合同

### 3.1 转写语言字段

- 字段：`defaultTranscriptionLanguage` / `form.language`。
- 业务含义：新转写任务使用的首选识别语言；默认值为简体中文。`[SRC-U1]`
- 控件与展示：保留现有 select 及所有选项；默认选中值 `zh`，显示既有“简体中文”文案。`[SRC-U1][CODE-C2]`
- 状态所有权：持久化默认值由后端 `SettingsManager` 拥有，转写页 `form.language` 是页面表单状态，设置加载后由现有 watcher 同步。`[CODE-C1][CODE-C2]`
- 全新设置：生成 schema v2 文档时值为 `zh`。`[SRC-U1]`
- 旧版缺省：schema v1 没有该字段时迁移为 `zh` 并按现有机制持久化。`[SRC-U1][DATA-D1][CODE-C1]`
- 显式设置：已保存的合法 `auto`、`en` 或其他现有枚举值原样保留；本次不做强制迁移。`[SRC-U1][CODE-C1]`
- 前端尚未收到设置时：表单本地初值为 `zh`，避免先显示自动检测再跳变。`[SRC-U1][CODE-C2]`
- 空值与非法值：继续由现有后端反序列化/迁移回退和 TypeScript 联合类型约束处理，不增加模板兜底。

### 3.2 任务列表导航与加载

- 触发：用户点击现有侧边栏“任务列表”链接。`[SRC-U2][CODE-C4]`
- 首次可见状态：路由立即创建任务页，页面标题、工具栏和现有 skeleton 可渲染；此时不得已经启动任务事件订阅。`[SRC-U2]`
- 后续副作用：在首次渲染让出当前任务后，由 `TasksPage` 启动现有下载任务与转写任务 store 初始化。`[SRC-U2][CODE-C3]`
- 初始化内部顺序：各 store 继续先订阅事件再读取快照，不改变事件缓冲与一致性规则。`[CODE-C3]`
- 快速离页：如果延迟初始化尚未触发，卸载时取消待执行调度；如果已经触发，继续使用现有 `dispose` 终止 store 消费。`[SRC-U2][CODE-C3]`
- 成功：快照加载完成后显示现有任务列表/空状态。
- 失败：继续显示现有错误状态；错误文本和重试范围不变。

## 4. 模块与责任边界

- `src-tauri/src/services/settings/mod.rs`：权威默认值和 schema v1 缺省字段迁移结果。
- `src/pages/TranscribePage.vue`：表单加载前的本地缺省值。
- `src/features/settings/demo.ts`：demo 运行时默认设置，与生产默认一致。
- `src/features/download-center/store.ts`：设置效果尚未注入前的自动转写预设缺省值，与生产默认一致。
- `src/pages/TasksPage.vue`：首次渲染后的初始化调度及卸载清理。
- 测试文件只验证上述所有权，不创建第二套业务逻辑。

前端/后端责任保持不变：后端负责设置默认值、迁移和持久化；前端负责页面初始展示、设置消费和生命周期副作用。IPC 字段名、联合类型和 payload 结构不变。

## 5. 异步、失败与性能约束

- loading owner 仍是两个 Pinia store 的 `status`，不新增重复 loading 状态。
- 延迟调度只改变初始化开始时机，不改变请求去重、事件缓冲、错误归一化或结果合并。
- 页面卸载必须清理定时调度，避免离页后注册监听。
- 不引入缓存、debounce、throttle、虚拟列表或预取；当前问题是首次绘制前副作用时序，不是数据计算吞吐量。
- 定时器句柄允许为 `null`，`null` 表示没有待执行初始化；清理后必须恢复为 `null`。

## 6. 类型与代码质量约束

- 前端由根 `tsconfig.json` 管辖：`strict`、`noUnusedLocals`、`noUnusedParameters`、`moduleResolution: bundler`、DOM 类型可用。
- 复用现有 `TranscriptionLanguage` 联合类型和 service/store 类型，不新增宽泛类型或断言绕过。
- 任务页初始化调度作为单页面单调用方生命周期逻辑，保留在 `TasksPage.vue`，不抽取新 composable/helper 文件。
- 副作用只存在于组件生命周期和既有 store action；不把订阅或定时器放入 computed/template。
- 命名使用明确的 `initializeTimer`/`startTaskInitialization` 一类所有权表达；不添加 class、manager 或 factory。
- 任何异常继续由既有 store 捕获与归一化，不吞掉新的 Promise 拒绝。

## 7. 模式与架构决策

- pattern depth：Level 0 direct code。
- selected decision：`reuse existing pattern` + 局部直接调度。
- 复用内容：既有设置默认/迁移模式、Vue 生命周期清理模式、Pinia store 初始化模式。
- 候选信号：存在副作用时序协调，但只有一个页面和一个当前变化轴；新建 composable、调度器或 strategy 会增加所有权跳转且没有第二个生产调用方。
- locality：延迟加载函数和计时器与 `TasksPage` 生命周期同文件；默认值留在各自现有 owner，不新建共享常量，因为 Rust 与 TypeScript 跨语言无法共享运行时常量且各位置承担不同层级的安全缺省。
- Anti-DRY：`zh` 在后端、demo、表单和下载 store 中语法重复但分属持久化权威值、demo fixture、页面启动安全值和 store 安全值；生产使用点语义/层次不同，保持分离并用测试校准。
- `architecture-design_required=false`：不改变模块边界、公共 API、状态拓扑、依赖方向或共享抽象。

## 8. UI、字段与可访问性合同

- 仅语言 select 的默认选择发生变化；标签、选项顺序、键盘操作、focus/hover/disabled 状态均沿用现有 Naive UI 控件。
- “简体中文”是字段当前值的普通默认展示，不新增强调色、徽标或提示。
- 任务页继续使用现有标题、工具栏、skeleton、错误态、空态和任务行；不修改样式。
- 首次绘制后 loading skeleton 必须可被辅助技术通过现有 `aria-label="Loading tasks"` 识别。

## 9. 测试与验收标准

- AC-01：默认设置快照的转写语言为 `Zh`。`[SRC-U1]`
- AC-02：schema v1 缺字段迁移为 `Zh`，且迁移结果被保存。`[SRC-U1][DATA-D1]`
- AC-03：schema v2 显式 `Auto` 加载后仍为 `Auto`。`[SRC-U1]`
- AC-04：转写页未加载设置时语言 select 的值为 `zh`。`[SRC-U1][CODE-C2]`
- AC-05：任务页 mount 完成后标题存在且订阅尚未调用；推进延迟调度后才启动下载和转写初始化。`[SRC-U2][CODE-C3]`
- AC-06：延迟调度触发前卸载页面不会启动订阅。`[SRC-U2][CODE-C3]`
- AC-07：定向 Vitest、Rust settings tests、TypeScript typecheck 和前端 build 通过。
- AC-08：浏览器手工检查任务菜单点击后先出现任务页及 loading，再呈现数据。

## 10. 风险与回滚

- 风险：延迟一个事件循环周期可能短暂显示 store 中已有数据；现有 `initialize` 随后进入 loading，行为仍由现有状态机收敛。验证需检查无错误闪烁。
- 风险：旧 schema v1 缺字段将采用新默认；这是用户明确要求，且显式字段仍保留。
- 回滚面：默认值行、前端安全缺省和任务页局部调度可分别回滚，不涉及数据结构降级。

## 11. 人工评审与完成约束

- 每个 diff hunk 必须映射到默认语言、延迟初始化、清理或直接测试适配。
- 不格式化或重写当前工作区中与本请求无关的既有改动。
- 执行结束前清理未使用 import、调试输出、过期注释和临时测试状态。
- 验证材料需记录定向测试、全量前端门禁、Rust 门禁、手工页面证据及未执行项原因。
- 完成条件：需求一致性、业务逻辑、类型/语法、运行时清理、代码组织和可维护性检查均无可修复 blocker。

## 12. 不适用项

- 页面设计阶段：不适用，仅改变默认选择和加载时序，不改变布局或视觉层级。
- 样式约束：不适用，本次不新增或修改 CSS。
- 新 API / protobuf / DTO adapter：不适用，现有 IPC 合同形状不变。
- Vue 组件抽取：不适用，不新增或拆分 SFC。

## 13. 复测增量约束（2026-10-11）

- 首次渲染期间不得读取、投影或渲染两个任务 store 中的缓存任务；页面标题、工具栏和 loading skeleton 必须可先呈现。
- 初始化调度必须跨过一次浏览器绘制机会，而不只是进入下一个零延迟定时器。
- 初始化开放后继续复用既有缓存、订阅和快照合并语义，不清空任务状态。
- 快速离页时必须同时取消尚未执行的 frame 与 timer 调度。
- 发布版本升级为 `0.1.19`，以生成可明确替换 `0.1.18` 的安装器。
- AC-09：store 预置缓存任务时，mount 后首帧不出现任务行且显示 skeleton；推进首帧调度后才允许出现缓存任务。

## 14. Iteration 8：共享任务运行时生命周期

- 从 `/transcribe` 离开到 `/tasks` 时，不得销毁下载任务或转写任务订阅。
- 使用相同 service/event source 再次进入任一页面时，应复用进行中或已就绪的初始化会话，不得重复注册监听和加载快照。
- `TasksPage` 首次懒初始化仍保留“先绘制、后加载”的调度门。
- 页面只负责取消自身未执行的 animation frame 和 timer；共享订阅仅在应用根节点卸载时统一清理。
- AC-10：转写页任务 store 就绪后切换到任务列表，两类 unsubscribe 均未调用，两个 event source 也未发生第二次订阅。
- AC-11：相同运行时的并发 `initialize` 只订阅一次、加载一次快照，且所有调用方在同一次加载完成后结束等待。
