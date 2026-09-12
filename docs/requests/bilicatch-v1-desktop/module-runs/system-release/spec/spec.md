# 工程规格：系统集成与发布

## 交付单元标识

`system-release`

## 背景与目标

为已完成的 BiliCatch 下载能力提供可发布桌面运行时：托盘与窗口行为、任务通知、文件对话框/拖拽入口、更新检查与签名校验、FFmpeg sidecar 注入及三平台产物配置。

## 范围

- Tauri lifecycle、托盘菜单、窗口位置/大小记忆、关闭确认和活动任务保护。
- 下载完成/失败/更新可用通知，受设置开关控制。
- 文件选择、保存位置和拖拽解析沿用现有业务入口。
- updater HTTPS 元数据获取、版本/平台匹配、签名校验、安装确认和失败反馈。
- `externalBin` 配置、FFmpeg sidecar 清单、平台打包目标和签名产物元数据。

## 不在范围

- 新业务页面、新下载协议、新任务状态。
- 视频转码、HDR、字幕、直播和额外容器。
- 签名私钥生成、证书采购、发布服务器搭建。
- 在没有可信二进制和签名材料时伪造真实发布 smoke 结果。

## 触发与启动条件

- 应用启动：恢复设置并初始化系统适配器，异步发起 updater 检查。
- 任务终态：TaskManager 产生 Completed/Failed/Cancelled 后，系统服务按设置发通知。
- 窗口关闭：无活动任务时按设置退出或隐藏到托盘；有活动任务时先确认。
- 手动更新：用户从托盘或设置触发检查；可用更新必须在安装前二次确认。
- 发布构建：CI 提供平台 sidecar、图标、签名环境变量和发布 URL 后运行 bundler。

## 功能完整行为

### 生命周期与托盘

- 托盘菜单包含显示主窗口、暂停/继续所有任务、打开下载目录、检查更新和退出。
- 关闭行为读取现有设置：最小化到托盘或退出。存在活动任务时，退出项必须确认；取消确认保持窗口和任务不变。
- 窗口位置和大小在合法范围内持久化；读取失败使用默认值，不阻塞启动。

### 通知与文件交互

- 通知只展示稳定本地化标题/正文和任务文件名，不展示 URL、cookie、stderr 或原始后端详情。
- 通知开关关闭时不触发系统通知，但任务状态仍照常更新。
- 文件选择器只返回用户明确选择的路径；拖拽输入继续交给既有解析服务，系统层不复制解析规则。

### 更新状态机

`idle -> checking -> up_to_date | available | failed`；`available -> installing -> installed | failed`。

- 检查请求必须 HTTPS；响应平台、版本和签名字段缺失视为失败。
- 版本可用时展示稳定提示，用户确认后才安装；取消回到 `available`。
- 签名验证失败、下载失败、安装器退出或取消均返回稳定错误码并清除临时更新文件。
- 检查进行中禁止重复请求；旧响应不得覆盖更新一代请求的状态。

### 发布与 sidecar

- Tauri bundler 目标固定覆盖 Windows NSIS/MSI、macOS DMG/App、Linux deb/rpm/AppImage。
- 每个平台产物的版本号来自同一发布版本源；同时生成 `latest.json` 与 `.sig`。
- FFmpeg 通过 `externalBin` 注入并由现有 locator 验证绝对路径/存在性；缺失时运行时返回 E008。
- 真实 MP4/AAC stream-copy smoke 必须在可信 sidecar 就绪后执行，当前没有该材料时只能记录 deferred。

## 设计约束

- Rust serde DTO 为 IPC 契约源，前端类型保持 camelCase 和显式 nullable/enum 语义。
- capability 只允许本模块实际使用的托盘、通知、窗口、对话框和 updater 权限。
- 平台 API、更新器、文件系统和 sidecar 均通过 adapter/port 隔离；services 不直接依赖平台 SDK。
- 不使用 shell 拼接命令；不把绝对路径、签名原文、私钥或 stderr 写入任务/日志/UI。
- 生产文件超过 350 行必须拆分；每个 adapter 只拥有一个外部副作用域。

## 项目脚手架决策

继续使用仓库现有 Tauri 2 + Vue + Pinia + i18n 脚手架；不引入新 starter 或升级依赖。允许增加 capability、infrastructure adapter、typed service 和 bundler 配置，禁止重建应用入口。

## API 与数据契约

| 契约 | 来源 | 关键字段/语义 | 边界 |
| --- | --- | --- | --- |
| `UpdateState` | Rust serde DTO | `status`, `version`, `platform`, `errorCode`；状态显式 enum | Tauri command/event adapter |
| `WindowBehavior` | SettingsManager | `closeAction`, `rememberBounds`, `notificationsEnabled` | settings service -> system service |
| `ReleaseArtifact` | 发布清单 | `platform`, `target`, `version`, `signaturePath` | updater/bundler infrastructure |
| `SidecarManifest` | 发布清单 | `name`, `platform`, `sha256`, `licensePath` | externalBin locator |

无现成 TypeScript 服务声明；前端只声明与 Rust serde 输出对应的最小类型。更新原始 JSON 先在 Rust adapter 中校验并归一化，再进入 IPC。

## 边界情况

- 无网络、超时、非 HTTPS、版本不匹配、签名错误、安装器取消和重复检查。
- sidecar 缺失、哈希不匹配、平台目标缺失、产物重名或版本不一致。
- 活动任务退出确认、通知关闭、窗口边界损坏、托盘初始化失败。
- 外部文件路径不存在或不在用户明确选择范围内。

## 验收标准

| ID | 可观察标准 |
| --- | --- |
| SR-AC-01 | 启动初始化托盘/窗口/通知，不阻塞既有下载页面。 |
| SR-AC-02 | 关闭行为按设置执行，活动任务退出前必须确认。 |
| SR-AC-03 | 通知受开关控制且不泄漏 URL、cookie、stderr、私密路径。 |
| SR-AC-04 | 窗口边界可恢复，损坏值回退默认且不崩溃。 |
| SR-AC-05 | 更新状态机拒绝重复请求、校验 HTTPS/平台/版本/签名并正确处理取消与失败。 |
| SR-AC-06 | updater 安装只在用户确认且签名验证通过后执行。 |
| SR-AC-07 | capability 权限最小化，未增加通配 WebView 或 shell 权限。 |
| SR-AC-08 | externalBin 缺失/哈希错误稳定映射 E008，临时文件清理。 |
| SR-AC-09 | Windows/macOS/Linux 产物配置、版本号、`latest.json`、`.sig` 字段一致。 |
| SR-AC-10 | 真实 sidecar MP4/AAC smoke 有可追溯证据；无 sidecar 时明确 deferred，不伪造通过。 |
| SR-AC-11 | `npm run typecheck`、`npm run build`、Rust fmt/check/test 和发布配置静态检查通过。 |

## 人工审查与交接

- 规格和计划需要用户批准后才进入 execute。
- 发布前由人工提供签名材料、发布 URL、图标和各平台 sidecar 清单。
- `system-release` review 必须附平台产物清单和真实 smoke 证据；缺少外部材料时保持 blocked/deferred，不标记完整发布。

## 风险

- 真实签名/公证和跨平台 installer 行为不能由当前 Windows 工作区替代。
- sidecar 许可证、哈希和平台架构不一致会阻塞发布。
- Tauri updater/plugin 配置与锁定依赖可能限制某些平台能力，需要在执行前验证。
