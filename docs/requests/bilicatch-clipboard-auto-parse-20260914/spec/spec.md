# 规格：剪贴板自动识别

## 范围与流程

下载页在挂载时检查一次系统剪贴板，并在页面所在窗口从失焦变为聚焦时再次检查。读取值先经既有 `normalizeParseInput` 归一和校验；成功则把归一值写入输入框并走既有 `store.parse`，失败则不修改任何用户可见状态。

普通键入不自动解析；输入框 paste 与拖拽行为保持不变。重复聚焦且剪贴板原始文本未变化时不重复解析；剪贴板内容先变为其他值再变回合法值时可再次解析。解析中的并发控制继续由 store 负责。

## 合同

- 新增 `ClipboardReader`：`readText(): Promise<string>`。
- 原生实现只调用官方 `@tauri-apps/plugin-clipboard-manager` 的 `readText`。
- demo/测试通过依赖注入提供受控实现，不直接访问原生 API。
- 权限最小化为 `clipboard-manager:allow-read-text`，不授予写入或图片权限。
- 读取异常被生命周期边界吞掉，不记录或展示剪贴板内容。

## 状态与异步

- 页面拥有 `lastClipboardText`，用于同内容去重；它不是 Pinia 业务状态。
- 每次开始读取时递增 revision；只有最新一次完成的读取可触发解析，避免异步旧值覆盖新值。
- 卸载时移除 focus 监听，已完成的旧读取不得触发后续解析。

## 架构与质量

- Pattern：Level 1，复用项目现有依赖注入模式，以窄 `ClipboardReader` 端口隔离原生插件。
- 校验复用 `normalizeParseInput`，不复制 Bilibili URL 规则。
- 页面保留工作流编排；插件 adapter 独立成文件，因为它是环境边界并需要在 demo/test 中替换。
- 不新增组件、样式、文案、轮询器、全局 store 或 manager。
- TypeScript 由根 `tsconfig.json` / Vue 类型检查约束；严格处理异步异常和卸载清理。

## 验收标准

1. 合法剪贴板值在挂载或重新聚焦后显示于 `#video-input`，解析服务收到归一后的同一值。
2. 无效文本、读取拒绝与重复聚焦不触发解析，也不覆盖现有输入。
3. paste、drop、手动提交测试继续通过。
4. 前端测试、类型检查、构建与 Rust 检查通过。

## Source Grounding

产品行为来自 `request.md`、`artifacts/prd-snapshot.md` 与 `requirements/requirement-map.md`；插件接口与权限来自 Tauri 官方 clipboard-manager v2 文档。无新增产品可见内容。
