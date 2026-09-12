# 代码上下文：BiliCatch 基础模块

## 官方脚手架

- 生成器：`create-tauri-app` 4.7.4，官方 Vue + TypeScript 模板，Tauri 2，npm。
- 运行时：Node 24.19.0，npm 11.17.0。
- 生成后的基础生产构建已通过：Vue 类型检查与 Vite 构建均成功。
- 当前环境 PATH 中没有 `cargo`/`rustc`，因此 Rust 门禁仍需在 FND-06 如实判定。

## TypeScript 闭包

- 根入口：`tsconfig.json`，没有 extends；通过 project reference 引用 `tsconfig.node.json`。
- 应用配置：`strict: true`、`target: ES2020`、`module: ESNext`、`moduleResolution: bundler`、`lib: [ES2020, DOM]`、`isolatedModules: true`、`noEmit: true`。
- `types`、`baseUrl`、`paths` 均未配置；实现使用相对导入，不假定 alias。
- `src/vite-env.d.ts` 只保留 `vite/client` 声明；Vue SFC 类型由 Vue/Vite 工具链提供。
- Node 配置：`tsconfig.node.json`，`composite: true`、`module: ESNext`、`moduleResolution: bundler`。

## 前端依赖事实

- Vue 3.5、Vue Router 5、Pinia 4、vue-i18n 11、Naive UI 2、`@lucide/vue` 1。
- 测试：Vitest 5、Vue Test Utils 2、jsdom 30。
- scaffold 安装解析到 Vite 8.2.2；按批准规格保留兼容组合，不强制降级到 PRD 示例版本。

## Tauri 与 Rust 上下文

- `src-tauri/Cargo.toml` 使用 edition 2021、Tauri 2、serde/serde_json；库 crate 为 `bilicatch_lib`。
- `tauri.conf.json` 使用 `com.bilicatch.app`，dev server 固定端口 1420。
- capability 基线只含 `core:default` 与 `opener:default`；基础健康命令不增加额外权限。
- Rust DTO 使用 serde camelCase；命令只在 `src-tauri/src/lib.rs` 组合注册。
- Rust stable 1.98.1 已通过 rustup 最小安装；`cargo` 使用 `C:/Users/yangjianlin/.cargo/bin/cargo.exe`，当前 shell 不依赖 PATH 更新。

## 解析模块增量上下文

- 新增 TS feature contract 是 `parse_video` 前端唯一消费形状；服务端没有可复用 TypeScript/protobuf。
- Rust `models::parse` 通过 integration serde test 与 TS 字段对齐；B 站 raw JSON 后续只进入 infrastructure 私有 structs。
- task draft store 只交接意图字段，不拥有 id/status/progress/persistence。

## 解析模块代码图与依赖回退（review）

- graph status：missing。仓库扫描 `codegraph`、`code-graph`、`scip`、`lsif` 以及当前可调用工具，未发现代码图运行时、配置或仓库指定的 bootstrap。
- bootstrap attempt：已执行仓库配置与工具可用性探测；没有兼容的仓库安装入口，因此未引入与交付无关的第三方图谱工具。
- fallback：使用 `rg` 对 import/export、Tauri command 注册和 service/store 使用点进行入口到叶节点追踪。
- 前端流向：`main.ts` 组装 service/notifier -> `DownloadPage` 编排 -> download-center store/components；提交后进入 task-drafts store 与 router。
- Rust 流向：`lib.rs` 注册 command/state -> `commands::parse` -> `ParserService` -> `BilibiliPort` -> client/adapter/wbi -> stable models。
- side effects：网络访问只在 Rust client；解析结果与 WBI key 只在进程内缓存；下载任务草稿只在 Pinia 会话态；当前模块没有文件写入、认证持久化或下载执行。
- remaining blind spots：在本模块范围内，经人工调用链与静态边界扫描后无剩余盲点。

## 事故恢复记录

- 官方生成器以 `--force` 在非空根目录执行时移除了 PRD 与 `docs/`，与计划中的保护要求冲突。
- 已从当前 Codex 会话日志中的原始 UTF-8 PRD 输出和 8 次成功补丁逐一恢复；失败补丁未重放。
- 恢复后 PRD 为 818 行，生命周期状态仍为已批准 plan 后的 execute，不伪造或跳过审批。

## 任务管理模块增量上下文

- code graph 仍为 missing，沿用已记录的 bootstrap 探测结果；本阶段通过 `rg` 与入口文件读取追踪实际依赖。
- 现有 `/tasks` 只有共享 EmptyState，占位页不包含需兼容的本地状态或副作用。
- 上游交接为 `useTaskDraftsStore().items: DownloadTaskDraft[]`；解析页在 append 后跳转 `/tasks`，任务模块需保留 `append/items/clear` 兼容面并增加幂等 handoff 元数据。
- Tauri 现有 composition root 只 manage ParserService；commands 集中在 `commands/mod.rs`。任务模块可拆出 `commands/tasks.rs`，但必须继续由 `lib.rs` 统一 manage/register。
- `tauri-plugin-opener` 已安装并具有 `opener:default` capability；打开任务文件应由 Rust 先按 task id 解析可信路径，前端不传任意路径。
- 当前无任务持久化、event adapter、Tokio 显式依赖或下载执行器；这些是 task-management 新边界，不可假定已有实现。

## 任务管理模块最终结构（review）

- 前端流向：`main.ts` 组装 task service/event source -> `TasksPage` 编排 -> Pinia store 合并权威集合 -> TaskToolbar/TaskRow 以 typed props/emits 呈现；组件不导入 Tauri 或 store。
- Rust 流向：`lib.rs` 组装 JSON store/event/cleaner -> commands -> `TaskManager`；状态规则、执行更新、进度合并和 ports 分文件，真实下载算法由后续 executor 实现。
- side effects：JSON 读写只在 `JsonTaskStore`，文件删除只在 cleaner，可信 opener 路径只从已完成 task 解析，Tauri emit 只在 adapter。manager 使用 candidate transaction 保证 save 成功后才提交内存/emit。
- 实时一致性：全局 sequence 处理跨任务事件顺序，task revision 防延迟进度覆盖命令结果，delete tombstone 防删除响应与 removed event 之间的旧 progress 复活任务。
- 高频进度：`ProgressCoalescer` 将窗口内普通进度保留为内存最新快照，每秒最多一次持久化/事件；显式及终态 flush 提交最终快照，状态变化不延迟。
- graph status 仍为 missing；`rg` import/注册/事件名扫描与测试替身覆盖后，本模块无剩余结构盲点。

## 认证模块执行上下文

- graph status 仍为 missing；沿用已记录的仓库探测与 `rg` 手工调用链，不引入与交付无关的新图谱运行时。
- 前端 governing config 仍为根 `tsconfig.json`：strict、ES2020、DOM、ESNext/bundler、isolatedModules、noEmit、无 alias；`vite-env.d.ts` 只引入 `vite/client`。
- 当前 `/login` 与 `TopBar` 是 UI 入口，`App.vue`/`main.ts` 是根级认证订阅的 composition owner；现有组件不得直接持有 Tauri transport。
- Rust composition 由 `lib.rs` 统一 manage/register；认证将新增 models/services/infrastructure/commands，并通过窄 `AuthContextProvider` 接入 `ParserService`。
- `ParserService` 当前在 normalization 后读取仅按输入键控的 result cache；认证任务必须将远端 session validation 移到 cache lookup 前，并把 auth revision 加入 cache key。
- secret side effect 只允许位于 Rust Bilibili auth adapter 与 keyring adapter；public snapshot、TypeScript、Tauri event、日志、错误与普通文件均不得携带 credential。
- 后续检查：AUTH-01 固定跨端合同；AUTH-02 至 AUTH-05 在实际 client/parser seam 上复核架构可行性；AUTH-06 再确认根订阅不干扰 task event 生命周期。

## 认证模块 Review 回流结构补充

- B 站 nav 的明确匿名语义同时存在于顶层 `code=-101` 与 `data.isLogin=false`；二者统一由 raw adapter 映射为 `SessionValidation::Invalid`，manager/parser 不解释外部数字码。
- 根 auth store 的 listener 所有权必须在异步 subscribe 完成并确认 lifecycle 后才写入共享 `unsubscribe`；stale 初始化自行释放返回的 listener。
- DownloadPage 只观察公开 auth 状态的 `authenticated -> anonymous` 转换并发出本地化提示，不读取 credential、generation 或 B 站 raw code。
- QR canvas 使用组件局部 render generation 隔离迟到 promise；Rust credential 补偿错误使用 manager generation + poll abort 隔离后台提交，两者都留在既有 owner 内，没有新增通用抽象。
- 手工依赖追踪确认这些修复未改变主方向：UI -> store/service -> Tauri -> AuthManager -> adapter；code graph 状态仍为 missing，无新增结构盲点。

## 设置模块架构上下文

- `/settings` 当前仍为 `EmptyState`；根 `App.vue` 已通过 `app` store 同步主题和 i18n，设置初始化必须在根生命周期完成，不能依赖访问设置路由。
- `download-center` 当前把 `outputDir` 默认成 `Downloads` 并在解析成功后选择首个可用媒体项；设置模块需通过应用组合层注入已确认默认值，不把跨 store 协调塞进展示组件。
- `TaskManager` 当前拥有私有 `(3, 8)` scheduler config；其 setter 把连接数错误限制为 `1-10`。架构改为 `TaskSettingsPort` 按认领时读取权威设置，范围固定为并发 `1-10`、连接 `1-32`。
- 现有依赖只有 opener plugin，store/dialog/updater 均未安装。当前模块新增 store/dialog；updater 与许可目标通过 deferred release port 留给 `system-release`，不申请未使用权限。
- 设置数据流固定为 Vue component -> Pinia settings store -> typed IPC service -> Rust SettingsManager -> plugin-store adapter；主题/语言候选由根 effect sink 即时应用，保存失败由同一 owner 回滚。
- code graph 状态仍为 missing；沿用已记录的探测结果，以 import、composition root、command 注册和 manager 调用点追踪完成本阶段影响确认。

## 设置模块最终结构复核

- graph status 仍为 missing；未发现仓库指定 bootstrap，沿用既有探测记录，不为最终验收引入无关运行时。
- fallback 使用 `rg` import/call-site 扫描、composition root/command 注册读取、前后端 contract tests 与全量回归。
- 最终前端流向：`App.vue/main.ts` 组装 -> `SettingsPage` 编排 -> group/field components -> Pinia settings store -> typed IPC service -> Tauri commands。
- 最终 Rust 流向：commands -> `SettingsManager` candidate transaction -> `SettingsStorePort` -> plugin-store adapter；同一 manager 通过窄 `TaskSettingsPort` 为后续 claim 提供 limits。
- 跨模块副作用：root `SettingsEffectSink` 是主题/语言与 download defaults 的唯一写入口；UI 组件不持有 Tauri、Pinia 或 IPC。
- 影响面经 settings/app/download/task 全量测试与静态边界扫描覆盖，无剩余结构 blocker；真实 release 与 executor 仍属于后续模块。

## 音频下载架构上下文

- graph status 仍为 missing；仓库没有新增图谱 bootstrap 入口。本阶段用 `rg` 追踪 `lib.rs -> TaskManager -> ports/mutations/store`、`ParserService -> BilibiliClient/raw/adapter` 以及 download-center/task UI 消费链。
- `TaskManager` 已拥有 claim、attempt、防迟到更新、并发设置、网络重试和 persisted temporary paths，但 `DeferredTaskExecutor` 尚未被 composition root 消费；当前没有后台 runner。
- active cancel 当前在 command 内立即使 attempt 失效并删除临时文件，真实 writer/FFmpeg 接入前必须改为 request -> executor stop -> cleanup -> Cancelled acknowledgement，避免进程继续写入已清理路径。
- `TaskExecutionSpec` 当前仅含 taskId/attemptId/connectionCount，无法独立执行；音频模块需加入不可变 task snapshot 与 settings temporary directory，同时保持 WebView 不可见。
- Bilibili raw audio 当前只解析 bandwidth，production source adapter需要读取 audio id、primary/backup URL、codec/container和FLAC/Hi-Res分支；raw字段继续限制在 infrastructure，任务JSON不保存时效URL或credential。
- download-center 当前把 `audioBitrates` 当选择值；音频规格将 `audioBitrateId` 固定为输出profile id，并新增source capability字段，解决MP3输出码率与B站源质量混用。
- Rust composition root 继续拥有 runner/executor/adapter创建；HTTP、文件、FFmpeg与auth side effect均通过窄port隔离，默认测试使用loopback/fake，不依赖公网或系统FFmpeg。
- 主要回归邻居是 task control/state recovery、parse serde合同、download-center默认选项与task row副信息；后续plan必须逐一绑定现有全量回归和新增竞态/安全测试。

## 音频下载最终结构（review）

- graph status仍为missing；没有仓库指定bootstrap，本阶段继续用 `rg`、composition root读取、跨端contract test、loopback HTTP和filesystem fake恢复实际调用链。
- production流向：`lib.rs -> DownloadRuntimeRunner -> TaskManager -> AudioExecutor -> ParserService/HttpByteDownloader/FsAudioWorkspace/DeferredFfmpegMediaProcessor -> TaskManager reporter`。
- 前端流向：`main.ts -> DownloadPage/TasksPage -> Pinia store + typed service -> options/components`；叶组件不持有Tauri transport、Pinia或raw backend error。
- side effects：B站HTTP与redirect校验位于bilibili/download adapters；checkpoint与原子finalize位于workspace；FFmpeg argv/spawn位于audio adapter；任务JSON/event不保存credential或时效media URL。
- 竞态所有权：TaskManager保留attempt、cancel intent和终态唯一写权；AudioExecutor只线性编排并通过reporter提交。取消与迟到Completed竞态由manager将完成回报转换为受控清理后的Cancelled。
- 职责拆分：HTTP body、cover、response helpers分别位于download子模块；executor attempt、runtime controls、progress adapter分别位于audio子模块。生产文件均不超过350行，700行HTTP文件中第318行后为同模块loopback测试。
- remaining blind spots：真实FFmpeg sidecar签名/打包和安装包退出行为需在 `system-release` 以打包产物验证；公网B站smoke默认opt-in，不作为本模块CI完成证据。
