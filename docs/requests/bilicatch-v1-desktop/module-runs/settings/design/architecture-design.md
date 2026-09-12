# 架构设计：设置管理

## 交付单元标识

`settings`

## 架构目标

为 `/settings` 建立一个版本化、可校验、可迁移且可被后续下载与系统模块消费的设置权威源。前端负责页面交互、即时主题/语言效果与逐字段保存反馈；Rust 负责默认路径解析、值域校验、事务式持久化和运行时设置提供。任何 Vue 组件都不得直接访问 Tauri transport、store 文件或原生插件。

## 架构范围与触发条件

本模块包含：

- 四组设置页面及其 11 个可持久字段、3 个关于区动作/展示。
- `tauri-plugin-store` 持久化 adapter、默认值补齐和 V1 schema 迁移入口。
- `tauri-plugin-dialog` 目录选择 adapter。
- 前端设置 store、逐字段保存状态、同字段连续修改合并、主题/语言即时应用和失败回滚。
- 下载中心默认目录、视频清晰度、音频格式的消费边界。
- 任务调度对最大并发任务数和单任务连接数的动态读取边界。
- 当前版本复用既有 `AppInfo`；更新检查与许可入口定义稳定 port，实际发布配置由 `system-release` 注入。

本模块不包含：

- 下载执行器、通知发送、关闭窗口拦截、托盘和自动更新安装流程。
- 更新端点、公钥、许可 URL 或发布凭据的硬编码。
- 用户 Cookie 或其他秘密信息的存储。
- 通用表单框架、全局事件总线或为未来 schema 假设建立的插件体系。

## 上游输入与假设

- 页面结构以已批准的 `design/page-design.md` 为准：一个连续页面、四个 section、稳定三列设置行、窄屏单列、无手动保存按钮。
- `src/stores/app.ts` 当前拥有运行时 `themePreference` 与 `locale`，`src/app/App.vue` 已负责主题和 i18n 同步；本模块保留该根级效果边界。
- `src/features/download-center/store.ts` 当前把输出目录写死为 `Downloads`，解析成功后总选首个可用清晰度，尚未消费用户默认值。
- `TaskManager` 当前内部保存 `(3, 8)`，但 `set_scheduler_limits` 错误地把连接数也限制到 `1-10`；本模块必须按 PRD 改为 `1-32`，并消除重复运行时配置源。
- 仓库尚未安装 store/dialog/updater 插件；本模块只新增 store 与 dialog。updater 的真实 adapter 留给 `system-release`。
- code graph 能力此前已探测为不可用，本阶段沿用 durable fallback，通过 import、composition root、command 注册和 service 调用点的静态追踪完成影响分析。

## 模块边界设计

### 前端设置领域

`src/features/settings/` 是设置页面的公开功能边界：

- `contracts.ts` 定义前后端共享字段语义、snapshot、typed patch 和页面状态类型。
- `service.ts` 只封装 `get_settings_snapshot` 与 `update_setting` IPC，并统一 `AppError` 归一化。
- `store.ts` 是前端设置值与逐字段保存状态的唯一 owner；组件不得自行持久化或实现回滚。
- `injection.ts` 暴露设置持久化 service、目录选择 port 与发布动作 port。
- `effects.ts` 只定义 `SettingsEffectSink` 接口；实现位于应用 composition 层，避免设置 feature 直接依赖其他 Pinia store。
- `options.ts` 提供稳定枚举值与 i18n key，不包含硬编码展示文案。
- `components/` 只接收 typed props 并 emit 用户意图，不导入 Pinia、Tauri API 或 IPC service。

### 应用组合层

`src/main.ts` 组装 production/demo adapters 并 provide；`src/app/App.vue` 在应用生命周期内初始化设置 store。新增 `src/app/settings-effects.ts`，把已确认设置应用到 `app` store 和 `download-center` store：

- 外观候选值可即时写入 `app` store；保存失败时由同一 sink 恢复最后确认值。
- 下载默认值只在成功载入或成功保存后写入 `download-center` store，不做失败前的乐观跨模块变更。
- 设置初始化属于根生命周期，不依赖用户是否访问 `/settings`，因此下载页、任务调度和系统模块始终能消费恢复后的值。

### Rust 设置领域

`src-tauri/src/services/settings/` 是权威设置领域：

- `SettingsManager` 拥有已校验 `SettingsDocument` 与唯一事务入口。
- `validation.rs` 拥有全部字段范围、枚举和路径校验规则。
- `migration.rs` 把持久化 raw value 迁移/补齐成当前 schema；UI 和 command 不做兼容分支。
- `ports.rs` 定义 `SettingsStorePort`，基础设施细节不进入 manager。
- `SettingsManager` 同时实现窄的运行时读取能力，后续消费者只看到所需子集，不获得写权限。

### 原生基础设施

- `src-tauri/src/infrastructure/settings/store.rs` 使用 `tauri-plugin-store` 的 `settings.json`，通过 `StoreBuilder::disable_auto_save()` 关闭隐式落盘，只在固定 key `document` 下读写整个版本化文档。
- 默认下载目录和临时目录由 `AppHandle::path()` 在 composition root 解析后传给 manager；前端不展开 `~`、盘符或平台分隔符。
- 目录选择由前端 `@tauri-apps/plugin-dialog` adapter 完成，返回 `string | null`；选中路径仍必须经过 Rust update command 再校验和保存。
- 更新检查与许可打开只通过 `ReleaseActionsPort` 暴露。当前 production 组合使用明确的 deferred adapter，返回本地化可映射的 `E_INTERNAL` details；`system-release` 替换 adapter，不改设置组件或 store。

## 文件与目录结构

```text
src/
  app/
    App.vue                              # 根级设置初始化
    settings-effects.ts                 # 跨 store 效果组合
  contracts/
    media.ts                            # 共享 VideoQualityId / AudioFormat
  features/settings/
    contracts.ts                        # SettingsValues/Snapshot/Patch/UI 状态
    options.ts                          # 枚举选项与 i18n key
    service.ts                          # 设置 IPC adapter
    store.ts                            # hydrate/update/coalesce/rollback
    effects.ts                          # SettingsEffectSink port
    injection.ts                        # service/native port 注入键与 hooks
    dialog.ts                           # Tauri directory picker adapter
    release.ts                          # ReleaseActionsPort + deferred/demo adapter
    demo.ts                             # 浏览器可运行的内存设置 runtime
    settings.css                        # 页面与业务组件样式
    components/
      SettingSection.vue               # 分组标题、说明与分隔结构
      SettingRow.vue                   # 稳定 label/control/status 网格
      FieldSaveStatus.vue              # 保存中/成功/失败 live 状态
      PathSetting.vue                  # 只读路径与目录选择命令
      RangeSetting.vue                 # 滑块预览与 commit 事件
      DownloadSettings.vue             # 下载组纯展示/emit
      AppearanceSettings.vue           # 外观组纯展示/emit
      SystemSettings.vue               # 系统组纯展示/emit
      AboutSettings.vue                # 版本、检查更新、许可动作
  features/download-center/
    contracts.ts                        # 兼容 re-export AudioFormat
    store.ts                            # configureDefaults + 解析结果默认选择
  pages/
    SettingsPage.vue                   # 页面编排与非阻断通知
  locales/
    zh-CN.ts
    en-US.ts
  main.ts                               # production/demo runtime provide

src-tauri/src/
  models/
    settings.rs                         # serde camelCase DTO
    mod.rs
  services/
    settings/
      mod.rs
      manager.rs                        # 事务式 load/get/update
      ports.rs                          # SettingsStorePort
      validation.rs                     # 单一规则 owner
      migration.rs                      # schemaVersion 迁移与补齐
    tasks/
      ports.rs                          # TaskSettingsPort/SchedulerLimits
      manager.rs                        # claim 时读取设置，移除内部双份 config
    mod.rs
  infrastructure/
    settings/
      mod.rs
      store.rs                          # tauri-plugin-store adapter
    mod.rs
  commands/
    settings.rs                         # get_settings_snapshot/update_setting
    mod.rs
  lib.rs                                # 插件、manager、provider、commands 组装

src-tauri/capabilities/default.json     # dialog 最小权限
src-tauri/Cargo.toml                    # tauri-plugin-store/dialog
package.json                            # @tauri-apps/plugin-dialog
```

测试与实现文件同目录放置，沿用现有 `*.test.ts` 和 Rust module tests。组件数量可在执行时合并相邻的薄包装，但不得把 store/IPC 逻辑移进组件；不得把 `SettingRow` 提升到全局 shared，除非本模块外出现第二个真实消费者。

## 代码关系与依赖方向

```text
SettingsPage
  -> settings/components (props + emits)
  -> SettingsStore
      -> SettingsService -> IpcTransport -> commands/settings
      -> SettingsEffectSink -> app store + download-center store
  -> DirectoryPickerPort -> tauri-plugin-dialog
  -> ReleaseActionsPort -> deferred now / system-release adapter later
  -> app store.appInfo (read-only version)

commands/settings
  -> SettingsManager
      -> validation + migration
      -> SettingsStorePort -> tauri-plugin-store

TaskManager -> TaskSettingsPort -> SettingsManager (read-only scheduler subset)
future audio/video/system services -> narrow settings provider views
```

允许的依赖方向：

- page -> feature public entries -> service/ports -> transport。
- app composition -> feature stores/adapters，用于组装跨模块效果。
- Rust command -> service -> port -> infrastructure。
- 已完成任务模块只能依赖 `TaskSettingsPort`，不能依赖设置 DTO、store 文件或 Tauri plugin。

禁止的依赖方向：

- 业务组件 -> Pinia/Tauri/invoke/dialog/updater。
- settings feature -> task manager concrete type 或 task persistence。
- Rust infrastructure -> Vue/command。
- download/task/system consumer -> `settings.json` 或可写 manager internals。

## 职责拆分

| 单元 | 负责 | 不负责 |
| --- | --- | --- |
| `SettingsPage` | 组装四组、目录/更新/许可动作、通知呈现 | 字段校验、持久化、迁移 |
| 设置业务组件 | 可访问渲染、emit typed intent | store、IPC、跨模块副作用 |
| `SettingsStore` | hydrate、值/confirmed 值、逐字段状态、同字段 coalescing、回滚 | 原生路径解析、发布实现 |
| `SettingsEffectSink` | 将候选外观和已确认下载默认值应用到现有 store | 持久化、业务校验 |
| `SettingsService` | command 名、payload、错误归一化 | UI 文案、重试策略 |
| `SettingsManager` | 权威状态、候选校验、save-before-commit、revision | UI 状态、目录对话框 |
| migration/validation | schema 兼容、默认补齐、字段规则 | Tauri API、通知 |
| store adapter | 固定文件/key 的 raw I/O | 默认策略、字段解释 |
| `TaskSettingsPort` | 提供合法调度上限 | 设置写入、完整设置快照 |
| release port | 稳定检查/许可接口 | 在本模块猜测端点与密钥 |

## 函数设计与公开入口

### 前端公开入口

- `createSettingsService(transport): SettingsService`
  - `getSnapshot(): Promise<SettingsSnapshot>`
  - `update(patch: SettingsPatch): Promise<SettingsSnapshot>`
- `useSettingsStore()`
  - `initialize(service, effects): Promise<void>`：幂等 hydrate；失败保留 retry 能力。
  - `retryInitialize(): Promise<void>`：复用已注册依赖。
  - `preview<K>(field, value): void`：仅供滑块更新画面值。
  - `commit<K>(field, value): Promise<void>`：更新 desired 值并启动/复用单字段 drain loop。
  - `applySnapshot(snapshot): void`：只接收非旧 revision，并更新 confirmed 值。
  - `resetForTests(): void` 或标准 Pinia 重建：不得暴露生产清库操作。
- `SettingsEffectSink`
  - `applyAppearance(values): void`：主题/语言候选值即时生效。
  - `applyCommitted(values): void`：成功 load/save 后同步下载默认值及最终外观。
- `DirectoryPickerPort.selectDirectory(initialPath): Promise<string | null>`。
- `ReleaseActionsPort.checkForUpdates(): Promise<UpdateCheckResult>`。
- `ReleaseActionsPort.openLicenses(): Promise<void>`。

### Rust 公开入口

- `SettingsManager::load(store, defaults) -> Result<Self, AppError>`：读取 raw 文档、迁移、校验，并在需要时持久化规范化 V1 文档。
- `SettingsManager::snapshot() -> SettingsSnapshot`：返回 clone，不暴露锁或 store。
- `SettingsManager::update(patch) -> Result<SettingsSnapshot, AppError>`：锁内 clone candidate -> apply/validate -> revision + 1 -> save -> commit；save 失败不改变内存状态。
- `SettingsManager::scheduler_limits() -> SchedulerLimits`：只返回 `max_concurrent_tasks` 与 `connections_per_task`。
- `get_settings_snapshot(State<Arc<SettingsManager>>)` 与 `update_setting(request, State<Arc<SettingsManager>>)` 为唯一 IPC 写入口。
- `TaskManager::claim_next` 每次认领任务时通过 `TaskSettingsPort` 读取最新合法值；移除 `scheduler_config` 与 `set_scheduler_limits`，避免第二权威源。

## 状态归属与数据流

### 初始化

```text
App mounted
  -> SettingsStore.initialize
  -> get_settings_snapshot
  -> SettingsManager.snapshot
  -> store.applySnapshot
  -> effects.applyCommitted
      -> appStore theme/locale
      -> downloadCenterStore.configureDefaults
  -> SettingsPage ready（无论当前路由是否为 settings）
```

若 load 失败，设置 store 标记 `load_error`，保留内建安全默认值用于主题/i18n，但不得把这些默认值伪装为已持久化成功。用户重试只重跑初始化，不重建整个应用。

### 普通字段更新

```text
component emit(field, value)
  -> store.commit: desired[field] = value
  -> per-field drain loop（若已运行则只更新 desired）
  -> service.update(latest desired)
  -> Rust validate -> persist candidate -> commit -> snapshot
  -> frontend accepts current response
  -> confirmed[field] = snapshot value
  -> effects.applyCommitted
  -> 若 desired 已变化，继续下一轮；否则 saved -> idle
```

每个字段最多一个 drain loop；不同字段可并行，Rust manager 的单事务锁保证整个文档不会 lost update。同字段连续变化被合并为“当前写入完成后提交最新 desired”，不依赖网络完成顺序猜测。

### 主题/语言更新

`commit` 在发 IPC 前调用 `effects.applyAppearance(candidate)`。成功后进入 confirmed；失败且没有更新的 desired 时恢复该字段的 confirmed 值并再次调用 `applyAppearance`。若失败期间用户又选择了新值，只回退失败 generation，不覆盖更新候选。

### 滑块与目录

- 滑块 `input` 只调用 `preview`；`change`、pointer/touch commit 或键盘失焦调用一次 `commit`。
- 目录按钮 pending 由页面局部状态拥有；picker 返回 `null` 表示取消。字符串只有通过 `commit` 并经 Rust 路径校验后才成为 confirmed 设置。
- 路径失败回退到 confirmed 值；错误对象只用于本地化映射，不把原始 OS 错误或完整 store 内容展示给用户。

## 数据结构与类型策略

### 共享前端媒体类型

`src/contracts/media.ts` 定义：

- `AudioFormat = "mp3" | "m4a" | "flac"`。
- `VideoQualityId = "16" | "32" | "64" | "80" | "112" | "120" | "125" | "127"`，分别对应 PRD 的 360P、480P、720P、1080P、1080P+、4K、HDR、8K。
- 下载中心原 `contracts.ts` re-export `AudioFormat`，保持现有 import 兼容；设置与下载不各自复制枚举。

### 设置文档

```text
SettingsDocument / SettingsSnapshot
  schemaVersion: 1
  revision: non-negative integer
  values:
    downloadDirectory: string
    temporaryDirectory: string
    maxConcurrentTasks: integer 1..10
    connectionsPerTask: integer 1..32
    defaultVideoQuality: VideoQualityId = "80"
    defaultAudioFormat: AudioFormat = "mp3"
    theme: "light" | "dark" | "system" = "system"
    locale: "zh-CN" | "en-US" = "zh-CN"
    notifyOnComplete: boolean = true
    closeBehavior: "minimizeToTray" | "exit" = "minimizeToTray"
    autoCheckUpdates: boolean = true
```

下载目录默认值由系统 Downloads 目录追加 `BiliCatch`，临时目录由系统 temp 目录给出；二者不在前端常量中伪造。目录不存在时 manager 可创建 BiliCatch 默认下载目录；用户选择值必须是绝对且存在的目录。

TypeScript `SettingsPatch` 使用由 `SettingsValues` 映射出的 discriminated union：每个 `field` 只能携带对应类型的 `value`。Rust 使用 `#[serde(tag = "field", content = "value", rename_all = "camelCase")]` 的 enum 镜像该契约，避免通用 JSON value 进入 manager。

`FieldSaveState` 为 `idle | preview | saving | saved | error` 与可选 `AppError`；仅前端 UI 使用，不跨 IPC。Rust `revision` 用 `u64`，前端用安全整数并只比较大小，不做业务算术。

### 迁移策略

- 持久化只写当前 `schemaVersion=1` 的完整文档。
- load adapter 返回 raw JSON value；migration 层按版本处理，缺失字段用运行时 defaults 补齐，未知字段丢弃。
- 可识别字段中的非法值不静默接受：启动迁移时回退该字段默认值并保存规范文档；整个文件无法读取或写回失败则返回 `load_error`。
- 未来 V2 必须增加显式 `v1 -> v2` 函数与夹具测试，不能在组件或 serde default 中隐式改变语义。

## 契约与 Adapter 边界

### IPC 契约

| Command | 输入 | 成功结果 | 错误所有者 |
| --- | --- | --- | --- |
| `get_settings_snapshot` | 无 | 完整 `SettingsSnapshot` | manager/store adapter -> `AppError` |
| `update_setting` | `{ request: { patch } }` | 新 revision 的完整 snapshot | validation/store adapter -> `AppError` |

后端 serde camelCase 是权威字段名；前端保持同名直用，不建立只为命名偏好的 mapper。只有 store raw schema 与当前 settings DTO 之间存在 migration adapter。

稳定 `details` 建议值为 `SETTINGS_INVALID_VALUE`、`SETTINGS_PATH_INVALID`、`SETTINGS_STORE_UNAVAILABLE`、`UPDATE_NOT_CONFIGURED`、`LICENSES_NOT_CONFIGURED`，UI 只按 details/i18n 映射，不展示后端英文 message。PRD 没有专属设置错误码，因此使用 `E_INTERNAL`，不挪用 E001-E010 的既有业务语义。

### 插件与 capability

- Rust 注册 `tauri_plugin_store::Builder::default().build()` 与 `tauri_plugin_dialog::init()`。
- 前端目录 adapter 是 `@tauri-apps/plugin-dialog` 的唯一导入点。
- `default.json` 只加入目录选择所需最小 dialog permission；前端不直接使用 store plugin，因此不开放 store permission 给 WebView。
- 更新/许可 adapter 当前不申请 updater/shell 新权限；权限随 `system-release` 的真实实现一并评审。

### Demo 与测试替身

- demo settings service 使用内存 snapshot 与相同校验范围，支持 query 控制 load/save failure，用于浏览器视觉验证。
- demo directory picker 返回确定路径或取消；demo release actions 可返回 `latest/available/error`。
- production fallback 不能偷偷切换到内存持久化；Tauri 不可用时必须显式报错，只有 `demo=1` 使用 demo runtime。

## Pattern 决策与拒绝方案

### 采用：Adapter / Port

- **问题**：Tauri IPC、plugin-store、dialog 和未来 updater 的 API 与 UI/领域契约不同，并且 demo/test 需要可替换实现。
- **选择**：前端 service/native ports 与 Rust `SettingsStorePort`。
- **为何直接调用不足**：直接从组件调用插件会散布权限、错误归一化和测试知识；manager 直接依赖 plugin store 会阻碍迁移与失败事务测试。
- **隔离轴**：运行环境、持久化实现、原生插件可用性。
- **停止信号**：如果某个 port 永远只有一个无状态函数且没有替身需求，应降为普通函数模块，不保留类层级。

### 采用：Candidate transaction

- **问题**：即时保存失败不能让内存权威值与磁盘分叉。
- **选择**：沿用任务模块已验证的 clone candidate -> validate -> persist -> commit 结构。
- **为何直接原地修改不足**：原地修改后 save 失败需要易错的逆向补偿，并会让任务设置 provider 短暂读到未持久值。
- **隔离轴**：持久化成功/失败。
- **停止信号**：这是 manager 内的直接控制流，不抽象成通用 transaction framework。

### 采用：每字段 coalescing drain loop

- **问题**：滑块、select 或 switch 在一次保存未完成时可能再次改变，旧响应不得覆盖新意图。
- **选择**：每字段至多一个 worker，保存当前 desired，完成后若 desired 已变化再保存最新值。
- **为何 generation-only 不足**：忽略旧响应并不能阻止旧写在 Rust store 中最后落盘；串行 drain 同时保证写入顺序与 UI latest-wins。
- **隔离轴**：同一字段的快速连续用户意图。
- **停止信号**：不建立全局 command bus、撤销栈或跨字段队列。

### 明确拒绝

- 不使用 Repository + Service + UseCase 的多重前端类封装；一个 typed service 与 Pinia store 足够。
- 不使用事件总线同步主题、语言和下载默认值；根 composition sink 是显式调用且只有一个真实协作点。
- 不让前端直接写 `tauri-plugin-store`；否则 Rust 调度和后续系统能力会形成第二读取/校验路径。
- 不建立字段组件 schema 驱动的动态表单；11 个字段的交互差异明显，显式组件更易读、可访问和测试。
- 不在设置模块实现 updater Strategy；只有一个未来 production provider，先保留 port 与 deferred adapter。

## 可读性与维护护栏

- 字段名统一使用 `downloadDirectory`、`temporaryDirectory`、`maxConcurrentTasks`、`connectionsPerTask`、`defaultVideoQuality`、`defaultAudioFormat`、`theme`、`locale`、`notifyOnComplete`、`closeBehavior`、`autoCheckUpdates`；不得出现 `threadCount`、`lang`、`downloadPath` 等同义分叉。
- 范围、默认值和枚举只在 Rust validation/defaults 与共享 TS contract/options 各有一个平台必要定义，并用 serde/contract tests 绑定；组件不重复判断范围。
- `SettingsPage.vue` 只保留页面编排和通知，单文件不得吸收字段保存算法或插件调用。
- 后端 manager 不持有 `AppHandle`；原生 store/path 通过构造参数进入。
- 当前锁定的 Rust plugin-store API 中 `get/set/save` 为同步操作，manager 使用标准 mutex，锁内不出现 await。store adapter 关闭 auto-save；`save_document` 在 `set(candidate)` 前保存旧 cache value，显式 `save()` 失败时恢复旧 cache，防止失败候选在后续退出或保存中意外落盘。
- 所有 option label、状态、tooltip、错误映射同时补齐中文和英文；测试不依赖任一语言的 DOM 结构偶然值。
- 不记录完整设置文档日志；路径不是秘密但仍不进入无必要错误详情，防止用户名/目录结构泄露。

## 架构风险

- plugin-store 的 cache 与磁盘是两层状态；adapter 的“关闭 auto-save + 保存失败恢复旧 cache”必须有失败注入测试，否则 candidate transaction 只保护 manager 内存而未保护后续落盘。
- 根级异步 hydrate 可能先短暂使用系统主题再切到持久主题；实现应尽早初始化并保持布局稳定，视觉验证关注闪烁但不阻塞首屏壳层。
- 把默认清晰度 ID 固定为 B 站 ID 会受上游枚举变化影响；允许 parser adapter 保留未知 ID，但设置 UI 只提供 PRD V1 的已知集合，消费时找不到则退到首个可用值。
- 动态并发变更只影响后续任务认领，不强制暂停已运行任务；规格必须把这一时序写成可观察规则。
- deferred 更新/许可 adapter 会在当前模块显示“尚未配置”的错误；`system-release` 完成前这是明确受控降级，不得误报“已是最新”。

## 未决架构问题

- 更新端点、签名公钥、许可目标与“有更新”后的安装确认仍由 `system-release` 决定；稳定 port 和 UI 状态已固定，不阻塞设置持久化实现。
- 通知开关、关闭行为、自动更新开关的实际系统副作用分别由后续音视频执行和 `system-release` 消费；本模块只保证权威值可读取及重启恢复。
- 当前 Rust adapter 的同步 `get/set/save` 与 `disable_auto_save` 已按 Tauri v2 官方文档和当前 2.x API 源码确认；若依赖解析到不兼容 major，必须回到架构设计而不是在 execute 中改变事务语义。
