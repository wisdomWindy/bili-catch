# 工程规格：设置管理

## 交付单元标识

`settings`

## 背景与目标

BiliCatch 现有 `/settings` 只是占位页，主题/语言仅有会话内默认值，下载目录和媒体默认值写死在下载中心，任务调度也持有独立的 `(3, 8)` 配置。当前模块交付完整设置页和版本化权威设置源，使设置在修改后立即保存、重启后恢复，并由下载中心、任务调度及后续系统模块通过明确边界消费。

目标：

- 完成下载、外观、系统、关于四个设置分组及已批准页面状态。
- 用 Rust `SettingsManager` 统一默认值、校验、迁移、持久化和 revision。
- 主题与语言即时生效，保存失败时 UI 与持久值同时回滚。
- 同字段快速连续变更按用户最新意图顺序落盘，旧结果不能覆盖新值。
- 下载中心使用已确认的目录/媒体默认值，任务调度动态使用 `1-10` 并发和 `1-32` 连接数。
- 为后续 `system-release` 保留稳定的更新检查与许可动作 port，不虚构发布配置。

## 范围内

- `/settings` 页面、中文/英文文案、响应式样式和无障碍状态。
- 11 个持久字段：下载目录、临时目录、最大并发、单任务连接数、默认视频清晰度、默认音频格式、主题、语言、完成通知、关闭窗口行为、自动检查更新。
- 当前版本、主动检查更新、开源许可三项关于区行为。
- TypeScript settings contract/service/store/effect sink/native ports/demo runtime 和测试。
- Rust settings DTO、manager、validation、migration、plugin-store adapter、commands 和测试。
- `tauri-plugin-store` Rust 依赖、`tauri-plugin-dialog` Rust/JS 依赖及 `dialog:allow-open` 最小 capability。
- 对现有 app/download/task 边界的必要接入，以及 `connectionsPerTask` 上限从错误的 10 修正为 32。

## 范围外

- 真正的下载执行、完成通知发送、窗口关闭拦截、托盘行为。
- updater 插件安装、更新下载/安装、签名、公钥、端点和 GitHub Releases 配置。
- 最终许可 URL 配置。
- Cookie、凭据或其他秘密的存储。
- 设置导入/导出、恢复默认按钮、账号同步、云同步和移动端专用设置体验。
- 重构与当前设置消费无关的任务、认证或解析逻辑。

## 触发与开始条件

- 用户从侧栏或顶栏进入 Hash 路由 `/settings`。
- 应用根组件启动时，无论当前路由为何，都开始一次设置 hydrate。
- 前置条件：基础壳、下载中心、任务管理、认证模块均已 review PASS；设置 page design 和 architecture design 已完成。
- 若 Rust backend 不可用，现有全局 backend banner 继续负责总体状态；设置页同时显示本模块 load error 与重试，不切换到隐式内存持久化。

## 需求拆分摘要

- 来源：`biliCatch_PRD.md` 模块六 479-534 行、6.3、6.4；归一化模块为 `requirements/modules/settings.md`。
- 只交付 `settings`，不把 `audio-download`、`video-download` 或 `system-release` 的执行能力提前混入。
- 上游强制保留：四分组、全部字段/范围/枚举/默认值、即时保存、目录选择、滑块释放保存、主题/语言即时生效。
- 下游交接：音视频执行读取合法下载参数；系统发布读取通知、关闭行为和自动检查更新设置，并替换 release port。

## 用户流程

### 启动恢复

1. 根组件调用 `SettingsStore.initialize`。
2. service 调用 `get_settings_snapshot`。
3. Rust 从 `settings.json` 的 `document` key 读取 raw value，迁移到 V1，用运行时默认路径补齐字段并校验。
4. 首次运行或规范化后需要写回时，显式保存完整 V1 文档；保存失败则初始化失败，不伪报 ready。
5. 前端接受 snapshot 后，将主题/语言应用到 app store，将已确认下载默认值应用到 download-center store。
6. 访问设置页时显示已确认值；访问下载页时也能使用恢复后的默认值。

### 修改与即时保存

1. 用户修改一个字段，组件只 emit `{field, value}` 意图。
2. store 更新 visible desired；主题/语言同时立即应用候选效果。
3. 该字段进入 `saving`，由单字段 drain loop 调用 `update_setting`。
4. Rust clone candidate、应用 typed patch、校验、revision 加一、写入 plugin store；只有保存成功才替换 manager 内存。
5. 前端接收不旧于当前 snapshot 的结果，更新 confirmed 值和跨模块已确认效果，短暂显示 `saved` 后回到 `idle`。
6. 保存失败且没有更新候选时回退该字段；主题/语言同步回退界面。失败状态保持到用户再次修改或重新初始化。

### 同字段连续修改

1. 保存进行中时允许用户选择新值；desired 被更新，但不启动第二个同字段 worker。
2. 当前写入完成后，若 desired 与 confirmed 不同，worker 立即保存最新 desired。
3. 中间值可以合并；最终持久值必须等于最后一次用户意图。
4. 不同字段可以并行请求；Rust manager 串行 candidate transaction，返回完整 snapshot 和单调 revision，防止 lost update。

### 目录、更新与许可

1. 目录按钮调用 `DirectoryPickerPort`，以当前目录作为 `defaultPath`，选项固定为 `directory: true`、`multiple: false`。
2. 返回 `null` 是取消：保持值和状态，不通知错误。返回路径则通过普通 commit 流程校验/保存。
3. “检查更新”调用 `ReleaseActionsPort`，显示 checking；结果为 available/latest/error，当前页不跳转。
4. “开源许可”调用 release port 打开系统浏览器；失败留在当前页并显示本地化错误。
5. 在 `system-release` 接入前，production deferred adapter 必须明确返回 `UPDATE_NOT_CONFIGURED` 或 `LICENSES_NOT_CONFIGURED`，不得伪造成功。

## 页面与模块设计

页面严格承接 `design/page-design.md`：

- 一个连续、左对齐、最大宽度约 880px 的设置表单；四个 section 之间使用留白和分隔线，不用页级卡片、tabs 或 accordion。
- `>=900px` 使用 `label/control/88px status` 三列；`760-899px` 缩窄标签列；`<760px` 每行单列；任何宽度不产生横向滚动。
- 路径行是只读输入 + `FolderOpen` 图标按钮，长路径省略但可获取完整值。
- 滑块后有稳定数值输出；select、switch 与 About 行保持统一控件起始线。
- 首次加载为稳定 skeleton；整体 load error 为 inline alert；逐字段状态为固定尾列。
- 成功状态短暂且局部，错误持久；状态图标与文字共同表达，不只使用颜色。
- 页面组件层次固定为 `SettingsPage -> group components -> SettingRow/field controls`。业务组件只用 props/emits。

## Function-Complete 行为分解

### 页面初始化与全局状态

| 状态 | 可见行为 | 操作 |
| --- | --- | --- |
| `idle/loading` | 四组稳定 skeleton；页面标题存在 | 设置控件禁用 |
| `ready` | 显示权威 snapshot | 全部适用控件可操作 |
| `load_error` | 标题下 inline alert，使用本地化错误文案 | “重试”重新调用 initialize |

- 根 initialize 幂等：已有进行中的同一生命周期请求不并发重复；旧 initialize 结果不得覆盖新生命周期。
- load error 时使用 `system/zh-CN` 作为运行时安全外观默认，但 UI 不显示为已持久值，也不允许修改未知设置。
- `AppInfo.version` 成功时显示原值；不可用时显示本地化“不可用”，不猜版本。

### 字段合同

| 字段 | 必填/控件 | 允许值与默认值 | 提交与失败行为 |
| --- | --- | --- | --- |
| `downloadDirectory` | 必填、只读路径 + 目录按钮 | 绝对、存在的目录；默认安装目录下 `BiliCatch` | picker 选中后保存；失败回退 |
| `temporaryDirectory` | 必填、只读路径 + 目录按钮 | 绝对、存在的目录；默认安装目录下 Temp | picker 选中后保存；失败回退 |
| `maxConcurrentTasks` | 必填、整数滑块 | `1..10`、step 1、默认 3 | input 预览；commit/失焦保存 |
| `connectionsPerTask` | 必填、整数滑块 | `1..32`、step 1、默认 8 | input 预览；commit/失焦保存 |
| `defaultVideoQuality` | 必填、select | `16/32/64/80/112/120/125/127`；默认 `80` (1080P) | change 后保存；失败回退 |
| `defaultAudioFormat` | 必填、select | `mp3/m4a/flac`；默认 `mp3` | change 后保存；失败回退 |
| `theme` | 必填、select | `light/dark/system`；默认 `system` | 立即换肤后保存；失败换回 confirmed |
| `locale` | 必填、select | `zh-CN/en-US`；默认 `zh-CN` | 立即切换全局文案后保存；失败切回 confirmed |
| `notifyOnComplete` | 必填、switch | boolean；默认 true | toggle 后保存；失败回退 |
| `closeBehavior` | 必填、select | `minimizeToTray/exit`；默认 `minimizeToTray` | change 后保存；失败回退 |
| `autoCheckUpdates` | 必填、switch | boolean；默认 true | toggle 后保存；失败回退 |

路径输入不接受键盘输入、粘贴或换行，因此无 trim/空白编辑语义。dialog 返回值按平台原样保留，不在前端 trim 或拼接；Rust 将空字符串、相对路径、不存在路径和非目录判为 `SETTINGS_PATH_INVALID`。默认 `BiliCatch` 目录不存在时 Rust 尝试创建；创建失败为 load error。

select 不接受自由文本。滑块 UI 自身限制范围，但 Rust 对整数与范围再次校验；非法 IPC patch 返回 `SETTINGS_INVALID_VALUE`，不改变 revision、内存或磁盘。switch 只传 boolean。

### 逐字段保存状态

- `idle`：尾列空白但保留尺寸。
- `preview`：滑块数值变化；不调用 IPC、不进入 live region。
- `saving`：spinner + 本地化“保存中”；控件继续接受最新意图。
- `saved`：check + “已保存”，约 1.2-2 秒后回到 idle；定时器不得清掉更新 generation 的状态。
- `error`：alert icon + 短文案，`aria-live=polite`；再次修改后进入 saving。
- 页面卸载不取消根 settings store；未完成保存继续由根 store 拥有。组件卸载后不得触发 DOM 通知，但最终 confirmed 状态仍正确。

### 下载设置消费

- `download-center` store 新增 `configureDefaults`，只接收下载默认子集，不依赖完整 settings DTO。
- `outputDir` 在没有用户当前页面选择时使用 confirmed `downloadDirectory`；已有当前选择不能被后台 hydrate 或后续设置修改突然覆盖。
- 新解析结果默认选择 `defaultVideoQuality` 对应且当前可用的 quality；不存在/需登录时退到首个可用 quality。
- 切换到仅音频时优先选择 `defaultAudioFormat`，仅当解析结果包含该格式；否则选择结果中的首个格式。
- 清空解析结果保留已配置默认目录，清除本次解析选择；下一次解析重新应用最新 confirmed 媒体默认值。

### 任务调度消费

- `TaskManager` 移除独立 `scheduler_config` 与可写 setter，构造时注入 `TaskSettingsPort`。
- 每次 `claim_next` 读取最新 `SchedulerLimits`；最大并发只影响是否认领新任务，不暂停或取消已运行任务。
- `connectionsPerTask` 写入新 `TaskExecutionSpec`；已经启动的 attempt 保持启动时连接数。
- manager 和 settings validation 都必须接受 `connectionsPerTask=32`，拒绝 0 和 33。

### 关于区

- 版本行从既有 `useAppStore().appInfo?.version` 读取，不新增版本 command。
- 检查更新按钮 pending 时禁用并保持尺寸；latest 显示“已是最新版本”，available 显示检测到的版本，error 显示本地化失败。
- current module 的 available 结果不包含安装动作；安装确认归 `system-release`。
- 许可动作使用外链图标和可访问名称。deferred error 不打开空 URL、不使用 `javascript:`/自造本地页面。

### 无障碍与国际化

- `SettingRow` 为每个控件建立 label/id，说明和错误通过 `aria-describedby` 关联。
- switch 可通过 label 点击和键盘操作；select 保持原生/Naive UI 键盘语义。
- slider 提供 min/max/step/value 与本地化 `aria-valuetext`；每刻度变化不进入 live region。
- 目录按钮具备 tooltip 与 `aria-label`，dialog 关闭后焦点回到触发按钮。
- theme/locale 更新不移动焦点、不滚动页面。中英文最长 label/status/button 可换行但不覆盖控件。
- 新增文案同时进入 `zh-CN.ts`、`en-US.ts`；用户界面不展示 raw backend message 或 details 常量。

## 设计约束

### 职责与边界

- `SettingsPage` 只编排；components 只展示/emit；Pinia store 只管理前端设置流程；service 只管理 IPC；manager 管理领域事务；infrastructure 管理 plugin I/O。
- 目录、store、release plugin 只能从各自 adapter 导入。Vue 组件不得 `invoke()` 或直接 import Tauri plugin。
- 根 `SettingsEffectSink` 是唯一跨 store 写入口；不得用 watch 链或事件总线复制同步逻辑。

### 命名与规则所有权

- 字段名必须使用架构工件中的 11 个 camelCase 名称；Rust serde 输出相同字段名。
- Rust `validation.rs` 是运行时值域权威；TS options/type 是编译与控件约束，并由跨端 serde fixture 测试防漂移。
- 媒体枚举提升到 `src/contracts/media.ts`；下载中心保持兼容 re-export，不复制 `AudioFormat`。
- 页面组件不得重复默认值、范围或回滚规则。

### Side Effect

- 持久化只在 Rust store adapter；`disable_auto_save` 后每次 update 显式 save。
- adapter 在 `set(candidate)` 前保存旧 cache value；save 失败恢复旧 cache，manager 不提交 candidate。
- 主题/语言仅由 `SettingsEffectSink.applyAppearance` 乐观应用；下载默认值仅通过 `applyCommitted` 应用。
- 任务 scheduler 只读 `TaskSettingsPort`，不写设置。
- 通知、窗口、更新和许可的真正系统行为保留给后续 owner。

### 复杂度与可读性

- 同字段 drain loop 是唯一保存并发算法；不得在各组件各写一套 token/watch/debounce。
- 不使用锁跨 await；Rust plugin-store `get/set/save` 按当前 2.x API 为同步调用。
- 不引入动态表单 schema、全局 command bus、通用 transaction 框架或多层前端 class。
- 修改已完成模块仅限共享媒体类型、下载默认接入和 task settings port；其他行为必须保持回归测试通过。

## 项目脚手架与 Starter 决策

项目已基于 `create-tauri-app` 4.7.4 的官方 Vue + TypeScript 模板完成脚手架，并已扩展为现有 Pinia/feature/service/Rust command-service-infrastructure 结构。本模块复用现有工程，不重跑生成器、不替换构建工具。

允许偏离仅包括：新增 settings feature、Rust settings 模块、store/dialog 依赖与最小 capability，以及必要的 composition 和消费者接入。不得改变 Hash Router、TypeScript module resolution、Naive UI、测试框架或既有目录总结构。

## 变化轴与 Pattern 决策

- **Adapter/Port**：用于隔离 IPC、plugin-store、dialog、未来 release provider 和 demo/test runtime；真实变化轴是运行环境与原生实现。禁止从组件直接访问这些依赖。
- **Candidate transaction**：复用 task manager 的 save-before-commit 形态，解决磁盘失败时 manager 状态分叉；只作为 manager 内直接控制流，不抽象成框架。
- **Per-field coalescing drain**：解决同字段连续修改的写入顺序与 latest intent；不扩展为全局队列或撤销系统。
- **拒绝事件总线/动态表单/Repository-UseCase 层叠**：当前只有一个页面与一组固定字段，直接 module/store/component 边界更清晰。
- Review 必须验证这些结构是否真实降低耦合；任何只有一层转发且没有替换/测试价值的多余 wrapper 应删除。

## 代码上下文与影响假设

- `src/stores/app.ts` 和 `src/app/App.vue` 继续拥有有效主题/i18n 应用机制；设置 store 不直接操作 DOM 或 i18n global。
- `src/main.ts` 是所有 production/demo adapter 的 composition root。
- `src/features/download-center/store.ts` 的 parse/apply/clear 是媒体默认值影响面；现有用户本次选择优先于后台设置同步。
- `src-tauri/src/lib.rs` 统一 plugin、manager 与 command 注册；`SettingsManager` 与 `TaskManager` 使用 `Arc` 共享读取边界。
- `TaskManager::claim_next` 是 scheduler limits 的唯一运行时消费点；`set_scheduler_limits` 没有其他调用点，可安全移除并由端口测试替代。
- code graph unavailable 状态已记录在 `artifacts/code-context.md`；fallback 影响面通过 `rg` 和关键文件读取确认。

## TypeScript 上下文

- `src/**/*.ts` 和 Vue SFC 由根 `tsconfig.json` 管理：`strict: true`、`target: ES2020`、`module: ESNext`、`moduleResolution: bundler`、`lib: [ES2020, DOM]`、`isolatedModules: true`、`noEmit: true`。
- 无 `baseUrl`/`paths` alias，所有新增导入使用相对路径。
- `src/vite-env.d.ts` 只提供 `vite/client`；Tauri dialog 类型来自安装后的 `@tauri-apps/plugin-dialog` 包，Vue/Pinia/i18n 类型来自直接依赖。
- 后端没有生成 TypeScript、protobuf、OpenAPI 或共享 schema；TS settings types 按 Rust serde DTO 字段原名镜像，并通过 fixture/serialization tests 绑定。
- `SettingsPatch` 的 mapped discriminated union 必须在 strict 模式下保证 field/value 关联，不能退化为 `{field: SettingKey; value: unknown}`。

## API 与数据合同

### 权威来源

- 产品字段/默认/范围：`biliCatch_PRD.md` 模块六与 `requirements/modules/settings.md`。
- IPC DTO：Rust `models/settings.rs` 的 serde camelCase field table，是运行时权威；无 backend-owned TS declaration。
- plugin API：Tauri v2 官方 store/dialog 文档及锁定的 2.x crate/package 声明。
- 前端直接消费 Rust DTO 语义，不因本地偏好重命名；raw persisted JSON 仅通过 Rust migration adapter 规范化。

### `get_settings_snapshot`

- 协议：Tauri invoke command，无 request body。
- 成功：

```text
{
  schemaVersion: 1,
  revision: number,
  values: SettingsValues
}
```

- `values` 的 11 个字段全部 required、不可 null；字符串/枚举/数字/boolean 语义见字段合同。
- 失败：`AppError`，当前模块使用 `E_INTERNAL` + stable details；UI 显示本地化 load error。
- loading：前端 `loading` skeleton；没有 empty success 状态。

### `update_setting`

- 协议：Tauri invoke command，args 为 `{ request: { patch: SettingsPatch } }`。
- `SettingsPatch` 为 field/value 关联的 union；每次只改一个字段。
- 成功：返回完整、revision 增加 1 的 `SettingsSnapshot`。即便请求值等于当前值，也允许返回当前 snapshot 且不增加 revision；实现需选定 no-op，并固定为“不写盘、不加 revision”。
- 失败：非法值为 `E_INTERNAL/SETTINGS_INVALID_VALUE` 或 `SETTINGS_PATH_INVALID`；I/O 为 `E_INTERNAL/SETTINGS_STORE_UNAVAILABLE`。失败不得改变 snapshot revision、manager 内存、plugin cache 最终值或磁盘。
- 前端不实现自动时间退避重试；用户重新修改或点击页面重试触发下一次尝试。

### `SettingsStorePort`

- `load_raw() -> Result<Option<serde_json::Value>, AppError>`。
- `save_document(&SettingsDocument) -> Result<(), AppError>`。
- adapter 固定 path `settings.json`、key `document`、auto-save disabled。
- store cache rollback 属于 adapter 内部契约，不暴露到 manager/command。

### `DirectoryPickerPort`

- 输入：当前路径作为 `defaultPath`。
- 调用：`open({ directory: true, multiple: false, defaultPath })`。
- 返回：单个 desktop path 字符串或 `null`；数组一律视为 adapter contract violation。
- plugin reject 经 adapter 归一化成 `AppError`；取消不当作错误。

### `ReleaseActionsPort`

- `checkForUpdates()` 返回 `{status:"latest", currentVersion}` 或 `{status:"available", currentVersion, latestVersion}`。
- `openLicenses()` 成功返回 void。
- 当前 production deferred adapter 只抛 stable details；demo/test 可注入各分支。后续 system-release 必须保持接口或经同一 adapter 转换。

## 上下文与依赖来源

- 请求与 PRD：`request.md`、`artifacts/prd-snapshot.md`、`requirements/requirement-map.md`、`requirements/modules/settings.md`。
- 设计：`design/page-design.md`、`design/architecture-design.md`。
- 代码：`src/app/App.vue`、`src/main.ts`、`src/stores/app.ts`、download-center store/contracts、task manager/ports、Rust composition root。
- 工具链：`tsconfig.json`、`src/vite-env.d.ts`、`package.json`、`src-tauri/Cargo.toml`、capability 与 Tauri config。
- 官方插件依据：[Tauri Store plugin](https://v2.tauri.app/plugin/store/)、[Tauri Dialog plugin](https://v2.tauri.app/plugin/dialog/)。

## 边界情况

- 首次运行无文档：创建 V1 defaults，显式保存成功后返回 revision 0；保存失败为 load error。
- 文档缺字段/含未知字段/单字段非法：迁移为完整合法 V1，未知字段丢弃，非法字段回退默认并写回。
- 文档整体不可反序列化或 store 文件 I/O 失败：不覆盖原文件，返回 load error；不得自动清空用户设置。
- 默认安装目录下 BiliCatch 不存在：创建；无法创建则 load error。安装目录解析失败时不拼接 `~`，返回 load error。
- 用户在 picker 取消：不调用 update、不改变状态。
- 用户选择相对/不存在/文件路径：Rust 拒绝并回退 UI。
- 同字段快速 A->B->C：允许不保存 B，但最终 confirmed/disk 必须 C；A 的迟到结果不能覆盖 C。
- 两字段并行更新：最终文档同时包含两者，revision 单调，无 lost update。
- save 失败发生在 plugin cache set 后：adapter 恢复旧 cache；后续成功保存不得意外带入失败 candidate。
- theme/locale 保存失败：当前页面、顶栏、侧栏和 Naive UI theme 同时恢复；焦点不丢失。
- preferred quality 不存在或 requiresLogin：退到首个可用，不产生不可入队状态。
- 设置改并发时已有活动任务超过新上限：不终止活动任务；在数量降到新上限前不认领新任务。
- 页面切走时保存仍在进行：根 store 完成工作；返回设置页显示最终状态/值。
- demo mode：所有路径和状态可观察，但不得写真实 plugin store 或打开原生 dialog。

## 验收标准

- **SET-AC-01** `/settings` 展示下载、外观、系统、关于四组和全部 14 个设置/展示/动作项，无 EmptyState、tabs、accordion 或页级卡片。
- **SET-AC-02** 页面在 `1280/900/800` 及最小应用宽度下无横向溢出、文字/控件重叠和不可达操作；`<760px` 行布局单列。
- **SET-AC-03** 两个目录按钮调用单目录 picker；取消不改值，合法路径保存，非法路径显示行级错误并回退。
- **SET-AC-04** 最大并发 slider 为 `1-10` 默认 3；连接数为 `1-32` 默认 8；拖动实时显示、释放/键盘提交只保存一次最终值。
- **SET-AC-05** 视频默认包含 PRD V1 8 个 quality 值且默认为 1080P；音频为 MP3/M4A/FLAC 且默认 MP3。
- **SET-AC-06** 主题 `light/dark/system` 与语言 `zh-CN/en-US` 立即影响全局 UI，保存失败完整回滚。
- **SET-AC-07** 完成通知和自动更新默认开；关闭行为默认最小化到托盘；所有修改即时保存并有 saving/saved/error 状态。
- **SET-AC-08** 同字段连续变更最终只确认最新意图，旧响应/定时器不覆盖新值或新状态；不同字段并行无 lost update。
- **SET-AC-09** 重启或重新初始化恢复完整 V1 设置；缺失/非法字段按迁移规则补齐，未知字段不进入公开 snapshot。
- **SET-AC-10** plugin-store 写失败不改变 manager snapshot、revision、plugin cache 最终值或磁盘，并可在下一次写入恢复。
- **SET-AC-11** 下载中心在无当前用户覆盖时使用 confirmed 下载目录和媒体默认；不可用默认安全退到第一个可用选项。
- **SET-AC-12** 任务认领动态读取最大并发和连接数；连接数 32 可用，33/0 被拒绝；降低并发不终止已运行任务。
- **SET-AC-13** About 显示真实 `AppInfo.version`；检查更新覆盖 checking/latest/available/error；许可失败不打开空目标。
- **SET-AC-14** release 未配置时使用稳定 details 显示受控错误，不误报最新、不硬编码 endpoint/license URL。
- **SET-AC-15** 全部控件有程序化标签、键盘路径、焦点环；图标按钮有 tooltip/aria-label；保存状态 live region 不朗读 slider 每个刻度。
- **SET-AC-16** 中文和英文文案完整；UI 不显示 raw backend message、OS error、settings JSON 或 details 常量。
- **SET-AC-17** 组件不 import Pinia/Tauri/IPC，所有 Tauri 访问位于 adapter；根 effect sink 是唯一跨 store 设置同步入口。
- **SET-AC-18** TypeScript typecheck、前端单测、Rust fmt/check/test、生产 build 全部通过；新增 contract fixture 验证 serde camelCase、枚举、默认和 patch field/value 对齐。
- **SET-AC-19** demo 设置页覆盖 loading/ready/load-error/save-error/update states，并通过 desktop/narrow 视觉截图与 overflow/overlap 检查。
- **SET-AC-20** capability 只新增 `dialog:allow-open`；WebView 不获得 store/updater/shell 的未使用权限，设置/错误/日志中无 credential。

## 人工评审与交接

- 本规格需要用户明确批准后才能进入 plan；批准前 `settings` 的 `spec_approved` 保持 false。
- plan 必须把 SET-AC-01..20 映射到前端 contract/store/UI、Rust manager/store、consumer integration、权限与验证任务。
- execute 若发现 plugin-store 2.x API 无法保持同步 save + cache rollback 事务语义，必须回到 architecture-design/spec，不得静默改成前端直写或 auto-save。
- 设置模块 review PASS 后，交接给 `audio-download`，其只能消费设置 provider，不得读取 store 文件。

## 风险

- 新增 store/dialog 依赖需要网络安装和 Tauri capability 生成/校验；计划需单独验证 lockfile 与权限。
- 设置模块会触及已完成的 download-center 和 task manager，回归面高于纯页面；必须保留其全部现有测试并补交叉消费测试。
- 根级 hydrate 的异步主题切换可能有短暂闪烁；视觉验证需观察，但不得以阻塞启动换取同步文件 I/O。
- plugin-store cache 回滚是容易遗漏的失败路径；必须使用可失败 store port 单测，不依赖真实磁盘偶然行为。
- updater/license 的真实可用性受后续外部配置约束；当前模块只能交付可信受控降级与稳定接口。
