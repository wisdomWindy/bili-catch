# 任务板

| ID | 名称 | 状态 | Readiness | 模式 | 文件/符号 | 来源/功能 | 关键约束 | 测试 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| T1 | 建立列表边界回归 | completed | pass | 串行 | `scripts/verify-task-visuals.cjs` | USER-DEFECT-01 / AC-01..03；product delta none | 真实页面、相对列表断言、预期 RED | RED 已观察，最终 PASS |
| T2 | 容器宽度布局修复 | completed | pass | 串行，依赖 T1 | `task-management.css` 的 list/query | USER-DEFECT-01 / AC-01..04；product delta none | Level 0 direct CSS；不改 DOM/动作/尺寸/技术栈 | visual、focused、full test、typecheck、build PASS |

## 全局执行约束

- speed profile：S1 local；context narrow。
- 变更链：TasksPage -> task-list -> TaskRow -> actions；只 owning CSS/视觉测试。
- 测试适配：视觉脚本扩充；既有 Vue tests 保持行为断言。
- 专家/质量：语义、焦点、ARIA、disabled、尺寸、状态 owner 不变；无副作用和性能监听。
- 评审预检：无无关 diff、debug、dead/stale 代码、临时命名、未使用引用；证据可复跑。
- 待确认项：无；纯技术假设是 Edge/WebView2 支持 CSS container query，执行时由真实浏览器验证。
