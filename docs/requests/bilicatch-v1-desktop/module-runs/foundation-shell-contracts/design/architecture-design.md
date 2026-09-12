# 架构设计：应用基础、壳层与共享契约

## 交付单元标识

`foundation-shell-contracts`

## 架构目标

以官方 Tauri Vue TypeScript 脚手架建立可运行的双端工程，提供稳定的应用壳、路由、主题、国际化、共享契约与 IPC 适配边界，使后续业务模块可以纵向添加而不改写基础设施。

## 架构范围与触发因素

- 绿地项目，需要明确 scaffold、包管理器、TypeScript 配置和测试入口。
- 后续至少七个模块会共享路由、错误、IPC、状态和视觉 token。
- B 站与桌面环境不稳定，页面不能直接依赖原始 Tauri 或第三方数据。
- 本模块只建设基础壳和契约，不实现解析、任务、登录、设置或真实系统命令。

## 上游输入与假设

- 以当前模块需求文件和已批准页面设计为边界。
- 使用官方 `create-tauri-app` 的 Vue + TypeScript 模板；不手写 Tauri/Vite bootstrap。
- 包管理器优先 npm，因为当前仓库未提供锁文件或其他偏好。
- 脚手架生成后才存在真实 `tsconfig`；执行阶段必须先读取其 extends 链、`vite-env.d.ts` 和 Tauri 相关类型，再修改 TypeScript。
- 当前是绿地工作，代码图无现有关系可恢复，因此使用目录与依赖检查即可。

## 模块边界设计

### 前端

- `app/`：应用初始化、路由、providers 和壳层组合。
- `pages/`：仅路由容器；本模块提供轻量占位页，业务逻辑由后续模块替换。
- `components/layout/`：AppShell、Sidebar、TopBar 等壳层组件。
- `components/shared/`：真正跨业务的状态反馈与图标按钮，不放页面专用组件。
- `stores/`：仅应用级状态，如主题、语言、认证摘要；业务 store 后续就近增加。
- `services/ipc/`：前端唯一 Tauri invoke 边界，暴露类型化端口。
- `contracts/`：Rust Command 的稳定序列化契约镜像和领域枚举。
- `locales/`、`styles/`：文案与语义 token。

### Rust

- `commands/`：薄 IPC 命令，只做输入校验、调用 service、序列化结果。
- `services/`：业务用例边界；本模块仅提供应用信息/健康检查样例。
- `infrastructure/`：系统/第三方实现；本模块不接 B 站。
- `models/`：共享 DTO 与 AppError。
- `lib.rs`：Tauri builder 与 command 注册；`main.rs` 只启动库入口。

## 文件与目录结构

```text
.
├─ package.json
├─ vite.config.ts
├─ tsconfig.json
├─ tsconfig.app.json
├─ tsconfig.node.json
├─ index.html
├─ src/
│  ├─ app/
│  │  ├─ App.vue
│  │  ├─ router.ts
│  │  └─ providers.ts
│  ├─ components/
│  │  ├─ layout/{AppShell,Sidebar,TopBar}.vue
│  │  └─ shared/{AppIconButton,AppStatus}.vue
│  ├─ contracts/{app-error,ipc}.ts
│  ├─ pages/{Download,Tasks,Login,Settings,NotFound}Page.vue
│  ├─ services/ipc/{client,app}.ts
│  ├─ stores/app.ts
│  ├─ locales/{index,zh-CN,en-US}.ts
│  ├─ styles/{tokens,base,layout}.css
│  ├─ main.ts
│  └─ vite-env.d.ts
├─ src-tauri/
│  ├─ Cargo.toml
│  ├─ tauri.conf.json
│  ├─ capabilities/default.json
│  └─ src/
│     ├─ commands/mod.rs
│     ├─ services/mod.rs
│     ├─ infrastructure/mod.rs
│     ├─ models/{mod.rs,app_error.rs}.rs
│     ├─ lib.rs
│     └─ main.rs
└─ tests/
```

允许偏离：官方脚手架生成的文件名/配置拆分可保留；只有在职责仍满足上述边界时才调整，不为了目录外观做无意义移动。

## 代码关系与依赖方向

```text
pages -> layout/shared components -> stores/contracts
pages/stores -> typed IPC ports -> @tauri-apps/api

Tauri commands -> services -> infrastructure ports/implementations
Tauri commands/services -> models
infrastructure -/-> frontend
```

- 页面不得直接调用 `invoke`；只能调用 `services/ipc` 暴露的函数。
- 共享组件不得导入 pages 或具体业务 store。
- 前端契约字段与 Rust DTO 序列化字段保持一致；只有外部 B 站响应需要后续 adapter。
- Rust `lib.rs` 负责组合依赖，业务 service 不反向依赖 Tauri UI。

## 职责拆分

| 单元 | 职责 | 禁止内容 |
| --- | --- | --- |
| `App.vue` | provider 与 AppShell 组合 | 业务路由分支、IPC 细节 |
| `router.ts` | 路由表和标题元数据 | 页面业务状态 |
| AppShell | 布局、宽窄导航结构 | 登录/任务业务逻辑 |
| Sidebar/TopBar | 导航触发与状态展示 | 直接持久化或 invoke |
| app store | 主题、语言、后端可用性、壳层状态 | 下载任务集合 |
| IPC client | invoke 调用、AppError 规范化、mock/real 选择 | toast、路由跳转 |
| Rust command | 参数边界与 Result | HTTP/文件/FFmpeg 细节 |
| Rust service | 用例与领域规则 | WebView 展示逻辑 |

## 函数设计与公开入口

### 前端入口

- `createAppRouter()`：创建 Hash Router。
- `installAppProviders(app)`：集中安装 Pinia、Router、i18n、Naive UI 需要的 provider。
- `createIpcClient(transport)`：形成可替换 transport 的类型化客户端。
- `getAppInfo()` / `checkBackendHealth()`：本模块仅有的示例/健康命令。
- `useAppStore().setTheme()` / `setLocale()`：纯状态与 DOM 语义属性协调，持久化留给设置模块。

### Rust 入口

- `run()`：组装 Tauri app。
- `get_app_info() -> Result<AppInfo, AppError>`。
- `health_check() -> Result<HealthStatus, AppError>`。

公开面保持小而明确，不建立万能 `ApiService`、`Manager` 或事件总线。

## 状态归属与数据流

1. `main.ts` 创建应用并安装 provider。
2. app store 从系统偏好初始化主题/语言默认值。
3. AppShell 订阅 router 和 app store，仅渲染派生视图。
4. 页面通过 IPC service 查询后端健康；client 把 Tauri reject 规范化为 AppError。
5. UI 层根据 AppError code 选择本地化文案；不解析 Rust 调试字符串。

Router 是位置状态源，Pinia 是应用状态源，Rust 是桌面能力和后续任务状态源。任何一个状态都不得被多个层同时写入。

## 数据结构与类型策略

```ts
type AppRouteName = 'download' | 'tasks' | 'login' | 'settings' | 'not-found'
type ThemePreference = 'light' | 'dark' | 'system'
type AppLocale = 'zh-CN' | 'en-US'

interface AppInfo { name: string; version: string }
interface HealthStatus { status: 'ok'; timestamp: string }
interface AppError { code: AppErrorCode; message: string; details?: string }
type AppErrorCode = 'E001' | ... | 'E010' | 'E_INTERNAL'
```

- TypeScript 开启 scaffold 提供的 strict 配置；不使用未解释的 `any`。
- Rust DTO 使用 `serde` 的 camelCase 序列化策略与前端字段对齐。
- 前端 contracts 是本仓库的 IPC 权威 TypeScript 类型；Rust 测试校验序列化形状。没有 OpenAPI/protobuf 来源。
- `E_INTERNAL` 是系统性兜底，不替代 PRD 的 E001-E010。

## 契约与适配器边界

- `IpcTransport` 仅含类型化 `invoke<T>(command, args)`；真实实现包装 Tauri，测试/浏览器预览使用内存实现。
- `normalizeIpcError(unknown): AppError` 是唯一错误归一入口。
- 页面直接消费稳定 IPC DTO，不重复 mapper；未来 B 站原始响应只在 Rust infrastructure 转换。
- Mock transport 是开发/测试适配器，不是独立业务后端，不得在生产默认启用。

## Pattern 决策与拒绝项

- 采用 Adapter：隔离 Tauri `invoke` 与浏览器测试 transport。实际变化轴是运行环境与错误形状；直接在组件调用会使测试依赖 WebView。
- 采用 Composition Root：`providers.ts` 与 Rust `lib.rs` 集中组装依赖，避免隐藏全局单例。
- 不引入 Repository：本模块没有集合持久化语义。
- 不引入 Service Locator/Event Bus：Pinia、Router 和显式 IPC 端口已经覆盖当前协作，不需要隐式依赖。
- 不建立抽象组件工厂：导航项使用数据数组直接渲染即可。

当 Adapter 只有一个 transport 且不再需要浏览器测试时可重新评估；当前测试与 WebView 双环境使其合理。

## 可读性与维护门禁

- 页面容器只负责编排，布局组件只负责展示/导航，错误映射只存在于 IPC 边界。
- 单文件超过约 250 行或同时承担三类职责时，在当前范围内拆分，而不是继续堆叠。
- 路由名、错误码、主题和 locale 枚举只定义一次。
- 副作用函数使用 `load/check/set/install` 等动词，不把写操作藏在 `get` 中。
- CSS 只通过语义 token表达颜色；禁止页面内散落十六进制颜色和 inline style。
- 不为未来平台解析预建复杂插件注册系统；当前路由数组保留简单扩展即可。

## TypeScript 上下文门禁

当前没有 `tsconfig` 或声明文件，无法提前假定别名、moduleResolution 或 ambient globals。执行阶段顺序必须是：

1. 运行官方 scaffold。
2. 读取生成的 `tsconfig.json` 及直接 extends 链。
3. 读取 `src/vite-env.d.ts` 与实际使用包的相关声明入口。
4. 记录 strict、module、moduleResolution、target、lib、types 和 path alias 结论。
5. 才开始修改 TypeScript。

## 架构风险

- 官方 scaffold 版本可能与 PRD 的 Vite 6 目标有差异；以生成后的兼容依赖为基线，规格中记录版本偏差，不强制降级到不受支持组合。
- 浏览器 mock 与 Tauri real transport 可能行为漂移；契约测试必须覆盖成功与错误序列化。
- 同时使用 Naive UI provider 与自定义 token 容易出现双重主题源；app store 只生成一个主题配置并设置根属性。
- Rust 模块在首模块过度空壳化会增加噪音；只创建近期模块必需的目录和小型接口。

## 开放架构问题

- scaffold 实际生成的 `tsconfig` 形状需执行阶段确认。
- 最终应用图标与签名资源不在本模块。
- Cookie 安全存储、任务持久化和设置 store 的具体实现分别由对应模块架构决定。

