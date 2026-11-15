# 规格：BiliCatch 转写质量重构

- 请求：`bilicatch-asr-quality-overhaul`
- 上游设计：`design/solution.md`
- 阶段：spec
- 日期：2026-11-15

## 1. 范围

把转写链路从「Whisper 单引擎直出」改为「信源阶梯 + VAD + 多引擎 + 融合」。本规格覆盖 P0~P6 全部阶段的行为契约，实施按 `plan/task-board.md` 顺序推进。

不在范围：云端 ASR、说话人分离、实时流式、Linux 原生支持、歌词抓取。

## 2. 术语

| 术语 | 定义 |
| --- | --- |
| 信源 | 产生转写文本的来源，共四级：内嵌字幕、平台字幕、硬字幕 OCR、音频 ASR |
| 探测 | 用 ffmpeg 读取容器与流信息，判断可用信源 |
| 候选 | 单个信源产出的 `Candidate { origin, segments }` |
| 融合 | 对齐 ≥2 个候选并按一致性规则合成最终结果 |
| 质量分 | 段级的 0~1 数值，用于展示与排序，不用于自动改写文本 |

## 3. 功能需求

### FR-1 媒体探测

`inspect_transcription_source` 必须返回：

| 字段 | 类型 | 语义 |
| --- | --- | --- |
| `width` / `height` | `Option<u32>` | 视频轨分辨率；纯音频为 `None` |
| `audioCodec` | `Option<String>` | 首个音轨编码名，如 `aac` |
| `audioChannels` | `Option<u16>` | 声道数 |
| `hasVideoTrack` | `bool` | 是否存在视频流 |
| `embeddedSubtitles` | `EmbeddedSubtitleTrack[]` | 内嵌字幕轨列表 |
| `hardSubtitleDetected` | `bool` | 底部文字带抽样判定结果 |

`EmbeddedSubtitleTrack { index: u32, codec: String, language?: string, title?: string }`。

判定规则：

- 解析 `ffmpeg -hide_banner -i <src>` 的 stderr；每条 `Stream #0:N` 按 `Video:` / `Audio:` / `Subtitle:` 归类。
- 字幕轨的 `language` 取自流元数据 `(language: xxx)` 或 `(title: xxx)`，缺失为 `None`。
- `hardSubtitleDetected` 仅在 `hasVideoTrack` 为真时计算：均匀抽 30 帧，下 35% 区域灰度水平投影，若「连续高对比行」在 ≥60% 的帧中出现则置真。
- 抽样失败（解码错误）时 `hardSubtitleDetected=false`，不阻塞任务。

验收：

- 带 `mov_text` 字幕轨的 mp4，`embeddedSubtitles` 长度 ≥1 且 `codec="mov_text"`。
- 纯音频文件，`hasVideoTrack=false` 且 `embeddedSubtitles` 为空。
- stderr 缺少 `Duration:` 仍按现有 `SOURCE_DURATION_UNAVAILABLE` 失败。

### FR-2 L1 内嵌字幕

条件：`embeddedSubtitles` 非空且存在与请求语言兼容的轨。

行为：

- 优先选择语种匹配的轨；无匹配时选第一条。
- `ffmpeg -nostdin -y -i <src> -map 0:s:<n> -f srt -`，从 stdout 读取。
- 解析 SRT 为 `TranscriptSegment`，`origin=EmbeddedSubtitle`，`confidence=1.0`。
- ASS 样式标签 `{\...}` 必须剥离。

失败语义：解码失败或产出 0 段 → 记录 `embedded-subtitle-failed`，降级到下一级，任务不失败。

### FR-3 L2 平台字幕

条件：`TranscriptionSource::DownloadTask` 且 `bvid`、`cid` 均存在、`prefer_platform_subtitles=true`、已登录。

行为：

- 从 `subtitles[]` 中筛选语种兼容的候选，**不得**默认取第一条。
- 语种兼容映射：`zh` ← `zh-CN|zh-Hans|ai-zh`；`zh-TW` ← `zh-TW|zh-Hant|ai-zh`；`en` ← `en|ai-en`；`ja` ← `ja`；`ko` ← `ko`。
- 无兼容候选 → 记录 `platform-subtitle-language-mismatch`，降级。
- `origin=PlatformSubtitle`，`confidence=0.9`；若 `lan` 以 `ai-` 开头则 `confidence=0.75`。

### FR-4 L3 硬字幕 OCR

条件：`hasVideoTrack=true` 且 `hardSubtitleDetected=true` 且设置项 `enable_hard_subtitle_ocr=true`（默认 true）。

行为：

- 抽帧 2 fps，逐帧比较字幕带像素差，超过阈值才送 OCR。
- 只对字幕带 ROI 做 `det` + `rec`。
- 相邻帧归一化编辑距离 < 0.15 合并为同句；文本跳变处切句。
- 句子 `start_ms` 取首帧时间，`end_ms` 取末帧时间 + 帧间隔。
- 句子 `confidence` 取该句所有帧的 `rec` 置信度均值。
- `origin=HardSubtitleOcr`。

降级：产出句数 / 探测到的画面句数 < 30%，或平均置信度 < 0.5 → 丢弃 OCR 候选并记录 `ocr-low-coverage`。

### FR-5 L4 音频 ASR

条件：所有无 L1/L2 命中、或用户选择「快速」模式、或需要融合时。

行为：

- VAD 使用 Silero，段长上限：AED 55s、Qwen3 20s、8B 40s；最短语音 0.3s，最短静音 0.4s。
- 引擎路由：`language ∈ {ja, ko}` 或 `asr_engine=multilingual` → Qwen3-ASR-1.7B；`asr_engine=high-accuracy` → FireRedASR2-LLM；其余 → FireRedASR2-AED。
- 8B 档执行前检查可用内存，`可用内存 < 9.5 GB` 时返回 `E018 RESOURCE_INSUFFICIENT`，`details="ASR_HIGH_ACCURACY_MEMORY"`。
- `origin=Asr`，`confidence` 取引擎返回值；引擎不返回置信度时为 `None`。

### FR-6 融合

条件：候选数 ≥2。

行为：

- 时间轴扫描线切分重叠簇与独有段。
- 相似度 `sim = 1 - lev(a,b)/max(len(a),len(b))`，文本先做标点剥离、空白归一、全角转半角。
- `sim ≥ 0.85`：取置信度高者，`origin=Fused`，`confidence=max(a,b)`。
- `sim < 0.85`：取 OCR/字幕文本（`origin=Fused`，`confidence=0.4`），ASR 文本写入 `alternates`。
- 冲突段占比 > 15% → `PipelineReport.reviewRecommended=true`。

### FR-7 置信度展示

- 前端对 `confidence < 0.55` 的段标黄；`< 0.35` 标红并提示「建议人工校对」。
- 导出行为不变；提供可选「导出时对低置信度段加 `[?]` 前缀」设置，默认关闭。

### FR-8 模型包管理

- 模型清单条目改为模型包：`{ id, kind, runtime, files[], capabilities, recommendedFor, sizeBytes }`。
- 下载、校验、删除以「包」为单位；`.part` 路径为 `<model-id>/<file>.part`。
- 旧单文件模型目录需可读：若检测到旧文件名，按兼容条目处理并提示重新下载。

### FR-9 资源守卫

- 新增错误码 `E018 RESOURCE_INSUFFICIENT`，`details` 取值：`ASR_HIGH_ACCURACY_MEMORY`、`OCR_MODEL_UNAVAILABLE`。
- 触发时任务进入 `Failed`，并保留可重试性。

## 4. 入站接口

| 命令 | 变更 |
| --- | --- |
| `inspect_transcription_source` | 返回值扩展 FR-1 字段（新增字段均为可选，向后兼容） |
| `create_transcribe_task` | `options` 新增可选 `asrEngine`、`enableHardSubtitleOcr`、`preferEmbeddedSubtitle` |
| `list_transcription_models` | 模型条目扩展 `kind`、`runtime`、`files` |
| 其余命令 | 签名不变 |

前端契约 `src/features/transcription/contracts.ts` 同步扩展；新增字段全部可选，旧状态文件可读。

## 5. 出站依赖

| 依赖 | 用途 | 失败语义 |
| --- | --- | --- |
| ffmpeg sidecar | 探测、抽帧、抽字幕、转码 | 失败即任务失败（`E013`） |
| sherpa-onnx sidecar | VAD、AED、Qwen3 | 失败降级到 Whisper（若可用），否则失败 |
| OpenASR CLI | 8B 档 | 失败返回 `E014 SIDECAR_FAILED` |
| OCR 运行时（`ort`/`rten`） | 硬字幕识别 | 模型缺失 `E018 OCR_MODEL_UNAVAILABLE`，降级 ASR |
| B站字幕 API | 平台字幕 | 失败降级，不失败任务 |

## 6. 事件契约

沿用现有 `TranscriptionProgressEvent` / `TranscriptionRemovedEvent`，不新增事件类型。

`TranscriptionProgress.message` 新增取值：`Probing`、`ExtractingSubtitles`、`OcrRunning`、`Transcribing`、`Fusing`、`Saving`。

进度权重：探测 0.02、字幕 0.03、OCR 0.30、ASR 0.60、融合与保存 0.05。

## 7. 数据模型

见 `design/solution.md` §7。补充约束：

- 所有新增字段必须 `#[serde(default)]`，保证旧缓存与旧前端可反序列化。
- `TranscriptOrigin` 默认 `Asr`。
- `PipelineReport { sourcesUsed: origin[], ocrMs, asrMs, conflictCount, lowConfidenceCount, engine, modelVersion, reviewRecommended }`。
- 缓存 fingerprint 增加：管线版本号 `PIPELINE_VERSION="2"`、信源类型、引擎 id。

## 8. 流程与状态机

沿用 `TranscribeTaskStatus`。新增约束：

- `Preprocessing` 覆盖探测与字幕抽取。
- OCR 与 ASR 并行时，任一分支失败不改变任务状态，只在 `PipelineReport` 记录。
- 取消：`ProcessControl` 必须同时下发到 OCR 与 ASR 两个分支；两者都结束前不得进入 `Saving`。

## 9. 异常语义

| 情况 | 行为 | 错误码 |
| --- | --- | --- |
| 探测失败 | 任务失败 | `E013 SOURCE_DECODE_FAILED` |
| 无可识别语音 | 任务失败 | `E015 NO_SPEECH_DETECTED` |
| 内存不足（8B） | 任务失败，可重试 | `E018` |
| OCR 模型缺失 | 降级 ASR | 记录 `ocr-model-missing` |
| 平台字幕语种不符 | 降级 | 记录 `platform-subtitle-language-mismatch` |
| 融合后无段 | 任务失败 | `E015` |

## 10. 非功能

- **性能**：1 小时视频，L1 < 2s、L3 1~2min、AED 8~18min、8B 24~46min；目标值以 P6 实测记录。
- **内存**：非 8B 路径峰值 ≤ 3.5 GB；8B 路径 ≤ 9.5 GB。
- **幂等**：同一 fingerprint 的重复任务必须复用缓存；`force=true` 跳过。
- **并发**：OCR 与 ASR 并行，合计占用 1 个任务并发额度（不改变 `max_concurrent_transcriptions` 语义）。
- **超时**：ffmpeg 与 sidecar 不设硬超时，依赖用户取消；OCR 单帧超时 3s，超时帧跳过。
- **观测**：`PipelineReport` 写入结果 JSON；日志记录每级命中的信源与降级原因。

## 11. 安全与许可

- 不引入网络请求（平台字幕除外，且域名白名单已存在）。
- 模型下载仅允许 HTTPS，沿用 `model_repository.rs` 的校验与重定向策略。
- 新增依赖许可：Silero VAD（MIT）、FireRedASR2（Apache-2.0）、Qwen3-ASR（Apache-2.0）、PP-OCRv5（PaddleOCR Apache-2.0，需在实施时复核模型文件许可）、OpenASR（Apache-2.0 CLI）。
- 打包清单与 NOTICE 必须同步更新。

## 12. 兼容与迁移

- 旧 `MediaSourceInfo` / `TranscriptSegment` JSON 可读（serde default）。
- 旧模型目录：单文件模型仍可列出；新模型包使用子目录。两者互不覆盖。
- 旧缓存结果缺少 `origin` 时按 `Asr` 处理。
- 回滚：把 `PIPELINE_VERSION` 回退为 `"1"` 并保留旧 Whisper 适配器即可恢复原链路；新增字段为可选，无需回滚数据结构。

## 13. 验收标准

| 编号 | 验收项 | 验证方式 |
| --- | --- | --- |
| AC-1 | 带内嵌字幕轨的样本走 L1，耗时 < 2s | 集成测试 + 样本 |
| AC-2 | 请求 `zh` 时不使用 `en` 字幕轨 | 单元测试 |
| AC-3 | 评测集幻觉段 = 0 | P6 评测 |
| AC-4 | 关键实体错误数相对基线 -50% | P6 评测 |
| AC-5 | 所有导出格式时间轴偏差 ≤ 1s | 单元测试 + 样本 |
| AC-6 | 8B 内存不足返回 `E018` | 单元测试（注入内存探测） |
| AC-7 | 取消在 OCR/ASR 并行时生效 | 集成测试 |
| AC-8 | 旧缓存与旧前端契约可读 | 反序列化测试 |
| AC-9 | 模型包下载/校验/删除可用 | 单元测试（本地 HTTP fixture） |

## 14. 风险

见 `design/solution.md` §14。spec 阶段新增风险：OCR 模型许可需逐文件复核，若 PP-OCRv5 模型许可不满足，改用 Apache-2.0 的 `PP-OCRv4` 或自训练。
