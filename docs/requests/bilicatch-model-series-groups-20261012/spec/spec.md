# 工程规格：模型管理按系列分组

## 范围与意图

- 用户意图：在模型管理弹窗内快速识别模型系列，同系列模型集中展示。`SRC-01`
- 范围：`ModelManager` 的派生数据、渲染结构、局部样式、中英文系列标题和组件测试。
- 非范围：后端模型合同、下载流程、模型顺序、筛选、折叠、统计和新操作。

## 源映射与产品内容白名单

- 系列分组及系列标题：`source-backed` -> `SRC-01`。
- 模型 ID 到系列的映射：`code-fact-backed` -> `ASR_MODEL_IDS` 与官方模型清单。
- 已有操作和状态的保留：`code-fact-backed` -> 现有 `ModelManager.vue`。

## 行为与显示合同

1. 系列类型仅为 `whisper` 和 `fireRedAsr2`。
2. `tiny`、`base`、`small`、`medium`、`large-v3` 归入 Whisper；`fire-red-asr2` 归入 FireRedASR2。
3. 分组顺序固定为 Whisper、FireRedASR2；每组内保留 `models` props 的相对顺序。
4. 仅渲染包含模型的分组。
5. 每个分组使用本地化标题，并以 `section[aria-labelledby]` 建立可访问关联。
6. 模型行的名称、推荐文案、大小、默认标记、错误、进度和操作行为不变。

## 类型、状态与边界

- 复用 `AsrModel` 和 `AsrModelId`，新增组件内 `ModelSeries` 联合类型。
- ID 映射为 `Record<AsrModelId, ModelSeries>`，对当前和未来模型 ID 进行穷尽检查。
- `groupedModels` 是只读 `computed`，不修改 props，不引入新的可写状态或副作用。
- 空 `models` 输入生成空分组列表，不渲染系列标题。

## 模式、局部性与样式决策

- Pattern depth：Level 1 局部轻量数据映射。变化轴是“模型 ID 属于哪个系列”，typed record 比模板内条件或基于名称的字符串判断更稳定。
- 不引入 strategy/factory 或共享 helper：只有一个真实生产调用者，无副作用协调或跨模块复用。
- 不拆分 Vue SFC：新区块是单调用者的简单渲染结构，拆分只会增加 props/emits 透传。
- 样式锭定现有 `transcription.css` 和 CSS 令牌；不引入 Tailwind 或新样式技术栈。

## TypeScript 与质量约束

- Governing config：根 `tsconfig.json`，`strict`、`noUnusedLocals`、`noUnusedParameters` 和 bundler module resolution 生效。
- 类型来源：`src/features/transcription/contracts.ts`；不扫描或新增全局声明。
- 性能：模型数量很小，使用单个 `computed` 线性分组；不需要 cache、virtualization 或 watcher。
- 人工评审范围限于五个目标文件和请求工件，不进行无关格式化或重构。

## 验收标准

- `AC-01`：同时给定 Base、Tiny 和 FireRedASR2 时，显示且仅显示 Whisper 与 FireRedASR2 两个系列标题。
- `AC-02`：Base/Tiny 属于 Whisper section，FireRedASR2 属于 FireRedASR2 section，且组内顺序与 props 一致。
- `AC-03`：仅给定 Whisper 模型时不显示 FireRedASR2 空分组。
- `AC-04`：下载事件仍携带被点击模型的 `AsrModelId`；现有状态测试继续通过。
- `AC-05`：中英文标题存在，section 存在可访问标签关联。
- `AC-06`：聚焦测试、全量前端测试、类型检查和生产构建通过，弹窗无溢出或重叠。

## 风险

- 模型系列是前端展示知识；未来添加 ID 时由穷尽类型检查强制更新映射。
- 弹窗高度增加少量标题空间；使用紧凑间距并在实际 WebView 中检查。

