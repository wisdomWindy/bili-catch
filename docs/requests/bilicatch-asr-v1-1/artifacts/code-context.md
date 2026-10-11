# 音频转文本代码上下文

## 上下文需求

本请求跨 Vue 路由、Pinia、IPC、Rust 状态机、任务持久化、设置迁移、FFmpeg sidecar 和 Tauri 启动装配，结构理解会直接影响数据归属和回归范围。首次需要该上下文的阶段为 `architecture-design`；代码图优先，但在没有项目配置和可用 CLI 时允许人工恢复。

## 代码图可用性检查

- 状态：`missing`
- 检测方式：仓库文件搜索；`where.exe codegraph`、`code-graph`、`scip`、`lsif`。
- 结果：未发现仓库图配置或上述 CLI；仅发现 `rust-analyzer`，它不是本工作流可直接消费的完整跨语言图产物。
- 检查时间：2026-10-10，loop iteration 4。

## 安装或引导记录

- attempted：`yes`
- 方法：检查仓库/系统是否已有团队首选 graph 工具或可复用配置。
- 结果：`fail`
- 摘要：没有仓库指定工具，不能安全猜测并引入新的图生成依赖或持久化配置。
- 下一步：使用 `rg`、模块入口、类型导入、服务构造和测试消费关系重建范围；实施期若项目新增 graph 工具再刷新本工件。

## 回退记录

- fallback used：`yes`
- 方法：按 TypeScript 导入/注入入口与 Rust `mod`/`pub use`/Tauri setup/invoke handler/trait adapter 追踪依赖。
- 原因：代码图不可用，且无团队指定 bootstrap。
- 置信度：对本次已知入口和影响面为高；盲点集中在运行时外部二进制、B 站字幕远程字段和非 Windows 打包环境。

## 相关入口

### 用户入口

- `src/app/router.ts`：Hash 路由注册。
- `src/app/navigation.ts`、`src/components/layout/Sidebar.vue`：侧栏入口。
- `src/pages/TasksPage.vue`：现有任务统一页面。
- `src/pages/SettingsPage.vue`：设置分组装配。
- `src/pages/DownloadPage.vue` 与 `src/features/download-center/`：下载选项和草稿交接。

### 内部执行入口

- `src/main.ts`：前端服务、事件源和设置 effect 注入。
- `src/services/ipc/client.ts`：通用 Tauri invoke transport。
- `src-tauri/src/lib.rs`：manager/adapter 构建、后台 runner 和命令注册。
- `src-tauri/src/commands/tasks.rs`：下载任务命令形状参考。
- `src-tauri/src/services/download_runtime/runner.rs`：持久化队列轮询参考。
- `src-tauri/src/infrastructure/audio/ffmpeg.rs`：sidecar 定位、隐藏进程、取消和输出校验参考。

### 事件和异步触发

- `src/services/ipc/events.ts`：下载任务事件订阅模式。
- Rust `TaskManager` + `TauriTaskEventSink`：下载事件 sequence/revision 模式。
- 下载 runner 完成路径：自动转写的跨域触发点。
- Tauri 系统通知 adapter：转写完成/失败通知复用点。

## 关键符号与模块

| 现有符号/模块 | 当前责任 | 本次关系 |
| --- | --- | --- |
| `AppRouteName` | 路由名称 union | 增加 `transcribe` |
| `primaryNavigation` | 侧栏项目 | 增加转写入口 |
| `IpcTransport` | invoke 抽象 | 复用，不扩散 Tauri API 到组件 |
| `DownloadTask` / `TaskManager` | 下载状态、持久化和动作 | 不改变成混合实体；提供完成输出 |
| `TaskRow` / task store | 下载任务 UI | 重构为联合展示投影的消费者 |
| `SettingsValues` / `SettingsManager` | 设置合同和持久化 | schema 扩展及兼容读取 |
| `FfmpegMediaProcessor` | 下载音频格式处理 | 复用 runner/locator 思路，新增转写预处理 adapter |
| `StrategyRegistry` | 下载模式执行选择 | 不承载 ASR；转写使用独立 scheduler |
| `AppErrorCode` | E001-E010 | 扩展 E011-E017 |
| `tauri.conf.json` externalBin | FFmpeg sidecar | 增加 whisper sidecar |

## 依赖与副作用边界

- Vue 页面通过注入的 feature service 和 event source 工作；组件不直接调用 `invoke` 或 `listen`。
- Pinia store 合并查询快照和事件，使用 sequence/revision 阻止过期覆盖。
- Rust commands 是薄边界；状态转移、校验、缓存和任务创建归服务层。
- 文件、网络、sidecar、通知、打开目录和远程字幕均归 infrastructure adapter。
- 设置写入通过一个 manager 验证并持久化；页面不直接写 store 文件。
- 下载完成到自动转写是明确的跨域 command sink，不通过隐藏全局状态或前端轮询实现。

## 影响范围

### 预计新增

- 前端：`src/features/transcription/`、`src/pages/TranscribePage.vue`、页面 CSS 与测试。
- Rust：`models/transcription.rs`、`commands/transcription.rs`、`services/transcription/`、`infrastructure/transcription/` 与契约/集成测试。
- 分发：whisper sidecar 清单、模型 manifest 和相关第三方许可记录。

### 预计修改

- 路由、导航、App route 类型、main 注入和中英文语言包。
- 下载草稿/选项、任务展示投影、设置合同/页面/schema、错误码。
- Rust module exports、lib setup/commands、下载完成 hook、通知和 Tauri externalBin。

### 验证面

- 前端合同/store/service/components/page/router/i18n。
- Rust 序列化、模型 manifest、下载/摘要、预处理、sidecar 参数/解析、状态机、缓存、导出、取消清理、设置迁移和自动转写。
- 全量 `npm test`、`npm run build`、`cargo test`、`cargo check`。
- 浏览器 demo/fixture 视觉验证以及 Windows 真实 sidecar/model smoke。

### 回归敏感邻居

- 现有下载任务序列、清空完成、暂停全部和通知。
- 设置 schema v1 的老文件读取。
- 下载中心三种模式及任务草稿序列化。
- Tauri 启动在 sidecar 缺失时的容错；应用必须能启动并引导下载/安装，而不是崩溃。
- 更新和打包流程中的 externalBin 平台命名。

## 后续检查

- 固定 whisper.cpp 版本后验证 CLI 输出参数和取消行为。
- 固定模型资产后生成并复核 SHA-256 manifest。
- 确认下载完成 runner 中最小、幂等的自动转写 hook 位置。
- 确认设置 schema 当前反序列化对新增字段的 default 行为并补迁移测试。
- 获取 B 站 CC 字幕 fixture 后固定 adapter 合同。
