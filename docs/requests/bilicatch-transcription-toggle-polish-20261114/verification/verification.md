# 验证：自动转写开关行视觉优化

## 命令证据

| 命令 | 结果 |
| --- | --- |
| `pnpm run typecheck` | 通过 |
| `pnpm run test` | 通过（58 files / 242 tests） |
| `pnpm run build` | 通过 |
| 无头 Edge 截图（dev demo） | 通过，见下方证据 |

## 视觉证据

- 关闭态：[options-off.png](docs/requests/bilicatch-transcription-toggle-polish-20261114/verification/evidence/options-off.png)
- 开启态：[options-on.png](docs/requests/bilicatch-transcription-toggle-polish-20261114/verification/evidence/options-on.png)

关闭态：胶囊开关、标题/说明两行、卡片边框与内边距。
开启态：轨道变品牌色、滑块右移、卡片边框与浅底带品牌色。

## 验收标准映射

| AC | 结论 | 证据 |
| --- | --- | --- |
| AC-1 容器样式 | 通过 | 截图中卡片具备边框、圆角、内边距与 16px 上间距 |
| AC-2 胶囊开关与状态 | 通过 | on/off 截图对比：轨道颜色与滑块位置同时变化；`:focus-visible` 规则存在 |
| AC-3 文案两行 | 通过 | 截图中标题与说明分行，说明为次要色 |
| AC-4 行为不变 | 通过 | 未改 Vue 模板与 store；242 个测试全过 |
| AC-5 命令通过 | 通过 | 见命令证据 |

## 未覆盖

- 深色主题视觉：样式全部使用 tokens 变量，但未截深色图。
- 键盘焦点环：规则已写，未做截图断言。
- 真实 Tauri WebView 渲染：截图来自 Edge（同 Chromium 内核）；安装包内为旧样式，需要重新打包后生效。

## 残留风险

- `:has()` 依赖 Chromium 105+；本项目 WebView2 满足，Edge 截图已验证。
