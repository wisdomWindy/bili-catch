# BiliCatch 转写质量重构 — 完整方案

- 请求：`bilicatch-asr-quality-overhaul`
- 阶段：architecture-design
- 日期：2026-11-15
- 状态：待审批

---

## 1. 现状定位

现有链路的缺陷不是「模型不够大」，而是「信源单一 + 没有分段 + 参数缺省」。

| 现象 | 代码位置 | 根因 |
| --- | --- | --- |
| 与音频完全无关的成段文字 | `src-tauri/src/infrastructure/transcription/whisper_sidecar.rs:54` | 未启用 `--vad`、`-mc 0`、`-et`、`-lpt`、`-nth`；Whisper 在 BGM/静音上产生幻觉，并沿上文逐段传染 |
| 静音段被解成文字 | `src-tauri/src/infrastructure/transcription/runtime.rs:700` | `silence_points` 恒为空，切分逻辑虽支持静音点但从未使用，等价于无 VAD |
| 错别字、繁体 | `src-tauri/src/infrastructure/settings/store.rs:175` | 默认模型为 `Base`（74M），叠加 `-l auto` 语种误判 |
| 断句碎、时间轴漂移 | `src-tauri/src/infrastructure/transcription/ffmpeg_preprocessor.rs:19`、`fire_red_sidecar.rs:28` | 固定 30 分钟 / 55 秒硬切，切口不落在语音边界 |
| 内容对不上（第二成因） | `src-tauri/src/infrastructure/transcription/platform_subtitle.rs:62`、`src-tauri/src/services/settings/mod.rs:134` | 直接取 `subtitles[0]`，不校验语种，`prefer_platform_subtitles` 默认开启 |
| 有现成文字却仍走 ASR | 全仓库无 OCR、无内嵌字幕轨处理 | 只有「B 站 CC」和「音频 ASR」两条信源，中间缺失 |

结论：**换模型是最后一步，不是第一步。** 先把「白给的字」捡起来，再修复幻觉，最后才轮到 8B。

## 2. 设计原则

1. **信源优先于模型。** 文字已有出处时，识别就是降级方案。
2. **每个信源都要声明自己的能力上限。** 时长上限、是否需登录、是否需视频轨、是否有时戳，由探测结果驱动路由，而不是写死。
3. **不引入 Python、不依赖云端。** 继续保持 sidecar + 按需下载模型的形态，Windows/macOS 同一套二进制。
4. **可解释。** 每一段文本记录来源与置信度，冲突段必须可见，而不是糊成一段「看起来还行」的文本。

## 3. 目标架构

```
创建转写任务
  └─ S0 Probe（探测能力）
       ├─ 时长 / 分辨率 / 音轨编码 / 内嵌字幕轨
       ├─ 是否为本地文件（有无视频轨）
       └─ 底部字幕带是否存在（抽样 30 帧）
  └─ S1 Acquire（按阶梯取信源，命中即停；需融合时并行）
       L1 内嵌字幕轨 ────────────► 直接解析
       L2 B 站 CC / AI 字幕 ─────► 语言校验后解析
       L3 硬字幕 OCR ────────────► 抽帧 → 带检测 → OCR → 去重合并
       L4 音频 ASR ──────────────► VAD 切段 → 引擎识别 → 标点 → ITN
  └─ S2 Normalize（统一为 TranscriptSegment + confidence + origin）
  └─ S3 Fuse（≥2 信源时对齐、比对、标记冲突）
  └─ S4 Emit（txt / srt / vtt / json / md + 可疑片段清单）
```

## 4. 信源阶梯

| 级别 | 信源 | 探测条件 | 成本 | 典型精度 | 失败降级 |
| --- | --- | --- | --- | --- | --- |
| L1 | 内嵌字幕轨 | ffmpeg 探测到 `0:s:*` | 极低（< 2s） | 最高（作者源文件） | 解码失败 → L2 |
| L2 | B 站 CC / AI 字幕 | `DownloadTask` + 已登录 + `cid` 命中 | 低（一次 HTTP） | 高 | 无字幕/语种不符 → L3 |
| L3 | 硬字幕 OCR | 有视频轨且底部文字带稳定 | 中（约 1~2 min / 18 min 视频） | 很高（作者手打） | 覆盖率 < 30% → L4 |
| L4 | 音频 ASR | 始终可用 | 高（0.1~0.8× 实时） | 中高 | 无（兜底） |

**路由规则**

- 只要有 L1，直接使用，不再跑任何模型。
- L2 必须校验语种：请求 `zh` 时只接受 `zh-*` / `ai-zh`；请求 `en` 时只接受 `en` / `ai-en`。不符则跳过并记录原因，避免「翻译字幕冒充转写」。
- L3 与 L4 在「需要最高质量」模式下**并行执行并融合**；在「快速」模式下只跑 L4。
- 纯音频任务（`DownloadMode::AudioOnly` 或音频本地文件）没有画面，跳过 L1/L3。

## 5. 各级实现要点

### 5.1 S0 探测

扩展现有 `FfmpegTranscriptionPreprocessor::inspect`（`ffmpeg_preprocessor.rs:84`），解析 `ffmpeg -hide_banner -i` 的 stderr：

- `Duration:` → 已实现
- `Stream #0:0 ... Video: ... 852x480` → 分辨率、是否存在视频轨
- `Stream #0:1 ... Audio: aac, 44100 Hz, stereo` → 编码、采样率、声道
- `Stream #0:2 ... Subtitle: ass / mov_text` → 内嵌字幕轨列表（索引、编码、`language`、`title`）

底部文字带检测：抽 30 帧（均匀分布），灰度化后对下 35% 区域做水平投影，统计「连续高对比行」出现比例。比例 ≥ 60% 判定为「有稳定硬字幕」，否则不进入 L3。

### 5.2 L1 内嵌字幕

`ffmpeg -nostdin -y -i <src> -map 0:s:<n> -f srt -`，解析 SRT 为分段。注意：

- `mov_text` 与 `ass` 均需转 `srt`；`ass` 会带样式标签，需剥离 `{...}`。
- 若字幕轨语种与请求语言不符，跳过。

### 5.3 L2 平台字幕

复用 `platform_subtitle.rs`，增加：

- 候选轨按语种匹配，而不是 `subtitles[0]`。
- 记录 `lan` 与 `ai_type`，AI 字幕在结果中标注来源为 `platform-ai`。

### 5.4 L3 硬字幕 OCR

管线：**抽帧 → 廉价变化门控 → 字幕带裁剪 → OCR → 去重 → 合并成句**。

1. 抽帧：默认 2 fps，仅当「字幕带像素差」超过阈值时才认为文本变化。变化门控约 2ms/帧，可把 OCR 调用量从 3600 次/小时降到 800~1200 次/小时。
2. 字幕带：用 S0 的稳定 ROI，避免硬裁底部 20% 切到花字。
3. OCR：PP-OCRv5 `det` + `rec` 的 ONNX 模型，中文 `rec`。优先用 `ort` crate 复用已打包的 onnxruntime；若链接冲突则改用纯 Rust 的 `rten`。**此项是 P3 开始前的 1 天 spike，先定运行时再写业务。**
4. 去重：相邻帧文本归一化编辑距离 < 0.15 视为同一句，合并区间；文本跳变处切句。
5. 时间轴：首次出现帧为 `start`，末次出现帧 + 帧间隔为 `end`。
6. 质量：取该句所有帧的 `rec` 置信度均值；< 0.6 的句子标记低置信度，并在融合阶段让位给 ASR。

### 5.5 L4 音频 ASR

1. **VAD 切段**：Silero VAD（约 2 MB），段长上限按引擎设定（见 §6），最短语音 0.3s，最短静音 0.4s。
2. **引擎**：默认 FireRedASR2-AED int8；多语种切 Qwen3-ASR-1.7B；可选 8B。
3. **后处理**：标点恢复（引擎自带则跳过）、ITN（数字/单位/日期）。
4. **复用现有抽象**：仍然实现 `Transcriber` trait，仍然接受 `ProcessControl` 取消，仍然抛 `AppError`。

## 6. 引擎与模型清单

| 用途 | 模型 | 体积 | 峰值内存 | 单段上限 | 时戳 | 许可 | 运行时 | 交付档 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| VAD | Silero VAD | 2 MB | < 100 MB | — | — | MIT | sherpa-onnx | 随包 |
| ASR 默认 | FireRedASR2-AED int8 | 1.18 GB | ~2 GB | 55s | 字级 + 置信度 | Apache-2.0 | sherpa-onnx | 按需下载 |
| ASR 多语种 | Qwen3-ASR-1.7B int8 | ~2.4 GB | ~3.5 GB | 20s | 无（用 VAD 段） | Apache-2.0 | sherpa-onnx | 按需下载 |
| ASR 高精度 | FireRedASR2-LLM q4_k | 4.7 GB | 8.7 GB | 40s | 无（用 VAD 段） | Apache-2.0 | OpenASR CLI（仅 CLI 契约） | 可选下载 |
| 标点 | FireRedPunc | ~100 MB | ~300 MB | — | — | Apache-2.0 | sherpa-onnx / ONNX | 随包或按需 |
| OCR | PP-OCRv5 det + rec | ~20 MB | ~300 MB | — | — | 需逐个核对（PaddleOCR 为 Apache-2.0） | `ort` / `rten` | 随包 |

**为什么不是直接上 8B**

- 8B 的输入上限只有 40s，必须依赖 VAD，否则会截断；它本身不提供字级时戳。
- 8.7 GB 峰值内存会让 8 GB 机器直接 OOM，必须做前置检查。
- 0.77× 实时（M1 CPU）意味着 1 小时视频要 46 分钟，只能作为「高精度」可选档。
- 它只服务「既无字幕轨、又无硬字幕、又无 CC」的视频，而这部分在 B 站占比并不高。

因此 8B 放在 P5，且排在 OCR 之后。

## 7. 数据模型变更

### 7.1 `MediaSourceInfo`（`src-tauri/src/models/transcription.rs:101`）

新增字段，全部带 `#[serde(default)]` 以保证旧缓存可读：

```rust
pub struct MediaSourceInfo {
    // 现有字段不变
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub audio_codec: Option<String>,
    pub audio_channels: Option<u16>,
    pub has_video_track: bool,
    pub embedded_subtitles: Vec<EmbeddedSubtitleTrack>,
    pub hard_subtitle_detected: bool,
}

pub struct EmbeddedSubtitleTrack {
    pub index: u32,
    pub codec: String,
    pub language: Option<String>,
    pub title: Option<String>,
}
```

### 7.2 `TranscriptSegment`（同文件 `:175`）

```rust
pub struct TranscriptSegment {
    pub index: u32,
    pub start_ms: u64,
    pub end_ms: u64,
    pub text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confidence: Option<f32>,
    #[serde(default)]
    pub origin: TranscriptOrigin,        // 默认 Asr，兼容旧数据
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub alternates: Vec<SegmentAlternate>, // 融合冲突时的备选文本
}

#[derive(Default)]
pub enum TranscriptOrigin {
    EmbeddedSubtitle,
    PlatformSubtitle,
    #[default]
    Asr,
    HardSubtitleOcr,
    Fused,
}
```

### 7.3 `TranscriptResult`

新增 `pipeline: PipelineReport`，记录：实际使用的信源、各级耗时、冲突段数、低置信度段数、引擎与模型版本。用于前端展示和问题定位，不参与导出格式。

### 7.4 模型清单（`infrastructure/transcription/model_repository.rs:19`）

从「单文件」升级为「模型包」：

```rust
pub struct ModelPackage {
    pub id: ModelId,
    pub kind: ModelKind,          // Asr | Vad | Punc | Ocr
    pub display_name: String,
    pub runtime: ModelRuntime,     // Whisper | Sherpa | OpenAsr | Onnx
    pub files: Vec<ModelFile>,     // name / size / sha256 / role
    pub capabilities: Vec<String>,
    pub recommended_for: String,
}
```

影响：`download_url` 的自定义镜像拼接方式要改成「目录 + 相对文件名」；`.part` 命名改成 `<model-id>/<file>.part`；`delete` 改为删除整个模型目录；旧的单文件模型需要一次兼容读取或清理。

## 8. 运行时编排

`services/transcription/runtime.rs::run_pipeline` 改为阶段式：

```rust
let caps = probe(&source).await?;
let plan = route(&caps, &options);          // 决定跑哪几级信源

let mut candidates: Vec<Candidate> = Vec::new();
if let Some(sub) = try_embedded(&caps, &plan).await? { candidates.push(sub); }
if let Some(sub) = try_platform(&caps, &plan).await? { candidates.push(sub); }

let need_asr = plan.requires_asr(candidates.len());
let need_ocr = plan.requires_ocr(&caps);

let (ocr, asr) = tokio::join!(
    maybe_ocr(&caps, need_ocr, &control, &progress),
    maybe_asr(&plan, need_asr, &control, &progress),
);
candidates.extend(ocr);
candidates.extend(asr);

let fused = if candidates.len() > 1 { fuse(candidates) } else { candidates.remove(0) };
emit(fused)
```

约束：

- 任一级失败必须降级到下一级，不得让整任务失败（L1/L2/L3 的失败都是可降级的）。
- `ProcessControl` 需要同时传给 OCR 与 ASR 两个并发分支，取消时两者都要停。
- 进度需要重新分配权重：探测 2%、OCR 30%、ASR 68%。

## 9. 融合算法

仅当候选 ≥ 2 时执行。

1. 按时间轴做扫描线切分，得到若干「重叠簇」与「独有段」。
2. 独有段直接保留，来源标记为对应信源。
3. 重叠簇内：
   - 文本归一化（去标点、去空格、全半角统一、繁转简）后计算 `sim = 1 - lev(a,b) / max(len)`。
   - `sim ≥ 0.85`：判定一致，取 `confidence` 更高者，`origin = Fused`。
   - `sim < 0.85`：判定冲突。**优先取 OCR/字幕**（作者手打，错误类型是「漏字」而非「同音错字」），把 ASR 文本放入 `alternates`，并把该段 `confidence` 压到 0.4 以下。
4. 冲突段不超过总段数 15% 时只标黄；超过则整体标记「建议人工校对」。

## 10. 质量分

```
q = 0.40 * confidence
  + 0.20 * has_punctuation
  + 0.20 * cadence_score        // 字/秒 接近 3.5~6.0 时接近 1
  + 0.20 * hotword_hit_ratio
```

- `q < 0.55`：前端标红，导出时可选择性加 `[?]` 前缀。
- 质量分只用于展示与排序，不自动删改任何文本。

## 11. 资源与性能估算（1 小时视频，量级估算，以 P0 实测为准）

| 路径 | CPU 耗时 | 峰值内存 |
| --- | --- | --- |
| L1 内嵌字幕 | < 2s | 忽略 |
| L2 平台字幕 | 1~3s | 忽略 |
| L3 硬字幕 OCR（2fps + 变化门控） | 1~2 min | ~300 MB |
| L4 AED（4 线程） | 8~18 min | ~2 GB |
| L4 Qwen3-1.7B | 6~12 min | ~3.5 GB |
| L4 FireRedASR2-LLM 8B（M1 CPU / GPU） | 46 min / 24 min | 8.7 GB |

## 12. 分阶段实施

| 阶段 | 内容 | 改动文件 | 工时 | 验收 |
| --- | --- | --- | --- | --- |
| P0 | 探测扩展 + 内嵌字幕轨抽取 + 平台字幕语种校验 | `ffmpeg_preprocessor.rs`、新增 `embedded_subtitle.rs`、`platform_subtitle.rs` | 0.5d | 带字幕轨的样本 100% 走 L1；语种不符的样本不再误用 L2 |
| P1 | 接入 Silero VAD；修复 Whisper 参数；默认模型从 Base 换掉；中文禁用 `-l auto` | `runtime.rs`、`whisper_sidecar.rs`、`store.rs` | 1d | 评测集幻觉段降为 0 |
| P2 | 模型仓库改多文件包；AED 与 Qwen3-1.7B 双档接入 | `model_repository.rs`、`fire_red_sidecar.rs`、`models/transcription.rs` | 2d | 两档均可下载、校验、删除、转写 |
| P3 | 硬字幕 OCR（含 1 天运行时 spike） | 新增 `hard_subtitle_ocr.rs`、`ocr_engine.rs`、`ffmpeg_preprocessor.rs` | 3~5d | 硬字幕样本覆盖 ≥ 90% |
| P4 | 融合 + 置信度 + 前端展示 | 新增 `services/transcription/fusion.rs`、`runtime.rs`、`src/features/transcription/*` | 1.5d | 冲突段可见；导出不受影响 |
| P5 | 8B 高精度档（OpenASR CLI 契约） | 新增 `openasr_sidecar.rs`、`model_repository.rs`、前端模型页 | 2d | 内存不足返回结构化错误 |
| P6 | 评测集与回归脚本 | 新增 `scripts/asr-bench/` | 1d | 指标可重复产出 |

## 13. 评测方案

- **素材**：10 条 B 站视频，口播 3、访谈 2、方言 2、带 BGM 2、游戏实况 1；总时长 60~90 分钟。
- **标注**：只标关键实体（人名、数字、术语、英文缩写）+ 句子边界，不逐字标全稿，控制标注成本。
- **指标**：
  - `EntityErrorRate`：关键实体错误数 / 实体总数
  - `HallucinationSegments`：与音频无关的段数（人工判定）
  - `TimelineMAE`：段落起止与标注的平均绝对误差
  - `Coverage`：OCR 路径覆盖的画面句占比
- **脚本**：`scripts/asr-bench/run.mjs`，读取 `cases.json`，逐条调用 CLI 或 Tauri 命令，输出 Markdown 对比表与 JSON。

**验收线**：幻觉 0；`EntityErrorRate` 相对基线下降 ≥ 50%；`TimelineMAE` ≤ 1s。

## 14. 风险与对策

| 风险 | 影响 | 对策 |
| --- | --- | --- |
| OCR 运行时选型不确定（`ort` 与 sherpa 的 onnxruntime 冲突） | P3 阻塞 | P3 首日 spike，二选一后锁定；备选 `rten` |
| 花字/竖屏/动态字幕导致 ROI 失效 | OCR 覆盖不足 | ROI 检测 + `rec` 置信度阈值；覆盖率 < 30% 自动降级到 ASR |
| 8B 内存不足 OOM | 崩溃 | 启动前检查可用内存，新增错误码 `E018 RESOURCE_INSUFFICIENT` |
| 模型包体积膨胀 | 安装包/磁盘 | 除 VAD 与 OCR 外全部按需下载 + sha256 校验 |
| 平台字幕语种/版权不确定 | 结果不可用 | 语种白名单校验；来源标注为 `platform-ai` |
| Windows 非 ASCII 路径 | sidecar 读不到模型 | 复用 `fire_red_sidecar.rs` 已有的 ASCII staging 机制，OCR 与 OpenASR 同样处理 |
| OpenASR 单维护者、接口可能变 | 集成脆弱 | 只依赖 `transcribe` CLI 契约，不依赖 `serve`；封装在单一适配器内 |
| 旧缓存缺少新字段 | 反序列化失败 | 全部新字段 `#[serde(default)]`；缓存 fingerprint 加入管线版本号 |

## 15. 待决问题

1. **8B 档是否随包分发？** 建议：否，首次使用时按需下载，并在模型页明确标注「需 16 GB 内存」。
2. **OCR 是否默认开启？** 建议：检测到硬字幕即默认开启（约 +1~2 分钟/小时），可在设置中关闭。
3. **Whisper 引擎是否保留？** 建议：保留为「冷门语种兜底」，但不再是默认，也不再用 `Base`/`Tiny` 作为默认模型。
4. **平台字幕与 OCR 冲突时以谁为准？** 建议：OCR（可复现、无版权与登录依赖），平台字幕仅在其独占时段使用。
5. **是否接受新增一个外部二进制（OpenASR）？** 建议：接受，但限定为可选档，且只依赖 CLI。
