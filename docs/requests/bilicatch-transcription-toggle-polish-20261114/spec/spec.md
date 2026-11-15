# 规格：自动转写开关行视觉优化

## 交付单元

`bilicatch-transcription-toggle-polish-20261114`（direct-change，S1 local）。

## 背景

见 `design/page-design.md`：该行缺少专属样式，表现为无容器、默认方形复选框、标题与说明内联。

## 范围内

- `.download-transcription-option` 容器与排版。
- `.switch-control` 胶囊开关的默认/开启/悬停/聚焦态。
- 深色主题下的颜色变量适配。

## 范围外

- 文案、显隐条件、开关行为、store、其他控件、布局结构。
- 新增图标、动画库、第三方组件。

## 视觉合同

| 项 | 值 |
| --- | --- |
| 容器布局 | `display:flex; align-items:center; gap:10px`；`width:100%`；`margin-top:16px`；`padding:12px 14px` |
| 容器外观 | `border:1px solid var(--color-border)`；`border-radius:8px`；`background: color-mix(in srgb, var(--color-surface-muted) 55%, var(--color-surface))` |
| 容器悬停 | `border-color: color-mix(in srgb, var(--color-text-muted) 35%, var(--color-border))` |
| 开启态容器 | `border-color: color-mix(in srgb, var(--color-brand) 45%, var(--color-border))`；`background: color-mix(in srgb, var(--color-brand) 6%, var(--color-surface))` |
| 开关尺寸 | 38×22，`border-radius:999px`，`flex:none` |
| 轨道默认 | `background: color-mix(in srgb, var(--color-text-muted) 30%, var(--color-surface))` |
| 轨道开启 | `background: var(--color-brand)` |
| 滑块 | 18×18，白色，`top:2px;left:2px`，轻阴影；开启 `translateX(16px)` |
| 过渡 | `background 150ms ease, transform 150ms ease, border-color 150ms ease` |
| 焦点 | `input:focus-visible` → `outline:2px solid var(--color-focus); outline-offset:2px` |
| 文案容器 | `display:grid; gap:2px` |
| 标题 | `font-size:13px; font-weight:600; line-height:1.35; color:var(--color-text)` |
| 说明 | `font-size:12px; line-height:1.45; color:var(--color-text-muted)` |
| 光标 | 容器 `cursor:pointer` |

行为不变：`checked` 绑定、`@change` 事件、`mode === "video-only"` 时隐藏、`role="switch"` 均保持原样。

## 边界

- 说明文本在窄容器下换行，不截断、不溢出。
- 深色主题通过变量自动适配，不写死颜色。
- 不改变点击热区语义：整行仍是 label。

## 验收标准

- AC-1：`.download-transcription-option` 具备边框、圆角、内边距与 16px 上间距。
- AC-2：`.switch-control` 去除原生外观，呈胶囊形；开启为品牌色且滑块右移；有关键盘焦点环。
- AC-3：标题与说明分两行显示，说明为 12px 次要色。
- AC-4：行为、文案、显隐条件、可访问性语义与改动前一致。
- AC-5：`pnpm typecheck`、`pnpm test`、`pnpm build` 通过；demo 模式视觉复核无错位。

## 风险

| 风险 | 处理 |
| --- | --- |
| `color-mix` 兼容性 | 项目已在 `.mode-selector` 使用，WebView2 支持 |
| 开关在深色主题对比不足 | 轨道使用文字色混合，非固定灰 |
| 视觉改动影响测试 | 现有测试只断言结构/事件，不依赖样式 |
