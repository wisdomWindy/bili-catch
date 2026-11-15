# 规格澄清与决策记录

- 请求：`bilicatch-asr-quality-overhaul`
- 日期：2026-11-15

## 已定决策

| 编号 | 问题 | 决策 | 依据 |
| --- | --- | --- | --- |
| D-1 | 8B 高精度档是否随包分发 | **否，按需下载**，模型页标注「需 16 GB 内存」 | 用户「按建议来」；4.7 GB 包体不适合随安装包 |
| D-2 | 硬字幕 OCR 是否默认启用 | **默认启用**，检测到硬字幕即生效，设置可关 | 用户「按建议来」；错别字收益最大 |
| D-3 | 是否保留 Whisper 引擎 | **保留**，仅作冷门语种兜底，不再是默认，默认模型从 `Base` 换为 `FireRedASR2-AED` | 用户「按建议来」 |
| D-4 | 平台字幕与 OCR 冲突时以谁为准 | **以 OCR 为准**，平台字幕仅在其独占时段使用 | 可复现、无登录与版权依赖 |
| D-5 | 是否引入 OpenASR 外部二进制 | **引入，但仅作为 8B 档的可选运行时**，只依赖 `transcribe` CLI | 局限在单一适配器，可替换 |
| D-6 | 默认 ASR 引擎 | FireRedASR2-AED int8（中英） | 精度/体积/时戳平衡 |
| D-7 | 多语种引擎 | Qwen3-ASR-1.7B（`ja`/`ko` 或其他语种） | 52 语种、自带标点 |
| D-8 | VAD 实现 | Silero VAD，通过 sherpa-onnx 运行时 | 2 MB、跨平台、已具备运行时 |
| D-9 | VAD 实施顺序修订 | 先用 ffmpeg `silencedetect=noise=-35dB:d=0.4` 产出静音中点作为分块边界；Silero 留作后续增强 | 当前打包的 sidecar 只有 `sherpa-onnx-offline`，没有 VAD 变体二进制；silencedetect 零新增依赖、跨平台、可测，先解决「切口不在语音边界」这一主因 |

## 开放问题

| 编号 | 问题 | 现状 | 处理方式 |
| --- | --- | --- | --- |
| O-1 | OCR 运行时选 `ort` 还是 `rten` | 未定 | P3 首日 spike 后锁定；`ort` 优先（复用已打包 onnxruntime），冲突则 `rten` |
| O-2 | PP-OCRv5 模型文件许可 | 未逐文件复核 | P3 开始前复核；不满足则退到 PP-OCRv4 |
| O-3 | 8B 内存探测方式 | 未定 | 实施时按平台分别实现：Windows 用 `GlobalMemoryStatusEx`，macOS 用 `sysctl hw.memsize` 结合当前占用估算；测试通过注入 trait 覆盖 |
| O-4 | 评测集素材来源 | 未定 | 由用户提供或从已下载任务中选 10 条；P6 前确认 |

## 不影响实施的假设

- 现有 `ProcessControl` 语义足以支撑 OCR/ASR 双分支取消。
- ffmpeg 版本（9.0.1）支持 `-map 0:s:N` 与 `-f srt -`。
- 前端可在不改变路由的前提下扩展契约字段。
