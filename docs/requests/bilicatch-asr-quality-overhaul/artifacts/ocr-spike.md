# OCR 运行时 Spike 结论

- 请求：`bilicatch-asr-quality-overhaul`
- 任务：T10
- 日期：2026-11-15
- 结论：**采用 `rten`（纯 Rust ONNX 推理），不引入 C++ 侧车**

## 1. 背景

T11/T12 需要在 Windows 与 macOS 上做硬字幕 OCR。候选运行时：

| 方案 | 风险 |
| --- | --- |
| `ort`（ONNX Runtime 绑定） | 与已打包的 sherpa-onnx 所用 onnxruntime 可能版本冲突；需要额外分发 dll/dylib |
| `rten`（纯 Rust 推理） | 算子覆盖有限，遇不支持算子会在加载期报错 |
| 外部 OCR 侧车（RapidOCR CLI 等） | 无官方跨平台免 Python 二进制 |
| 系统 API（Windows.Media.Ocr / macOS Vision） | 两套实现，行为不一致 |

## 2. 实测

环境：Windows x86_64，Rust 1.98.1，`rten 0.28.0`。

1. 依赖体积：`cargo add rten` 引入 32 个 crate，全部为 Rust 实现，**不需要 C++ 工具链，不下载 ONNX Runtime 二进制**。
2. 构建耗时：`cargo build` 15.28s（debug）。
3. 模型加载：从 HF `SWHL/RapidOCR` 下载 PP-OCRv4 中英文模型，用 `Model::load_file` 加载：

```
LOADED ch_PP-OCRv4_det_infer.onnx  inputs=1 outputs=1
LOADED ch_PP-OCRv4_rec_infer.onnx  inputs=1 outputs=1
```

   两个模型均在图构建阶段通过，未报不支持算子。

4. 模型文件（Apache-2.0，来自 PaddleOCR 官方导出）：

| 文件 | 大小 | sha256 |
| --- | --- | --- |
| `ch_PP-OCRv4_det_infer.onnx` | 4,745,517 | `d2a7720d45a54257208b1e13e36a8479894cb74155a5efe29462512d42f49da9` |
| `ch_PP-OCRv4_rec_infer.onnx` | 10,857,958 | `48fc40f24f6d2a207a2b1091d3437eb3cc3eb6b676dc3ef9c37384005483683b` |

## 3. 决策

- 采用 `rten`，OCR 引擎作为库内模块（`infrastructure/transcription/ocr_engine.rs`），不新增 sidecar 二进制。
- 打包影响：只增加约 16 MB 的 ONNX 模型，无 dll/dylib，macOS 也不需要额外签名项。
- 与 sherpa-onnx 完全解耦：两者各自加载模型，互不影响运行时版本。

## 4. 残余风险

| 风险 | 处置 |
| --- | --- |
| 图加载通过不等于运行期全部算子可用 | T12 用真实字幕帧跑一次 det+rec，首次运行即验证 |
| PP-OCRv4 在竖屏/花字/低分辨率下的识别率未知 | T12 设 `rec` 置信度阈值，低于阈值的句子丢弃并降级到 ASR |
| 中文识别词典需要额外分发 | 随包附带 `ppocr_keys_v1.txt`（约 20 KB），与模型一并校验 |

## 5. 对后续任务的影响

- T11 只需实现抽帧 + 字幕带检测 + 变化门控，与运行时无关。
- T12 增加 `ocr_engine.rs`：图像预处理（归一化、resize）、DB 后处理、CTC 解码、词典加载。
- 新增依赖仅 `rten`（推理）与 `image`（解码帧），两者均无 C 依赖，符合 Windows/macOS 双平台打包约束。
