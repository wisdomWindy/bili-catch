# 请求：BiliCatch 转写质量重构（信源阶梯 + VAD + 多引擎）

## 请求标识

`bilicatch-asr-quality-overhaul`

## 来源

- 对话来源：2026-11-14 至 2026-11-15 的连续讨论
- 用户实测反馈：现有转写「存在错别字」「有的时候完全与视频的音频内容没半点关系」
- 参考流程：豆包 `probe-video-metadata → video-ocr（硬字幕）→ asr-subtitles` 的三级信源降级
- 用户取向确认：2026-11-15「我只考虑效果，什么效果好就是用什么模型」

## 业务摘要

在不改变「完全本地、Windows + macOS 双平台」这一前提的基础上，把转写链路从当前的「Whisper 单引擎直出」重构为「信源阶梯 + VAD 切段 + 多引擎可选 + 质量分融合」。

目标是同时消除两类缺陷：一是静音/音乐段被解成与音频无关的文字（Whisper 幻觉），二是有现成文字信源的视频仍然被强行送进 ASR 而产生的错别字。

## 目标（可观测的完成信号）

1. 自建评测集（10 条 B 站素材）上，与音频无关的幻觉段数量为 **0**。
2. 同一评测集上，关键实体（人名、数字、术语、英文缩写）错误数下降 **≥ 50%**。
3. 带内嵌字幕轨的视频，转写结果直接来自字幕轨，与人工稿一致率 **≥ 99%**。
4. 带稳定硬字幕的视频，OCR 覆盖画面句子的比例 **≥ 90%**，且低置信度段落被显式标记。
5. 所有导出格式的段落起止时间与人工标注偏差 **≤ 1s**。
6. 「高精度」档可选用 FireRedASR2-LLM，内存不足时返回结构化错误而非崩溃。
7. 取消、进度、缓存复用、五种导出格式、任务页集成均不回归。
8. 新增模型全部为 Apache-2.0 或 MIT，打包清单与 NOTICE 同步更新。

## 触发条件

用户对现有转写质量给出否定结论，并确认以「效果」为唯一选型标准；同时提供了豆包的三级信源工作流作为参照。

## 上下文来源

- 现有实现：`src-tauri/src/infrastructure/transcription/*`、`src-tauri/src/services/transcription/*`
- 现有数据模型：`src-tauri/src/models/transcription.rs`、`src-tauri/src/models/settings.rs`
- 现有打包：`src-tauri/tauri.conf.json`（ffmpeg / whisper-cli / sherpa-onnx）
- 现有前端契约：`src/contracts/*`、`src/features/transcription/*`
- 外部依据：FireRedASR2S 技术报告、Qwen3-ASR 模型卡、sherpa-onnx 文档、OpenASR 文档

## 人工交接点

- 本设计（`design/solution.md`）审批
- 规格与实施计划审批
- 评测集素材与人工 ground truth 确认
- 「高精度 8B 档」是否随包分发的产品决定

## 影响范围

- Rust：媒体探测、预处理、字幕抽取、OCR、ASR 适配、模型仓库、运行时编排、数据模型、错误码
- 前端：模型管理页、转写选项、结果页置信度展示、i18n
- 打包：模型清单、sidecar、许可证与 NOTICE

## 不做的事（范围外）

- 云端 ASR / 任何商业 API 依赖
- 说话人分离与角色标注
- 实时流式转写
- Linux 原生支持
- 在线歌词/字幕抓取等第三方版权内容
