# 来源快照：自动转写行视觉优化

## 逐字来源证据

| ID | 来源 | 原文 |
| --- | --- | --- |
| S1 | 用户直接指令（2026-11-14） | “图片中红框部分的UI样式我认为不协调，修改使之变得更美观” |
| S2 | 用户截图（红框） | 红框覆盖“下载完成后转写 使用当前转写预设生成本地文本”复选框一行 |

## 代码事实

| ID | 位置 | 事实 |
| --- | --- | --- |
| C1 | `DownloadOptions.vue:103-106` | 该行为 `label.download-transcription-option`，内含 `input.switch-control[role=switch]` 与 `<span><strong>…</strong><small>…</small></span>` |
| C2 | `download-center.css` | 不存在 `.download-transcription-option` 与 `.switch-control` 的任何规则；仅有 `.option-grid`、`.mode-selector` 等同级样式 |
| C3 | `tokens.css` | 品牌色 `--color-brand:#e84a72`，边框 `--color-border`，次要文字 `--color-text-muted`，焦点色 `--color-focus`，控件圆角惯例 6px |
| C4 | `download-center.css:43-51` | 模式选择器使用 `color-mix(in srgb, var(--color-brand) 8%, var(--color-surface))` 的品牌浅底；下拉框使用 1px 边框 + 6px 圆角 |

## 已确认口径

- 只改视觉，不改行为与文案（S1 限定“样式”）。
- 该行在 `mode !== "video-only"` 时显示（既有代码事实，保持）。
- 开关语义为 `role="switch"`（既有），保持可键盘操作。

## 开放问题

无。范围与判定标准清晰，不需要额外产品确认。

## 范围允许清单

| ID | 允许内容 |
| --- | --- |
| A1 | 该行的容器样式（边框、圆角、内边距、背景、间距） |
| A2 | 胶囊开关的轨道/滑块/开启/焦点态 |
| A3 | 标题与说明的排版（字号、行高、颜色、换行） |
| A4 | 悬停与聚焦反馈 |

明确不做：改文案、加新开关、改显隐条件、改下载设置其他控件。
