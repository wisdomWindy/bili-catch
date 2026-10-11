# `audio-transcription` 验证报告

## 结论

- 判定：`pass`
- 验证日期：2026-10-10
- 范围：模块八音频转文本的合同、模型、预处理、识别、结果、设置、任务集成、通知、界面与 Windows 分发。
- 发布交接：Windows 功能与 unsigned NSIS smoke 通过；正式 updater/code signing 仍需发布私钥；macOS/Linux 真实签名、安装和运行必须在对应主机执行。

## 自动化门禁

| 门禁 | 结果 | 证据 |
| --- | --- | --- |
| 前端测试 | pass | `npm test -- --run`：54 files、204 tests passed |
| 前端类型与生产构建 | pass | `npm run build`：`vue-tsc --noEmit` 与 Vite build 成功；仅有既存 chunk/dynamic-import 警告 |
| Rust 测试 | pass | `cargo test --manifest-path src-tauri/Cargo.toml -- --test-threads=1`：222 passed、1 ignored |
| Rust 编译检查 | pass | `cargo check --manifest-path src-tauri/Cargo.toml --tests` 成功；仅有既存 auth dead-code 警告 |
| 格式与补丁完整性 | pass | `cargo fmt -- --check`、`git diff --check` 均成功 |
| Windows 打包 | pass | `npm run tauri -- build --bundles nsis --no-sign` 成功生成 release NSIS；主程序 PE Subsystem 为 `Windows GUI` |

## 真实识别 smoke

- 环境：Windows，Intel Core Ultra 7 265K，20 核/20 逻辑处理器。
- 引擎：官方 `whisper.cpp v1.8.6` Windows CPU build；`whisper-cli` SHA-256 `111bd344b7bf0356818f2795a525cb5240ed0a99028fa9f3f1c68b4ff5b17b91`。
- 模型：官方 Tiny，77,691,713 bytes；SHA-256 `be07e048e1e599ad46341c8d2a135645097a538221678b7acdd1b1919c6e1b21`。
- 输入：SAPI 生成英文 WAV，经捆绑 FFmpeg 规范化为 16 kHz、mono、PCM s16，时长约 4.51 秒。
- 输出：语言 `en`，1 个非空 segment，文本 `Billy catch converts local audio into searchable text.`，时间戳单调。
- 性能：CPU 推理 0.721 秒，realtime factor `0.160`；达到实时处理目标。
- 隐私：推理进程仅消费本地 WAV 与本地模型；代码扫描未发现媒体上传路径。网络仅用于模型下载，以及下载任务启用“优先平台字幕”时的 B 站字幕元数据读取。

## 分发证据

- 安装包：`src-tauri/target/release/bundle/nsis/BiliCatch_0.1.18_x64-setup.exe`
- 大小：31,300,448 bytes
- SHA-256：`76abf67da8c756c468f4058d9b1d106ab0e8256819916fd2beb2193fdadd8ac7`
- 主程序：release 优化构建，PE Subsystem `2`（Windows GUI），启动不创建调试控制台。
- 构建输出包含 `whisper-cli.exe`、`whisper.dll`、`ggml.dll`、`ggml-base.dll`、`ggml-cpu.dll`。
- 仓库包含 `whisper.cpp v1.8.6` MIT 许可和第三方来源说明；未提交 ASR 模型或 smoke 输出。
- locator 测试覆盖 Windows x64、macOS x64/arm64、Linux x64/arm64 的冻结目标名。

## 界面验证

- 桌面 1440x900 与手机 390x844 均完成工作台、模型弹窗、联合任务、设置页检查。
- 手机端结果操作区 6 个控件自动换行，document width 等于 viewport width，无横向溢出。
- demo 主流程完成来源检查、开始转写、结果编辑展示；结果区包含复制、打开、重新转写、格式选择、导出和保存。
- 浏览器控制台错误数为 0；预览保留在 `http://127.0.0.1:1420/#/transcribe?demo=1`。

## 验收矩阵

| 标准 | 判定 | 主要证据 |
| --- | --- | --- |
| AC-01 路由与页面 | pass | 路由/导航测试；桌面与手机视觉检查；query handoff 与重新转写入口 |
| AC-02 来源 | pass | 来源合同、格式矩阵、FFmpeg probe/预处理与下载任务 handoff 测试 |
| AC-03 模型 | pass | 官方 manifest、流式下载、SHA-256、`.part` 清理、原子发布、损坏/删除/使用中保护测试；Tiny 真实模型 smoke |
| AC-04 真实转写 | pass | v1.8.6 + 官方 Tiny 的 Windows CPU 真实推理；无可见控制台；媒体无上传路径 |
| AC-05 状态、取消与恢复 | pass | 状态矩阵、幂等取消、重试、revision/sequence、恢复和 workspace drop 清理测试 |
| AC-06 结果与导出 | pass | 五格式 golden 行为、强制时间戳、冲突命名、revision 编辑、缓存与 force 测试；UI 导出/重转写入口 |
| AC-07 设置与联动 | pass | v1->v2、11 字段前后端测试；下载模式意图与幂等 completion hook；自动失败独立记录且不回写下载状态 |
| AC-08 任务与通知 | pass | 联合投影、筛选、动作与下载回归；完成/失败通知分支，取消不通知 |
| AC-09 工程门禁 | pass | 全量门禁、IPC/manifest/sidecar/export/fallback 测试、固定二进制与许可证、未提交模型 |
| AC-10 性能与平台交接 | pass | RTF 0.160；Windows NSIS smoke；五目标 locator 测试；跨平台实机发布门禁已明确 |

## 残余风险与发布门禁

- 正式构建启用了 updater artifacts，发布时必须提供 `TAURI_SIGNING_PRIVATE_KEY`；本次用 `--no-sign` 验证功能包，不替代正式签名。
- macOS/Linux 未提供本轮实机与签名环境；需要补齐各自 sidecar 二进制、签名、安装和真实推理 smoke 后方可发布对应平台。
- B 站登录态字幕读取已覆盖 JSON 解析、HTTPS host allowlist 与本地 ASR fallback；真实账号/线上 API smoke 留给发布环境，失败不会阻断本地识别。
- 并发全量 Rust 复跑曾有一次既有 auth 取消时序用例 flake；该用例单独通过，且认证代码无本次 diff，串行全量 222 项通过。后续可单独收紧该测试的调度同步。
