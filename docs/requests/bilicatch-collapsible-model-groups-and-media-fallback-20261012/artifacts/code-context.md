# 代码上下文

| 链路 | Owner | 变更点 |
| --- | --- | --- |
| 模型分组渲染 | `src/features/transcription/components/ModelManager.vue` | 本地折叠状态、按钮与列表条件渲染 |
| 模型弹窗样式 | `src/features/transcription/transcription.css` | 标题按钮和箭头状态 |
| 模型组件测试 | `src/features/transcription/components/ModelManager.test.ts` | 多模型折叠、单模型静态展示 |
| 媒体候选下载 | `src-tauri/src/infrastructure/download/http_downloader.rs` | 地址级错误分类及顺序回退测试 |

复用现有 URL 循环，不引入新的重试策略层。两个功能链彼此独立但共享同一请求验收。
