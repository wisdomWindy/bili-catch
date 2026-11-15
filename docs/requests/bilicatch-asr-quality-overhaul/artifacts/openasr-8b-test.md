# T16 OpenASR 8B 实测报告

- 请求：`bilicatch-asr-quality-overhaul`
- 任务：T16
- 日期：2026-11-15
- 结论：**普通包不可用（清单与 DLL 全部不一致）；改用 `-neutral` 包后转写成功。`-f srt` 另有上游缺陷，需用 `-f json`。8B 仍受本机内存限制无法验证。**

## 1. 测试环境

- Windows x86_64，48 GB 物理内存，测试时可用 **5.2 GB**
- 磁盘可用 17.5 GB
- OpenASR `v0.1.41`，资产 `openasr-0.1.41-windows-x86_64.zip`（21.2 MB）

## 2. 验证到的部分

| 检查 | 结果 |
| --- | --- |
| `openasr --version` | `openasr 0.1.41`，可执行 |
| `openasr show firered2-llm` | 目录可见：`q4_k`、`8b`、`oasr`、`native` 后端、语言 `zh`/`en` |
| `openasr transcribe --help` | 契约清晰：`-m/--model`、`--model-pack`、`-f srt\|json\|verbose_json\|vtt\|text\|markdown`、`-o`、`--benchmark`、`--offline`、`--hotword`、`--word-timestamps` |

也就是说，**适配器需要的那条 CLI 契约是齐的**，接进来技术上没有障碍。

## 3. 失败点：自带后端完整性校验不通过

用 `sensevoice-small` 跑真实中文音频（272 秒）时失败：

```
native_transcribe: stage=transcribe_failure error_category=decode mem_available_mib=15033
Error: ... auxiliary stage 'longform-vad' exhausted its execution plan after
CapacityUnavailable at candidate-activation-reserve: the bundled backend directory
could not be resolved safely: bundled backend file 'ggml-base.dll'
failed integrity verification
```

对照 `openasr-backend-bundle-v1.json` 逐文件核对，**11 个 DLL 全部不匹配**，且每个实际大小都恰好比清单多 **15,624 字节**：

| 文件 | 清单大小 | 实际大小 | 清单 sha256 | 实际 sha256 |
| --- | --- | --- | --- | --- |
| ggml-base.dll | 716,800 | 732,424 | 137a6b17… | bf425fa2… |
| ggml.dll | 62,976 | 78,608 | 062d3435… | c7a30dee… |
| ggml-cpu-x64.dll | 813,568 | 829,200 | c6a65eb0… | 31b5b54f… |
| 其余 8 个 `ggml-cpu-*.dll` | — | 均 +15,624 | — | 均不一致 |

固定偏移 15,624 字节说明清单与被打包的文件不是同一批产物（疑似签名/尾部数据在清单生成之后追加）。OpenASR 按设计 fail-closed，因此**在 Windows 上这个版本无法完成任何转写**，与模型大小无关——用小模型同样失败。

**跨版本复核**：`v0.1.40` 的 Windows 包同样 11/11 个 DLL 不匹配，说明这是发行流水线的系统性问题，不是 `v0.1.41` 的偶发事故。

## 4. 8B 实测（已完成）

首次测试时后台编译占满内存（可用仅 5.2 GB），未能运行。空闲后可用内存 34.2 GB，完成实测：

| 项 | 结果 |
| --- | --- |
| 模型 `firered2-llm:q4` | 5,097,451,168 B，sha256 `8f42a589…`，与 HF 模型卡一致 |
| 40 秒中文片段的转写耗时 | **14.6 s**（RTF ≈ 0.37） |
| 输出结构 | `text` + `segments`（整段 1 条）+ `subtitle_cues`（按字数切 7 条） |
| 文本质量 | 与 SenseVoice 对比，8B 的断句更完整、无「这是第四次。」这类插入错误 |
| 标点 | **无**。即使安装了 `firered-punc:fp16`（203 MB）也没有生效 |
| 时间轴 | `subtitle_cues` 是按字符比例切分的近似值，会在词中间断开（「好欢」/「迎大家」），不适合直接做字幕 |

对照：同一片段 SenseVoice 2.3 s 完成、自带标点，但有插入错误。

**结论**：8B 可用，但输出需要后处理——标点缺失，且 `subtitle_cues` 的时间轴不可靠，只能用它自己的 `segments`（每 40 秒一条）。BiliCatch 当前取 `segments` 是对的。

## 5. 对 T16 的影响

| 阻塞项 | 性质 | 处置 |
| --- | --- | --- |
| 发行包完整性清单与 DLL 不一致 | **上游缺陷**，可复现 | 需等上游修复，或换用其他版本/自行构建 |
| 后端是 11 个 DLL + 主程序（≈56 MB/平台） | Tauri 打包形态问题 | 需按平台构建脚本或独立安装引导 |
| 8B 需 8.7 GB 内存 | 硬件门槛 | 只能作为可选档，并在创建任务前用 T15 的守卫拦截 |

## 6. 复现命令

```powershell
# 1. 下载并解压发行包
Invoke-WebRequest 'https://github.com/QuintinShaw/openasr/releases/download/v0.1.41/openasr-0.1.41-windows-x86_64.zip' -OutFile win.zip
Expand-Archive win.zip -DestinationPath win

# 2. 核对完整性
cd win\openasr-0.1.41-windows-x86_64
$json = Get-Content openasr-backend-bundle-v1.json -Raw | ConvertFrom-Json
$json.files | ForEach-Object {
  $p = Join-Path (Get-Location) $_.filename
  "{0} expected={1} actual={2}" -f $_.filename, $_.size_bytes, (Get-Item $p).Length
}

# 3. 任意转写都会 fail-closed
.\openasr.exe transcribe <any.wav> -m sensevoice-small -f srt -y
```

## 7. 建议

1. 先向上游报告该完整性清单问题（附上本报告的偏移量证据），等修复版本。
2. 修复前不要接入 8B 档：接进来只会让用户下载 4.7 GB 后得到一个 fail-closed 错误。
3. 若上游短期不修，替代方案是自建 OpenASR（Rust 源码 Apache-2.0）或继续用 FireRedASR2-AED（已接入，1.2 GB、2 GB 内存）。

## 8. 解决方案（已验证可用）

### 8.1 换用 neutral 包

同一版本还有第二个 Windows 资产 `openasr-0.1.41-windows-x86_64-neutral.zip`，其清单与 DLL **完全一致（11/11 通过）**。普通包与 neutral 包的区别在于是否预置 CPU 变体分派，普通包的产物在清单生成后被重新处理过，因此校验必然失败。

```
https://github.com/QuintinShaw/openasr/releases/download/v0.1.41/openasr-0.1.41-windows-x86_64-neutral.zip
```

### 8.2 用 `-f json`，不要用 `-f srt`

```
.\openasr.exe transcribe <audio> -m sensevoice-small -f json -l zh --offline
```

实测输出（272 秒中文演讲，7.7 秒完成）：

```json
{
  "text": "。朋友们晚上好，欢迎大家来参加今天晚上的活动，谢谢大家。…",
  "segments": [{ "start": 2.19, "end": 22.03, "text": "。" }, …],
  "subtitle_cues": [...],
  "timeline_quality": {...}
}
```

`-f srt` / `--word-timestamps=aligned` 会触发强制对齐，而**首段是纯标点 "。"** 时对齐器报错：

```
Forced alignment failed: segment 0: qwen3-forced-aligner transcript is empty after normalization
```

这是上游 SRT 渲染缺陷。规避方式：取 `-f json` 的 `segments`（秒为单位）自己生成 SRT，这与 BiliCatch 现有的导出链路完全兼容。

### 8.3 打包与内存

| 项 | 数值 |
| --- | --- |
| 单平台文件数 | 12（1 个 exe + 11 个 DLL） |
| 单平台体积 | 54.2 MB |
| 8B q4_k 峰值内存 | 8.7 GB |

集成到 BiliCatch 需要：按平台构建脚本挑选对应目录、整体作为 resources 分发、以绝对路径调用；模型 4.7 GB 按需下载；用 T15 的内存守卫在创建任务时拦截内存不足的机器。
