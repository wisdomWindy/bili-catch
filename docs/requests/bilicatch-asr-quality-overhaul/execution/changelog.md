# 执行记录

- 请求：`bilicatch-asr-quality-overhaul`
- 阶段：execute
- 更新：2026-11-15

## T01 数据模型扩展（done）

改动：

- `src-tauri/src/models/transcription.rs`
  - `MediaSourceInfo` 新增 `width`、`height`、`audio_codec`、`audio_channels`、`has_video_track`、`embedded_subtitles`、`hard_subtitle_detected`，全部 `#[serde(default)]`。
  - 新增 `EmbeddedSubtitleTrack`、`TranscriptOrigin`（默认 `Asr`）、`SegmentAlternate`、`PipelineReport`。
  - `TranscriptSegment` 新增 `confidence`、`origin`、`alternates`；`origin` 为 `Asr` 时不参与序列化，保证旧 JSON 导出不变。
  - `TranscriptResult` 新增可选 `pipeline`。
  - `MediaSourceInfo` 派生 `Eq` 保留；`TranscriptSegment`/`TranscriptResult` 因含 `f32` 改为仅 `PartialEq`。
- `src-tauri/src/models/mod.rs` 导出新类型。
- `src-tauri/src/services/transcription/ports.rs`：`TranscriberOutput`、`SubtitleLookup` 去掉 `Eq` 派生（传递依赖）。
- `src-tauri/src/services/transcription/processing.rs`：`merge_segments` 透传 `confidence`/`origin`/`alternates`，不再丢弃来源信息。
- `src-tauri/src/infrastructure/transcription/platform_subtitle.rs`：段落标记为 `PlatformSubtitle`。
- 测试与构造点补齐：`transcription_manager.rs`、`transcription_result_pipeline.rs`、`runtime.rs`。
- 前端契约 `src/features/transcription/contracts.ts` 同步扩展，新增字段全部可选。

新增测试：

- `legacy_segment_json_defaults_to_asr_origin_and_no_confidence`
- `asr_origin_is_omitted_from_serialized_output`
- `non_asr_origin_round_trips_through_kebab_case`

## T02 媒体探测解析器（done，硬字幕抽样除外）

改动：

- `src-tauri/src/infrastructure/transcription/ffmpeg_preprocessor.rs`
  - 新增 `StreamProbe` 与 `parse_stream_probe`，解析 `Stream #0:N` 行：分辨率、音轨编码、声道、内嵌字幕轨（索引/编码/语种）。
  - 封面图（`attached pic`）不计为视频轨，避免纯音频文件被误路由到 OCR。
  - `inspect` 返回值接入探测结果。

新增测试：

- `stream_probe_extracts_resolution_audio_and_subtitle_tracks`
- `stream_probe_ignores_cover_art_and_reads_channel_layouts`

已知偏差：

- `hard_subtitle_detected` 目前恒为 `false`。抽样判定依赖逐帧解码能力，统一在 T11（抽帧与字幕带检测）实现，避免现在写两套抽帧逻辑。此偏差已在 `inspect` 源码处注释标注。

## T04 平台字幕语种校验（done）

改动：

- `src-tauri/src/services/transcription/ports.rs`：`SubtitleProvider::lookup` 增加 `language` 参数。
- `src-tauri/src/infrastructure/transcription/platform_subtitle.rs`
  - 新增 `select_candidate` 与 `language_matches`：按请求语种筛选字幕轨，不再取 `subtitles[0]`。
  - 语种不符返回 `SubtitleLookup::Failed("SUBTITLE_LANGUAGE_MISMATCH")`；运行时只消费 `Available`，因此仍是降级语义，不会让任务失败。
  - `ai-` 前缀的机器轨置信度记 0.75，人工轨记 0.9。
- `src-tauri/src/infrastructure/transcription/runtime.rs`：调用点传入 `task.options.language`。

新增测试：

- `subtitle_language_matching_rejects_translated_tracks`

## T03 内嵌字幕抽取（done）

改动：

- 新增 `src-tauri/src/infrastructure/transcription/embedded_subtitle.rs`
  - `parse_srt`：处理 BOM、CRLF、多行 cue、`,` 与 `.` 两种毫秒分隔符、无序号块。
  - 剥离 `{...}`（ASS 样式）与 `<...>`（HTML）标签。
  - 产出 `origin=EmbeddedSubtitle`、`confidence=1.0` 的分段。
- `src-tauri/src/infrastructure/transcription/ffmpeg_preprocessor.rs`
  - `FfmpegRunOutput::Completed` 增加 `stdout`，sidecar 运行器改为同时捕获 stdout/stderr。
  - 新增 `extract_subtitle(source, track_index, control)`：`-map 0:s:N -f srt -`。
- `src-tauri/src/infrastructure/transcription/runtime.rs`
  - 新增 L1 路由：`source_info.embedded_subtitles` 非空时先尝试抽取，命中即完成，不再进入平台字幕与 ASR。
  - `select_embedded_track`：按请求语种优先匹配，无匹配退第一条。
- `src-tauri/src/infrastructure/transcription/mod.rs` 注册新模块。

新增测试：`parses_bom_crlf_and_multi_line_cues`、`strips_ass_and_html_markup`、`ignores_blocks_without_a_valid_cue`。

## T05 VAD 切段（done，实现修订见 D-9）

改动：

- `ffmpeg_preprocessor.rs` 新增 `detect_silence`：`silencedetect=noise=-35dB:d=0.4`，解析 `silence_start`/`silence_end` 并取区间中点作为边界候选。
- 新增 `parse_silence_points` 与 `value_after`。
- `runtime.rs`：预处理前先跑静音检测，把结果填入 `FfmpegPreprocessRequest.silence_points`——该字段此前恒为空，切分逻辑从未真正生效。

说明：未引入 Silero ONNX 模型。当前打包的 sidecar 只有 `sherpa-onnx-offline`，没有 VAD 变体二进制；`silencedetect` 零新增依赖、可单测，先解决「切口不在语音边界」这一主因。Silero 作为后续增强项保留。

新增测试：`silence_points_use_the_middle_of_each_silent_interval`。

## T06 Whisper 参数修复（done，参数部分）

改动：

- `whisper_sidecar.rs::build_whisper_args` 增加：`-mc 0`（关闭跨段上下文）、`-et 2.4`（熵阈值）、`-lpt -1.0`（对数概率阈值）、`-nth 0.6`（静音阈值）；中文/繁体请求追加简体中文 initial prompt。

未完成（并入 T08）：

- 默认模型仍为 `Base`。切换 `default_asr_model` 会改变模型页默认选中项，若目标模型未下载会导致创建任务直接失败；等 T08 把 FireRedASR2-AED 加入清单后再一并切换，避免出现「默认指向一个下不到的模型」。
- `store.rs` 与 `settings/mod.rs` 的默认语言不一致（`Auto` vs `Zh`），一并留到 T08 处理，避免现在改动引发设置契约测试与迁移测试连锁调整。

新增测试：`chinese_requests_disable_context_and_add_the_simplified_prompt`；同步更新 `tests/whisper_sidecar.rs` 的完整参数断言。

## T07 模型仓库改多文件包（done）

改动：

- `model_repository.rs`
  - 新增 `ModelFile { name, size_bytes, sha256, url, mirror_url }`；`ModelManifestEntry` 增加 `directory` 与 `files`。
  - 单文件模型（Whisper 系列、FireRed CTC）`directory=None`，仍落在模型根目录，旧安装无需迁移。
  - 多文件模型落在 `<模型目录>/<directory>/`，`AsrModel.path` 指向该目录。
  - 下载改为按文件循环：`download → download_one → stream_url`，进度按整包累计；每个文件独立超时/校验/重试，任一文件失败整体回滚该文件。
  - `ModelDownloadWriter` 自己完成大小与 sha256 校验后再 rename，不再依赖 `publish_part` 二次校验。
  - 新增 `file_urls`：官方 URL + 每个文件自带的镜像 URL，去掉了原先「FireRed 特判文件名」的硬编码。
  - `list` 的 `installed/valid` 改为「所有文件都在且校验通过」；`verify` 失败时删除整个包目录；`delete` 递归删除包目录。
  - 每条 `ModelFile` 的 sha256 与字节数取自 Hugging Face LFS 元数据，非估算值。

新增测试：`package_models_install_into_a_subdirectory_and_delete_with_it`；同步更新 `tests/model_repository.rs` 的 fixture 与清单断言。

## T08 FireRedASR2-AED 接入（done）

改动：

- 新增 `AsrModelId::FireRedAsr2Aed`（前端 id `fire-red-asr2-aed`）。
- 清单新增 AED 包：`encoder.int8.onnx`（817,286,833 B，sha256 `54048d66…`）+ `decoder.int8.onnx`（417,291,928 B，sha256 `b840ce71…`），合计 1,234,578,761 B。
- `fire_red_sidecar.rs`
  - `build_fire_red_args` 按 `model_path` 是否为目录走 AED 分支（`--fire-red-asr-encoder` / `--fire-red-asr-decoder`），否则维持 CTC 分支。
  - `validate_request` 接受目录形态的模型，要求 encoder/decoder 同时存在。
  - Windows 非 ASCII 路径 staging 新增 `stage_directory`，用硬链接搬运包目录。
  - **AED 不返回字级时间戳**（实测输出 `timestamps: []`），因此 AED 的切分段上限从 55s 降到 20s，用更短的 chunk 换 SRT 时间轴精度；CTC 仍保持 55s。
- `runtime.rs`：`FireRedAsr2Aed` 路由到 FireRed 引擎，并按模型选择分块上限。
- 前端：`ASR_MODEL_IDS` 增加 `fire-red-asr2-aed`；模型管理器分组、`TranscribePage` 的语言/GPU 禁用逻辑同步；补两个语言包文案。
- 顺带修复：`settings/options.ts` 用 `replace("-","")` 生成 i18n key，多连字符模型永远匹配不到文案，改为 `replace(/-/g,"")`。

校验：包内 `tokens.txt` 与仓库已打包的 `binaries/fire-red-asr2-tokens.txt` 逐字节一致（sha256 `1bc613de…`，79172 B），因此 AED 复用现有 tokens，不额外分发。

新增测试：`directory_models_use_the_aed_encoder_decoder_arguments`、`single_file_models_keep_the_ctc_arguments`。

未完成（有意延后）：

- **默认模型仍未切换**。清单里已可选 AED，但把 `default_asr_model` 指向一个「很可能未下载」的模型，会让新装用户的第一次转写直接失败（`repository.verify` 返回 E017）。正确做法是加一个「安装推荐模型」引导流程，属于 UI 范围，单列后续请求处理。

## T09 Qwen3-ASR 多语种档（done，落地 0.6B）

**选型修订**：计划里的 1.7B 在 Hugging Face 上没有逐文件仓库（`zengshuishui/Qwen3-ASR-onnx` 与 `csukuangfj2/...-1.7B-...` 均 404），只有 0.6B int8 能按文件下载并做 sha256 校验。因此落地 **Qwen3-ASR-0.6B int8**：同样支持 52 语种、22 种中文方言、自带标点，代价是精度略低于 1.7B。

改动：

- 新增 `qwen3_asr_sidecar.rs`
  - 参数：`--qwen3-asr-conv-frontend/--qwen3-asr-encoder/--qwen3-asr-decoder/--qwen3-asr-tokenizer` + `--qwen3-asr-max-new-tokens=512`。
  - JSON 解析：`text`、`lang`、`ys_log_probs`（平均对数概率映射为 0~1 置信度）。
  - Windows 非 ASCII 路径 staging（模型目录硬链接 + 音频搬运）。
  - 语言能力覆盖 `zh/zh-TW/en/ja/ko`，GPU 强制拒绝。
- 清单新增 Qwen3 包，6 个文件全部取自 HF LFS 元数据：`conv_frontend.onnx` 44,148,281、`encoder.int8.onnx` 182,491,662、`decoder.int8.onnx` 755,914,231、`vocab.json` 2,776,833、`merges.txt` 1,671,853、`tokenizer_config.json` 12,487，合计 987,015,347 B。
  - tokenizer 三个文件在远端位于 `tokenizer/` 子目录，下载时拍平到包目录，正好满足 `--qwen3-asr-tokenizer=<包目录>`。
- `runtime.rs`：新增 `TranscriptionEngine::Qwen3`，分块上限 20s（Qwen3 解码预算限制），`validate_model_options` 拒绝 GPU 与 FireRed 的语言限制分离。
- 前端：新增 `qwen3-asr` 模型 id 与「Qwen3-ASR 系列」分组；GPU 选项对 Qwen3 一并禁用；中英文案补齐。

新增测试：`arguments_point_at_every_package_file`、`json_parser_emits_offset_text_and_language`、`empty_text_is_reported_as_no_speech`。

## T10 OCR 运行时 spike（done）

完整结论见 `artifacts/ocr-spike.md`。要点：

- 实测 `rten 0.28.0`：纯 Rust，32 个依赖，构建 15.28s，**不需要 C++ 工具链，不下载 ONNX Runtime 二进制**。
- PP-OCRv4 中文 det（4.7 MB）与 rec（10.9 MB）均通过 `Model::load_file` 图构建，无「不支持算子」报错。
- 决策：OCR 用库内 `rten` 推理，不新增 sidecar；模型随包分发（合计约 16 MB）。
- 残余风险：图加载 ≠ 运行期算子全覆盖，T12 用真实字幕帧首次运行时验证；识别率不足时用 `rec` 置信度阈值降级到 ASR。

## T11 / T12 / T13（模块完成，尚未接入运行时）

改动：

- `ffmpeg_preprocessor.rs`：新增 `extract_gray_frames`（`fps`、`width`、`height`、`max_frames` 由调用方给出）。
- `hard_subtitle_ocr.rs`
  - `GrayFrame` + `detect_bands`：只在画面下 45% 找字幕带，按「相邻像素差 ≥ 40 的强边缘数 ≥ 2」判定文字行，跨帧过半才激活，最多两条带。
  - `read_bands`：逐帧稀疏签名做变化门控，仅变化帧送 OCR；相同文本合并时间轴，产出 `HardSubtitleOcr` 段落。
- `ocr_engine.rs`：`rten` 加载 PP-OCRv4 rec，`[1,3,48,W]` 归一化输入、双线性缩放、CTC 贪心解码 + softmax 置信度、词典映射。
- `fusion.rs`：按信源优先级（内嵌字幕 > 平台字幕 > Fused > OCR > ASR）取骨架，重叠段做归一化编辑距离比对（≥0.85 视为一致），冲突段保留 `alternates` 并把置信度压到 0.4，冲突占比 >15% 时置 `review_recommended`。
- `Cargo.toml` 新增 `rten` / `rten-tensor`。

新增测试 11 个（lib 测试从 117 增至 137）：字幕带检测、空白帧、签名变化、OCR 预处理缩放、融合一致/冲突/补位/相似度。

**尚未完成的部分（必须说明）**：

- 三个模块都还没有接入 `runtime.rs`，也没有随包分发 OCR 模型与词典，因此在应用里**暂时不会触发**。
- 接入需要：`TranscriptionSidecarPaths` 增加 OCR 模型/词典路径、`lib.rs` 解析资源、`run_pipeline` 增加 OCR 候选与 `fuse` 调用、`tauri.conf.json` 增加资源条目。

## 接线与前端置信度（本轮的收尾）

- `runtime.rs`
  - `TranscriptionSidecarPaths` 增加 `ocr_model` / `ocr_dictionary`。
  - 新增 `ocr_candidate`：仅在「有视频轨 + OCR 模型与词典都存在」时执行，帧宽 480、最多 900 帧、fps 按时长自适应（0.2~2.0），模型推理放在 `spawn_blocking`。
  - ASR 与 OCR 结果进入 `fuse`，产出 `PipelineReport`（信源、冲突数、低置信度段数、引擎名）并写入结果。
  - `complete_recognition` 增加 `pipeline` 参数；内嵌字幕与平台字幕路径传 `None`。
- `lib.rs`：解析 `pp-ocrv4-rec.onnx` 与 `ppocr_keys_v1.txt` 资源路径。
- `tauri.conf.json`：新增两个 OCR 资源条目与 `licenses/pp-ocrv4-NOTICE.txt`。
- 随包新增 `binaries/ocr/pp-ocrv4-rec.onnx`（10,857,958 B，sha256 `48fc40f2…`）与 `binaries/ocr/ppocr_keys_v1.txt`（26,250 B，6,623 条词表，sha256 `a1c84d9b…`）。
- 前端 `TranscriptEditor.vue`：冲突段左侧红条、低置信度段黄条、`reviewRecommended` 徽标、hover 提示；中英文案补齐。

## 验证

| 项 | 命令 | 结果 |
| --- | --- | --- |
| Rust 构建 | `cargo build -j 1` | 通过，无新增 warning |
| Rust 单元测试 | `cargo test --lib -j 1` | 137 passed / 0 failed / 1 ignored |
| 前端类型检查 | `npm run typecheck` | 通过 |
| 前端测试 | `npm test` | 58 文件 / 242 用例通过 |
| 前端构建 | `npm run build` | 通过 |
| Rust 集成测试 | 逐 target：`cargo test --test <name> -j 1` | **31 个 target 全部通过，合计 143 个用例** |

## T17 评测脚本（done）

新增 `scripts/asr-bench/run.mjs`（零依赖）与 `cases.example.json`，并在 `package.json` 注册 `npm run bench:asr`。

设计取舍：脚本**不运行 ASR**，只对已导出的文本做客观统计，因此不会因缺模型或环境问题失败。指标：关键实体召回率、缺失实体清单、疑似幻觉行数（内置 Whisper 套话词表）、字符数与平均行长，并对多引擎给出与基线的差值。

已用两份构造样本冒烟验证：正确识别出「请不吝点赞订阅转发打赏…」这类幻觉行，并算出 100% vs 0% 的实体召回差。

Windows 下 PowerShell `Set-Content -Encoding UTF8` 会写 BOM，脚本已做 BOM 剥离。

## T18 许可与打包（done）

- 新增 `src-tauri/licenses/pp-ocrv4-NOTICE.txt`（PP-OCRv4 rec 与词表，PaddleOCR Apache-2.0，附 sha256）。
- 新增 `src-tauri/licenses/rten-MIT-LICENSE.txt`（rten / rten-tensor 为 MIT OR Apache-2.0，此处按 MIT 分发）。
- `tauri.conf.json` 资源条目补齐：OCR 模型、OCR 词表、两份许可。
- AED / Qwen3 为运行时按需下载，不随包分发，其许可随模型仓库提供。

## 验证

| 项 | 命令 | 结果 |
| --- | --- | --- |
| Rust 构建 | `cargo build -j 1` | 通过 |
| Rust 单元测试 | `cargo test --lib -j 1` | 137 passed |
| 评测脚本 | `node run.mjs cases.json` | 输出符合预期 |
| 前端 | 本轮未改动 | 沿用上一轮 242 用例通过 |

## T15 内存守卫与 E018（done）

- `models/app_error.rs` 新增 `E018`；前端 `APP_ERROR_CODES` 与 IPC 客户端测试同步。
- 新增 `infrastructure/transcription/memory_guard.rs`
  - `required_memory_bytes`：Tiny 0.5G / Base 1G / Small 2G / Medium 3G / LargeV3 4G / FireRed CTC 2G / AED 2G / Qwen3 3G（保守估计）。
  - `available_memory_bytes`：`sysinfo` 读取可用物理内存；探测失败返回 `None`。
  - `check_capacity`：可用内存低于需求返回 `E018 / MODEL_MEMORY_INSUFFICIENT`；探测失败时**放行**，不拦用户。
- `runtime.create_task` 在模型校验后调用 `ensure_capacity`，让用户在创建任务时就拿到结果，而不是跑一半 OOM。
- 新增 3 个单元测试（拒绝、放行、档位单调）。

## T16 OpenASR 8B（done，最小集成）

调研+实测过程见 `artifacts/openasr-8b-test.md`。最终交付的是「外部工具」形态，不随包分发 OpenASR：

- `models/transcription.rs`：新增 `AsrModelId::FireRedAsr2Llm`（前端 id `fire-red-asr2-llm`）。
- `model_repository.rs`：清单新增该条目，`files` 为空表示「外部工具托管」；`model_state` 对空文件列表直接返回 `installed/valid = true`，不参与下载与校验。
- 新增 `openasr_sidecar.rs`
  - 二进制解析顺序：`BILICATCH_OPENASR_BIN` → `%LOCALAPPDATA%\OpenASR\openasr.exe` → `%PROGRAMDATA%\BiliCatch\openasr\openasr.exe` → 程序目录下的 `openasr/`。
  - 调用：`openasr transcribe <wav> -m firered2-llm -f json --offline [-l zh|en]`。
  - 解析 `segments`（秒）为毫秒分段，**丢弃纯标点片段**（这正是上游 SRT 渲染崩溃的诱因）。
  - 失败分类：`OPENASR_BACKEND_INTEGRITY`（用错包）/ `OPENASR_MEMORY` / `SIDECAR_FAILED`。
- `runtime.rs`：新增 `TranscriptionEngine::OpenAsr`，分块上限 40s，引擎名 `openasr-firered2-llm`。
- `memory_guard.rs`：该档要求 9 GB 可用内存，不足时创建任务即返回 `E018`。
- `validate_model_options`：该档只支持中英，日韩与强制 GPU 会被拒绝。
- 前端：模型列表新增条目（归入 FireRedASR2 系列），语言/GPU 禁用逻辑同步，中英文案补齐。

新增测试 3 个（参数拼装、JSON 解析含偏移与纯标点过滤、空稿判无语音）。

**使用方式**：把 OpenASR 的 `-neutral` 发行包解压到任意目录，设置环境变量 `BILICATCH_OPENASR_BIN` 指向其中的 `openasr.exe`，执行一次 `openasr pull firered2-llm` 下载 4.7 GB 模型，然后在应用里选择「FireRedASR2 LLM 8B（OpenASR）」。

### 8B 实测与修正（同日）

空闲内存恢复到 34.2 GB 后完成真机测试：

| 项 | 结果 |
| --- | --- |
| 模型 | `firered2-llm:q4`，5,097,451,168 B，sha256 `8f42a589…` |
| 40 s 音频耗时 | **14.6 s**（RTF ≈ 0.37） |
| 标点 | 无；装了 `firered-punc` 仍未生效 |
| 时间轴 | `subtitle_cues` 按字数切分，会在词中截断，不可直接用 |

期间发现并修复一个实现 bug：**8B 不接受 `-l` 参数**（`unsupported_capability`），原实现无条件传 `-l zh`，会导致该档直接失败。已改为不传语言，并补了对应单测。

**仍未做**：随包分发（需要按平台挑选 54.2 MB 的 12 个文件，Tauri resources 不支持平台差异化）。

## 仍未完成

| 任务 | 状态 | 说明 |
| --- | --- | --- |
| OpenASR 随包分发 | 未做 | 需按平台构建脚本；当前依赖用户自行放置二进制 |

## 验证

| 项 | 命令 | 结果 |
| --- | --- | --- |
| Rust 单元测试 | `cargo test --lib -j 1` | 143 passed / 0 failed / 1 ignored |
| Rust 集成测试（关键 target） | `cargo test --test <name> -j 1` | model_repository、settings_*、transcription_*、whisper/fire_red、task_manager 全部通过 |
| 前端 | `npm run typecheck` / `npm test` | 通过 / 242 用例通过 |

### 调研与实测（历史记录）

调研结论：**当前打包形态不成立，因此未接入。** 证据：

1. OpenASR v0.1.41 的 Windows 包不是单文件：`openasr.exe` 46 MB 之外还依赖 `ggml.dll`、`ggml-base.dll` 和 10 个 `ggml-cpu-*.dll`（合计约 10 MB），必须与 exe 同目录才能加载。
2. Tauri 的 `externalBin` 每个目标只带一个可执行文件，且 `bundle.resources` 不支持按平台差异化。要同时满足 Windows x64/arm64 与 macOS x64/arm64，每个安装包都会带上全部平台的文件（约 180 MB）。
3. 模型是 4.7 GB 的 `.oasr` 包，另需独立下载。

### 实测结果（2026-11-15）

按用户要求真机跑了一遍，结论比调研更硬，完整报告见 `artifacts/openasr-8b-test.md`：

| 项 | 结果 |
| --- | --- |
| `openasr --version` / `show firered2-llm` | 可执行；目录可见 `q4_k` / `8b` / `zh,en` |
| `transcribe --help` | 契约齐全（`-m`、`--model-pack`、`-f srt\|json\|…`、`--benchmark`、`--hotword`） |
| 真实中文音频转写 | **失败**：`bundled backend file 'ggml-base.dll' failed integrity verification` |
| 清单核对 | **11 个 DLL 全部不匹配**，实际大小恰好多 15,624 字节 |
| 8B 内存 | 需 8.7 GB，本机可用 5.2 GB，未尝试加载 |

失败与模型大小无关：用小模型 `sensevoice-small` 同样 fail-closed。固定偏移 15,624 字节表明清单与打包产物不是同一批，属**上游 v0.1.41 的发行缺陷**。

**已找到可运行方案并验证**：改用同版本的 `-neutral` 资产（清单 11/11 通过）后，272 秒中文音频 7.7 秒转写成功；但不能用 `-f srt`（首段为纯标点时会触发强制对齐报错），需用 `-f json` 取 `segments` 自行生成字幕。单平台包体 12 文件 / 54.2 MB，8B 峰值内存 8.7 GB。

可行路径（未实施，建议单独立项）：用按平台的构建脚本在打包前挑选对应目录，作为 `resources` 整体分发并运行时按绝对路径调用；或改为引导用户自行安装 OpenASR 后再指向其路径。两条路都超出「一个 sidecar 二进制」的复杂度，且本环境无法做真机验证。

## 仍未完成

| 任务 | 状态 | 说明 |
| --- | --- | --- |
| T16 OpenASR 8B | blocked | 见上；需要独立的打包方案 |
| Rust 集成测试 | 已通过 | 不能用 `cargo test` 一次跑全部（并行链接会耗尽本机 page file）；改为逐 target 执行 `cargo test --test <name> -j 1`，31 个 target 全部通过 |

## 环境问题记录

加入 `rten` 后，本机 page file 不足，`cargo test` 并行链接全部测试二进制会失败（`os error 1455 页面文件太小`）。可用的验证方式是：

```
$env:CARGO_BUILD_JOBS='1'; cargo test --lib -j 1
```

结果：140 passed / 0 failed / 1 ignored。集成测试不能用 `cargo test` 一次跑（会并行链接全部 target），改为逐 target 执行 `cargo test --test <name> -j 1`，31 个 target 全部通过，合计 143 个用例。

## 验证

| 项 | 命令 | 结果 |
| --- | --- | --- |
| Rust 构建 | `cargo build -j 1` | 通过 |
| Rust 单元测试 | `cargo test --lib -j 1` | 137 passed |
| Rust 集成测试 | `cargo test -j 1` | **未跑通**：环境 page file 不足，非代码问题 |

## 下一步

- 把 OCR 与融合接入 `runtime.rs`，并把 PP-OCRv4 rec + 词典纳入打包（T12 收尾）
- T14 置信度前端、T15 内存守卫、T16 OpenASR 8B、T17 评测、T18 许可
