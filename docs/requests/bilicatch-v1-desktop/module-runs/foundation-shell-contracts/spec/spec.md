# 工程规格：应用基础、壳层与共享契约

## 交付单元标识

`foundation-shell-contracts`

## 背景与目标

仓库当前只有 PRD，没有工程。首模块交付一个可启动、可测试、可扩展的 Tauri Vue TypeScript 应用骨架，并实现 PRD 规定的全局导航、主题与语言基础、稳定 IPC/错误契约。完成后，后续模块无需重写 bootstrap 或绕开共享边界即可逐步接入。

## 范围内

- 使用官方 Tauri Vue TypeScript scaffold 初始化工程。
- 配置 Vue 3、TypeScript、Vite、Vue Router Hash history、Pinia、vue-i18n、Naive UI、Lucide Vue。
- 实现 AppShell、Sidebar、TopBar、路由内容区和全局反馈 provider。
- 注册 `/download`、`/tasks`、`/login`、`/settings` 与兜底路由；业务页在本模块为可辨识占位状态。
- 实现亮色、暗色、跟随系统的运行时主题基础和中英文壳层文案。
- 建立类型化 IPC transport、AppInfo/HealthStatus/AppError 契约与 Rust 健康命令。
- 建立前端单元/组件测试、Rust 单元测试、类型检查、构建命令和基础格式/静态检查。

## 范围外

- 不实现真实 B 站解析、任务下载、二维码、设置持久化、FFmpeg、托盘、通知和更新。
- 不生成最终品牌图标、签名材料或安装包。
- 不在占位页伪造可交互业务功能；占位只用于验证路由和布局。
- 不定制无边框原生窗口。

## 触发与开始条件

- 用户批准本规格后才能进入 plan。
- 用户批准计划后，执行阶段从官方 scaffold 开始。
- 写任何 TypeScript 前，必须读取 scaffold 实际生成的 tsconfig 链与相关声明。

## 需求拆分摘要

本模块是八个顺序模块的第一个，仅承接跨模块基础与应用壳。它向解析、任务、认证、设置、音频、视频和系统发布模块提供稳定路由、布局、类型、错误、主题和测试基础，不提前实现这些业务模块。

来源追踪：PRD 2、3、4.0、5.5、6；`requirements/modules/foundation-shell-contracts.md`；当前模块 page-design 与 architecture-design。

## 用户流程

1. 应用启动，壳层立即显示，默认路由进入 `/download`。
2. 用户通过侧栏切换下载中心、任务列表和设置，当前项同步高亮。
3. 用户点击顶栏登录状态进入 `/login`；点击设置图标进入 `/settings`。
4. 用户使用返回按钮返回上一历史位置；没有历史时按钮保持位置但不可用。
5. 系统主题或 app store 主题变化时，界面无整页闪烁地切换语义主题。
6. 语言切换时，壳层导航、标题、状态与占位页核心文案即时切换。
7. 前端通过类型化 IPC 获取应用信息/健康状态；后端不可达时展示非覆盖式横幅，导航保持可用。

## 页面与模块设计

### 稳定布局

- 桌面：208px 侧栏、56px 顶栏、独立滚动内容区。
- 中等窗口：72px 图标侧栏；窄容错：60px 图标轨道。
- 默认建议窗口约 1180x760，最小 860x600；通过 CSS grid/flex 的固定轨道、`min-width: 0` 和最大内容宽控制，无文字重叠。
- 内容默认最大 1180px，任务等页面后续可用全宽变体。

### 视觉系统

- 系统 UI 字体；正文 14px，页面标题 20px，section 标题 16px；字号不随视口缩放。
- 亮/暗语义 token 使用页面设计中已定颜色；品牌粉只用于关键操作与选中态，信息蓝、成功绿、警告琥珀、危险红分工明确。
- 输入/按钮 6px、卡片最大 8px；无渐变、装饰球或营销式 hero。
- 图标使用 Lucide；纯图标按钮必须有 tooltip 和可访问名称。

## 功能完整行为拆分

### 1. 应用启动与 Provider

- `main.ts` 创建 Vue app，并通过单一安装入口注册 Pinia、Hash Router、i18n。
- Naive UI 的 dialog/message/notification provider 在 AppShell 外层只安装一次。
- 初始化期间导航可见，内容区显示稳定高度 loading 占位；成功后渲染当前路由。
- 初始化异常转为全局 AppError，显示恢复/重试入口，不白屏。

### 2. 路由

| path | name | 标题键 | 本模块结果 |
| --- | --- | --- | --- |
| `/download` | `download` | `nav.download` | 下载中心占位页 |
| `/tasks` | `tasks` | `nav.tasks` | 任务列表占位页 |
| `/login` | `login` | `nav.login` | 登录占位页 |
| `/settings` | `settings` | `nav.settings` | 设置占位页 |
| 任意其他 | `not-found` | `errors.notFound` | 404 状态和返回下载中心按钮 |

- 根路径重定向 `/download`。
- 路由元数据是页面标题单一来源；TopBar 不按 path 写重复分支。
- 未知路由不抛白屏，返回入口可操作。

### 3. 侧栏

- 完整态显示品牌、三个导航项和底部版本区；紧凑态显示图标并用 tooltip 表意。
- 导航项顺序固定：下载中心、任务列表、设置。
- 当前路由以背景、左侧强调条和 `aria-current=page` 同时表达。
- 每项稳定高度 44px，预留任务徽标尾部空间；本模块不显示伪任务数量。
- 点击品牌导航 `/download`。
- 键盘 Enter/Space 可触发，焦点环清晰。

### 4. 顶栏

- 返回按钮占固定 36x36 区域；无有效历史时 `disabled`，不从布局移除。
- 页面标题来自 route meta，过长时单行省略并提供完整可访问名称。
- 匿名状态显示用户图标与“未登录”；点击进入 `/login`。
- 设置按钮是齿轮图标，带 tooltip/aria-label，点击进入 `/settings`。
- 窄容错模式隐藏重复设置按钮，侧栏设置仍可用。

### 5. 主题

- `ThemePreference` 只允许 `light | dark | system`。
- 本模块初始值为 `system`；未接设置持久化。
- `system` 订阅 `prefers-color-scheme`；切换为显式主题时停止由系统变化控制视觉结果，但订阅可保留用于再次切回。
- 根元素设置 `data-theme=light|dark`；CSS 只消费语义 token。
- 主题切换不得重建 router 或页面状态。

### 6. 国际化

- `AppLocale` 只允许 `zh-CN | en-US`，默认 `zh-CN`。
- 本模块提供品牌壳、导航、页面标题、通用状态、404、健康状态的中英文文案。
- 切换 locale 即时更新；不重载页面。
- 缺失键在开发测试中视为失败，生产回退 `zh-CN`。

### 7. IPC 与健康状态

- 页面和 store 不直接导入 Tauri `invoke`。
- `IpcTransport.invoke<T>` 是唯一传输端口；生产 transport 调用 Tauri，测试 transport 使用显式 mock。
- `get_app_info` 返回 `{ name, version }`；`health_check` 返回 `{ status: 'ok', timestamp }`。
- 首次壳层加载调用健康检查：成功显示正常内容；失败显示紧凑横幅。
- 健康检查失败不阻止路由切换；后端依赖命令由后续页面自行禁用。

### 8. 错误契约

- `AppError` 字段：`code` 必填、`message` 必填、`details` 可选且不得包含敏感信息。
- `AppErrorCode` 覆盖 E001-E010 与内部兜底 `E_INTERNAL`。
- Rust 错误以可序列化对象返回；前端 `normalizeIpcError` 处理对象、字符串和未知 reject，未知值归为 E_INTERNAL。
- 页面展示本地化用户文案；技术 details 只进入开发日志，不作为主提示。

### 9. 占位页

- 每个占位页只显示页面标题和简洁空状态，不描述功能教程或假装可用。
- 页面容器遵循已批准内边距、最大宽与响应式规则。
- 后续模块可直接替换页面内部，而无需改变 AppShell。

## 设计约束

### 职责与边界

- App.vue 只组合 providers 和 AppShell；pages 负责编排；layout/shared components 负责展示；stores 负责状态；IPC service 负责副作用与错误归一。
- Rust command 薄层调用 service；service 不依赖 UI；infrastructure 不进入前端。
- 页面不得直接调用 Tauri、localStorage 或全局 DOM 查询。

### 命名与领域语言

- 固定使用 `download`、`tasks`、`login`、`settings` 作为路由领域名。
- 使用 `AppError`、`AppErrorCode`、`AppInfo`、`HealthStatus`、`ThemePreference`、`AppLocale`；不得另建语义重复名称。
- 外部命令名使用 snake_case，TypeScript 函数使用 camelCase，通过 IPC service 显式映射。

### 规则所有权与去重

- 路由标题只存在于 route meta。
- 导航定义只存在于一个静态配置数组。
- 错误码只存在于 contracts，错误到文案的映射只存在于 i18n/错误展示边界。
- 主题颜色只存在于语义 token 文件。

### 副作用与依赖

- invoke 只在 transport；健康检查编排只在 app store/service；根 DOM 主题属性只由主题同步函数写入。
- `matchMedia` 订阅必须在应用卸载/测试销毁时清理。
- Router、Pinia 和 IPC transport 均由 composition root 安装，不使用 service locator。

### 复杂度与可读性

- 不建立万能 service、manager、base component 或事件总线。
- 显式 guard 与小型纯函数优先；导航/主题的派生值不落重复 state。
- 文件职责与架构设计保持一致；偏离需先更新规格/计划。

## 项目 Bootstrap 与 Scaffold 决策

- 存在合适 scaffold：官方 `create-tauri-app` 的 Vue + TypeScript 模板。
- 执行必须使用该模板，不手写 Tauri/Vite 初始化文件。
- 固定选择：Vue、TypeScript、npm；应用名/包名使用 BiliCatch 对应合法标识。
- 允许偏离：保留 scaffold 实际配置拆分；替换示例组件/样式；增加 Router、Pinia、i18n、Naive UI、Lucide 与测试工具；按架构设计拆分源目录。
- 不允许偏离：更换 React/Svelte、切换 Electron、删除 Tauri Rust 端、改成 history 路由。
- 若 scaffold 最新稳定依赖与 PRD 的 Vite 6 精确版本不一致，优先采用 scaffold 相互兼容的稳定组合，并在 changelog 记录，禁止强制降级造成已知兼容问题。

## 变化轴与 Pattern 决策

- 运行环境（Tauri/测试浏览器）确实变化：采用 Adapter 形式的 `IpcTransport`。直接在组件调用 invoke 会阻塞单测与浏览器预览，因此不采用。
- 前后端依赖组装：使用轻量 Composition Root，不使用容器或 service locator。
- 主题三态使用直接枚举与纯函数分支，不为三个值引入 Strategy 类。
- 导航使用数据数组直接渲染，不使用 Factory。
- 不使用 Repository、Event Bus、Command class 或继承层次；当前没有对应复杂度。

## 代码上下文与影响假设

- 当前仓库无工程代码、Git 和 tsconfig，属于纯绿地。
- 不需要代码图；不存在可恢复的调用关系。
- scaffold 后必须读取实际文件再执行后续修改，不能假定 alias 或 ambient globals。

## API 与数据契约

### 权威来源

本模块后端契约由同仓库 Rust DTO 与 TypeScript contracts 共同维护；没有后端生成的 TypeScript、protobuf 或 OpenAPI。Rust 序列化测试是字段形状的权威验证，TypeScript 契约是前端消费入口。

### `get_app_info`

- 协议：Tauri command `get_app_info`，无参数。
- 成功：`{ name: string, version: string }`，两字段非空。
- 失败：`AppError`。
- 消费：直接消费，无 mapper。

### `health_check`

- 协议：Tauri command `health_check`，无参数。
- 成功：`{ status: 'ok', timestamp: string }`，timestamp 为 RFC3339 字符串。
- 失败：`AppError`，前端规范化并进入后端不可达横幅状态。
- 消费：直接消费，无 mapper。

### 错误形状

```ts
interface AppError {
  code: 'E001' | 'E002' | 'E003' | 'E004' | 'E005' |
        'E006' | 'E007' | 'E008' | 'E009' | 'E010' | 'E_INTERNAL'
  message: string
  details?: string
}
```

不重命名字段，不让组件处理 Rust enum 内部形状。

## TypeScript 上下文

规格审批时尚无 tsconfig，因此无法声称某个现存配置 governs 文件。执行门禁为：scaffold 后以根 `tsconfig.json` 为入口，读取它直接引用/extends 的 `tsconfig.app.json`、`tsconfig.node.json` 或实际等价文件；记录 `strict`、`module`、`moduleResolution`、`target`、`lib`、`types`、`paths`。相关声明闭包仅包括 `src/vite-env.d.ts`、实际导入的 Vue/Vite/Tauri 包声明和本模块 contracts；禁止仓库级扫描所有 `.d.ts`。

## 上下文与依赖来源

- PRD 与当前模块拆分文件。
- 当前模块 page-design、architecture-design。
- 官方 scaffold 生成的 package/config 文件（执行后成为事实来源）。
- Rust `Cargo.toml` 与 Tauri capabilities（执行后读取）。

## 边界与异常

- 直接打开未知 hash：展示 404，不崩溃。
- 浏览器单测环境没有 Tauri：通过注入 mock transport，不读取未定义全局。
- 健康检查 reject 为字符串/对象/null：均规范化为 AppError。
- 系统主题在运行中改变：仅 `system` 偏好跟随。
- 英文标题更长：侧栏紧凑态隐藏文字，完整态不得覆盖徽标区。
- route meta 缺少 titleKey：回退通用应用名并在开发环境警告。
- 用户连续快速导航：壳层尺寸保持稳定，无状态丢失。

## 验收标准

- AC-FND-01：官方 Tauri Vue TypeScript scaffold 痕迹和配置存在，`npm` 安装成功，非手写空壳。
- AC-FND-02：开发服务器可启动，默认 hash 路由展示 `/download`，四个规定路由和 404 可访问。
- AC-FND-03：桌面与 900px/800px 宽度下，导航、顶栏、标题和内容无重叠；侧栏按设计切换完整/图标态。
- AC-FND-04：侧栏与顶栏入口均导航到正确路由，当前项具有视觉和 `aria-current` 状态，图标按钮有 tooltip/aria-label。
- AC-FND-05：亮、暗、跟随系统三态可切换；根主题属性和语义颜色同步，无页面重载。
- AC-FND-06：简体中文与 English 壳层文案可即时切换，核心键无缺失。
- AC-FND-07：前端不在 page/component 直接调用 `invoke`；所有 IPC 经 `services/ipc`。
- AC-FND-08：`get_app_info` 与 `health_check` 在 Rust 返回规定 JSON 形状；前端可消费成功与错误。
- AC-FND-09：E001-E010 与 E_INTERNAL 的类型和用户文案映射存在；未知 reject 归一为 E_INTERNAL。
- AC-FND-10：前端单元/组件测试覆盖路由、导航、主题、locale 和错误规范化；Rust 测试覆盖 DTO/错误序列化与健康命令核心逻辑。
- AC-FND-11：TypeScript 检查、前端测试、前端生产构建、Rust `cargo test` 和适用的 `cargo check` 通过。
- AC-FND-12：实现遵守已批准目录/依赖方向，无万能 manager、事件总线、重复路由/错误/主题规则。

## 人工评审与交接

- 规格需用户明确批准后进入 plan。
- 实现完成后提供可运行本地 URL，并用截图/浏览器检查桌面与紧凑宽度。
- 本模块评审通过后，模块状态完成并推进 `parse-download-center` 的 page-design。

## 风险

- scaffold 最新版本与 PRD 版本可能偏差：记录并采用兼容组合。
- Tauri 编译依赖可能未安装：前端仍可测试，但 Rust/Tauri 门禁必须准确报告，不伪造通过。
- 当前没有最终图标：使用可替换占位，不影响布局和契约验收。
- Naive UI 默认 token 与自定义 CSS 可能冲突：由单一主题对象映射语义 token，并以视觉验证收口。

