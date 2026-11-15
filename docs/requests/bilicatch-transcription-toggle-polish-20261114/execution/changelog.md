# 执行记录：自动转写开关行视觉优化

## 任务状态

| ID | 任务 | 状态 | 说明 |
| --- | --- | --- | --- |
| T1 | 补充开关行样式 | 完成 | 新增容器、开关、文案、状态样式，仅改 CSS |
| T2 | 视觉复核与回归 | 完成 | dev demo + 无头 Edge 截图；前端检查全绿 |

## 改动文件

- `src/features/download-center/download-center.css`（唯一生产文件改动）

未改：`DownloadOptions.vue`、文案、store、布局、其他控件。

## 实现要点

- 容器：flex、16px 上间距、12/14 内边距、1px 边框、8px 圆角、浅灰底。
- 开关：`appearance:none`，38×22 胶囊，16px 白色滑块，开启品牌色 + `translateX(16px)`。
- 状态：hover 边框加深；`:has(.switch-control:checked)` 品牌边框 + 6% 品牌浅底；`:focus-visible` 焦点环。
- 文案：标题 13/600，说明 12/1.45 次要色，两行网格排列。

## 执行中修正

- 初版滑块位移按 16px 计算，结合 `box-sizing:border-box` 与 1px 边框核对后仍为 16px，与轨道内宽（36px）匹配。

## 回滚

删除新增的 `.download-transcription-option*` 与 `.switch-control*` 规则块即可恢复原状。
