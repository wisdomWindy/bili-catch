# 任务板：BiliCatch 转写质量重构

状态：`todo` / `doing` / `done` / `blocked`

| ID | 任务 | 阶段 | 状态 | 依赖 | 主要文件 | 验收 |
| --- | --- | --- | --- | --- | --- | --- |
| T01 | 数据模型扩展 | P0 | done | — | `models/transcription.rs`、`src/features/transcription/contracts.ts` | AC-8 |
| T02 | 媒体探测解析器 | P0 | done（硬字幕抽样转入 T11） | T01 | `ffmpeg_preprocessor.rs` | AC-1 前置 |
| T03 | 内嵌字幕抽取 | P0 | done | T02 | 新增 `embedded_subtitle.rs` | AC-1 |
| T04 | 平台字幕语种校验 | P0 | done | — | `platform_subtitle.rs` | AC-2 |
| T05 | VAD 接入与切段 | P1 | done（用 ffmpeg silencedetect 替代 Silero，见 D-9） | T02 | `runtime.rs`、`ffmpeg_preprocessor.rs` | AC-3 前置 |
| T06 | Whisper 参数修复与默认模型 | P1 | done（参数部分；默认模型切换并入 T08） | — | `whisper_sidecar.rs`、`settings/*` | AC-3 |
| T07 | 模型仓库改多文件包 | P2 | done | T01 | `model_repository.rs` | AC-9 |
| T08 | AED 适配器 | P2 | done（默认模型切换延后，见 changelog） | T07 | `fire_red_sidecar.rs` | AC-1/AC-5 |
| T09 | Qwen3-ASR 适配器 | P2 | done（落地 0.6B，1.7B 无逐文件仓库，见 changelog） | T07 | 新增 `qwen3_asr_sidecar.rs` | AC-1 |
| T10 | OCR 运行时 spike | P3 | done（选 `rten`） | — | `artifacts/ocr-spike.md` | spike 结论 |
| T11 | 抽帧与字幕带检测 | P3 | done | T02、T10 | `hard_subtitle_ocr.rs` | AC-4 前置 |
| T12 | OCR 引擎与句子合并 | P3 | done | T11 | `ocr_engine.rs` | AC-4 |
| T13 | 融合引擎 | P4 | done | T03、T04、T12 | 新增 `fusion.rs` | AC-5 |
| T14 | 置信度前端展示 | P4 | done | T13 | `src/features/transcription/*` | AC-7 |
| T15 | 内存守卫与 E018 | P5 | done | T08 | `app_error.rs`、新增 `memory_guard.rs` | AC-6 |
| T16 | OpenASR 8B 适配器 | P5 | done（最小集成：外部工具 + 环境变量指定路径，未随包分发） | T15 | `openasr_sidecar.rs` | AC-6 |
| T17 | 评测脚本与基线 | P6 | done | T06、T08 | `scripts/asr-bench/*` | AC-3/AC-4 |
| T18 | 许可、打包与文档 | P6 | done | T07~T16 | `tauri.conf.json`、`licenses/*` | AC-9 |

## 里程碑

| 里程碑 | 包含 | 对用户可见的变化 |
| --- | --- | --- |
| M1 止血 | T01~T06 | 幻觉消失；带字幕轨的视频秒出稿；默认模型不再是 Base |
| M2 换代 | T07~T09 | 默认引擎换成 FireRedASR2-AED，多语种走 Qwen3 |
| M3 拾字 | T10~T14 | 硬字幕视频走 OCR；冲突段可见；置信度高亮 |
| M4 高精度 | T15~T16 | 可选 8B 档 |
| M5 验收 | T17~T18 | 评测报告与许可合规 |
