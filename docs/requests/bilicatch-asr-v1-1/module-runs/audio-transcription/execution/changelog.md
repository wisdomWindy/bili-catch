# `audio-transcription` 执行记录

## 基线

- 2026-10-10：规格与计划均已获用户批准，workflow 进入 `execute`，loop iteration 9。
- 执行方式：当前 Codex 主线程串行实施，不启用多代理或 workflow 并行。
- 合同来源：`spec/spec.md`、`design/architecture-design.md`、`plan/plan.md`。
- TypeScript 上下文：根 `tsconfig.json`，ES2020、ESNext、bundler resolution、strict、noUnused、无路径别名；`src/vite-env.d.ts` 仅引入 Vite client。
- 代码图：仍不可用，沿用 `artifacts/code-context.md` 记录的 import/mod/构造链回退分析。

## TDD 记录

### T01 合同、错误码与设置迁移

- 状态：`done`
- RED：Rust 缺少转写类型/设置字段并编译失败；前端缺少转写合同、E011-E017 与设置键，3 个 targeted suite 如预期失败；应用数据模型目录构造器测试先因 API 缺失失败。
- GREEN：新增 Rust/TypeScript 转写 IPC 合同，扩展 E011-E017；设置 schema 升级到 v2，兼容迁移 v1，加入 11 个字段、范围/路径/HTTPS 源校验；生产默认模型目录装配到 app data。
- REFACTOR：`SettingsDefaults::from_application_paths` 明确区分安装目录与应用数据目录；Rust tagged source 使用 `rename_all_fields` 固定 camelCase。
- 验证证据：`cargo test` 186 passed、1 ignored；`npm test` 198 passed；`npm run typecheck` 通过。

### T02 模型清单与本地仓库

- 状态：`done`
- RED：repository 模块缺失导致集成测试编译失败；流式 writer/cancellation API 和 reqwest downloader 分别先失败。
- GREEN：固定五模型官方 URL/字节数/SHA-256；实现扫描、显式校验、`.part` writer、SHA-256 后原子发布、取消清理、使用中删除保护、HTTPS 镜像和 HTTPS redirect 限制。
- REFACTOR：文件完整性集中在 `FileModelRepository`，reqwest 只负责网络与流式进度；环回 HTTP 入口仅 `cfg(test)` 可见。
- 验证证据：`cargo test --test model_repository` 8 passed；受控 HTTP 下载单测通过。

### T03 状态机、持久化与恢复

- 状态：`done`
- RED：状态机、manager、JSON store 三个模块缺失，集成测试如预期编译失败。
- GREEN：实现纯状态表、独立文档 schema、manager 候选提交、sequence/revision、重试/删除/清理和运行态启动恢复；实现不覆盖损坏文件的 JSON store。
- REFACTOR：创建只允许 Pending；Failed/Cancelled 重试由显式方法处理；事件严格晚于持久化。
- 验证证据：三个 targeted suites 共 6 passed。

### T04 来源检查与 FFmpeg 预处理

- 状态：`done`
- RED：缺少 `ffmpeg_preprocessor` 模块导致集成测试编译失败；随后用 runner fixture 固定 probe、分片输出、空源拒绝和清理行为。
- GREEN：实现严格格式矩阵、媒体 probe、16kHz 单声道 PCM s16le WAV 转码、30 分钟分片与静音点 ±5 秒择优、取消轮询和临时目录 cleanup guard。
- REFACTOR：独立转写 FFmpeg 适配器复用 locator/进程控制习惯，但不复用下载领域接口；stderr 限制 64 KiB，源文件只读。
- 验证证据：`cargo test --test transcription_preprocessor` 4 passed；`cargo test ffmpeg_preprocessor` 2 passed。

### T05 whisper sidecar 适配器

- 状态：`done`
- RED：sidecar 模块缺失导致合同测试编译失败；随后 fake runner 固定 auto GPU 回退和 enabled GPU 不回退行为。
- GREEN：实现 v1.8.6 平台定位、typed CLI builder、full JSON 解析、进度解析、64 KiB 诊断上限、Windows 隐藏进程与进程树取消、CPU/GPU 策略及 E014/E015 映射。
- REFACTOR：新增应用层 `Transcriber` port；runner 返回退出事实，由 adapter 统一决定 GPU 回退和业务错误，避免进程层吞掉策略。
- 验证证据：`cargo test --test whisper_sidecar` 4 passed；GPU 策略单测 2 passed。

### T06 调度、字幕、缓存与导出

- 状态：`done`
- RED：cache/export/processing/scheduler 模块缺失；manager 缺少结果原子提交与 revision 编辑接口。
- GREEN：实现稳定缓存指纹、标点偏好、分片归并、五格式渲染、200 字符冲突命名、原子导出、FIFO 去重队列、下载活跃容量降级、字幕三态端口与结果编辑。
- REFACTOR：识别缓存键与输出格式分离；纯渲染不做 I/O；manager 单次持久化结果与 Completed 任务关联。
- 验证证据：结果 pipeline 5 passed；manager 5 passed；scheduler 1 passed。

### T07 Tauri 命令、事件与自动钩子

- 状态：`done`
- RED：命令/runtime/event 模块缺失；全量 Rust 首跑出现一个既有 auth 并发测试抖动。
- GREEN：实现规格表 14 个命令、3 类事件、独立 runtime、pipeline、模型下载、动态并发、取消、下载完成后端 hook 与缺 sidecar 可启动装配。
- REFACTOR：下载事件通过通用 completion sink 转发，late-bound weak runtime 消除构造环；命令层无业务分支。
- 验证证据：`cargo check` 通过；新增 Rust suites 及 targeted auth 复现连续 5 次通过。全量 auth 抖动留待最终门禁复核。

### T08 前端 IPC、事件与 Pinia

- 状态：`done`
- RED：service/store 模块缺失导致 Vitest import 失败。
- GREEN：实现 14 命令 service、3 事件 source、注入键和 Pinia store；快照期间缓冲事件并按 sequence/revision 拒绝陈旧更新。
- REFACTOR：IPC 错误统一 normalize；页面只消费 store/service，不直接 invoke/listen；失败保留已有实体。
- 验证证据：transcription service/store 2 passed；`npm run typecheck` 通过。

### T09 转写工作台与结果编辑

- 状态：`done`
- RED：路由、工作台与组件缺失；页面测试先固定来源检查和模型门禁。
- GREEN：实现 `/transcribe`、侧栏入口、来源选择/拖放、参数表单、模型管理、任务进度、结果编辑/复制/打开以及双栏/单栏响应式布局。
- REFACTOR：来源、模型和结果拆为 props/emits 组件；图标操作均带 title；任务行避免嵌套 button。
- 验证证据：页面测试 1 passed；路由/service/store 8 passed；`npm run typecheck` 通过。

### T10 任务、设置、下载与 i18n 集成

- 状态：`done`
- RED：扩展下载意图和联合任务类型后，TypeScript 精确报告所有旧 fixture 缺字段；设置页控件数和任务投影测试先固定预期。
- GREEN：设置页接入全部 11 个转写字段；下载草稿固化自动转写预设；任务页以 `kind` 判别联合展示和分发动作；完成下载可进入转写来源；工作台及子组件完成中英文覆盖。
- REFACTOR：下载/转写 manager 与 store 保持独立，只在展示投影聚合；自动转写预设在创建下载任务时冻结，后续全局设置不追溯旧任务。
- 验证证据：TypeScript 类型检查通过；设置/下载/任务/转写 targeted suites 共 60 余项通过；Rust `cargo check --tests` 通过。

### T11 分发、真实 smoke 与全量验收

- 状态：`done`
- RED：Tauri 清单只有 FFmpeg，Windows `whisper-cli` 与运行时 DLL、MIT 许可均缺失；官方模型端点在当前网络超时。
- GREEN：加入固定 v1.8.6 官方 Windows CPU sidecar、DLL、许可和 `externalBin`/resources 映射；从镜像取得 Tiny 模型后用官方 SHA-256 `be07...1b21` 校验通过。
- 真实 smoke：SAPI 生成短英文 WAV，经捆绑 FFmpeg 转 16 kHz mono s16，实际 `whisper-cli` CPU 推理输出 1 个非空 segment，检测语言 `en`，时间戳单调；4.51 秒音频耗时 0.721 秒，RTF 0.160。
- 门禁：前端 54 files/204 tests、Rust 222 passed/1 ignored、类型检查、生产构建、`cargo check --tests`、格式与 diff 检查全部通过。
- 分发：最终以 `tauri build --bundles nsis --no-sign` 成功产出 31,294,929 bytes 的 release Windows NSIS；主程序为 Windows GUI subsystem，sidecar、FFmpeg 与 4 个运行时 DLL 均进入构建目录。
- 视觉：1440x900 与 390x844 工作台/任务/设置/模型流程无文档级横向溢出，控制台 0 error；结果导出与重新转写入口已补齐。
- REVIEW：无阻断项；修复模型下载 revision 竞态、转写进度单位与实际进度回调，补齐字幕 adapter、终态通知和自动失败记录。
- 发布交接：正式签名需要发布私钥；macOS/Linux 实机签名/安装/推理与登录态 B 站字幕 smoke 保持外部发布门禁。

## 偏差与阻断

- 暂无。现有设置模块和批准架构兼容，不需要回退到架构设计。

## 2026-10-10 模型管理缺陷修复

- 模型弹窗移除重复的自定义关闭按钮，保留 Naive UI 内置关闭按钮并验证关闭事件。
- 模型下载点击后立即进入 `queued`，下载中/校验中持续显示状态；请求失败写回模型行错误并保留重试入口。
- 模型 HTTP 建连增加 15 秒超时；E012 明确提示检查网络或配置模型镜像地址。
- 弹窗标明 Base 是默认选择、模型按需下载且不随安装包提供，保持 PRD 的模型分发边界。
- 回归：新增模型弹窗与 store 测试；全量前端 55 files/207 tests、Rust 222 passed/1 ignored。
