# 规格：统一选集界面（episode-selection-ui）

## 交付单元

`episode-selection-ui`（4 / 4）。上游：`design/page-design.md`、模块 1–3 契约、H2/H3/H4。

## 背景与目标

当前 `PartSelector.vue` 把 `parts` 当作平铺的 P 列表，标签固定 `P{page}`，不区分来源、不展示不可下载原因，也不支持集级联动。本模块把它改为“来源感知的统一选集列表”：番剧/合集用 `EP` 标签，多 P 集出现集头与子行，不可下载单元置灰并说明，全选只作用于可下载叶子。

## 范围内

1. `PartSelector` 视图模型（集头/单元行）与渲染。
2. 新增 store 动作：集头联动。
3. 标签、计数、半选态、禁用与原因。
4. `VideoSummary`：隐藏空 BV、时长为 0 的时长行。
5. 文案键（不可下载原因）。
6. 样式沿用 `download-center.css`。

## 范围外

- 新筛选/排序/搜索、整季一键、弹窗/抽屉/新路由。
- 修改下载设置区、任务页、解析逻辑。
- 引入 Tailwind 或新样式体系（项目无该依赖）。

## 来源锚定

| 项 | 标签 | 来源 |
| --- | --- | --- |
| 统一选集列表 | confirmed-decision（H2） | analysis H2 |
| 集为容器、P 为单元、展开到 P | confirmed-decision（H4） | analysis H4 |
| 多选 + 全选/反选 + 已选计数 | source-backed（既有规格）+ H3 | 既有页面规格、H3 |
| 不可下载置灰并说明 | confirmed-decision（H1）+ source-derived | H1、P5 |
| `EP` / `P` 标签区分 | confirmed-decision（H2） | analysis H2 |
| 集头联动全部子单元 | missing-source → 默认口径 Q9 | analysis Q9 |
| 空能力提示 | source-derived（模块 3 空能力状态） | 模块 3 |
| 隐藏空 BV / 0 时长 | source-derived（可空契约） | 模块 1、模块 2 |

## 行为拆解

### 视图模型

- 分组键：`groupIndex`。同组内按 `page` 升序；组间按 `groupIndex` 升序。
- 组 = `partCount > 1` 且 `source/container` 为合集或季 → 渲染集头 `EP{groupIndex}` + 子行 `P{page}`。
- 其余（含番剧、单 P 合集成员、普通视频）→ 平铺单元行。
- 单元行标签：
  - `container.kind === "video"` → `P{page}`
  - 否则 `EP{groupIndex}`；若该单元属于多 P 组则为 `P{page}` 子行。
- 标题空 → 回退标签本身。

### 计数与选择

- `m` = 可下载（`availability === "playable"`）叶子单元数。
- `n` = 选中集合中仍存在的叶子数；不可下载单元永不进入选中。
- 全选 = 选中全部可下载叶子；反选 = 可下载叶子减去当前选中。
- 集头点击：若组内可下载子单元全部已选 → 取消；否则补齐全选。集头不得选中不可下载子单元。
- 集头半选态：组内部分选中时 `indeterminate`；集头自身不可下载（组内无可下载子单元）时禁用。

### 不可下载

- 行：禁用复选框 + 行尾原因文本；`aria-describedby` 指向原因。
- 原因映射：`login-required` 需登录、`vip-required` 需大会员、`paid` 需付费、`region-locked` 地区限制、`download-disabled` 不可下载、`preview` 仅试看、`unknown` 暂不支持。
- 不可下载行不参与计数与全选/反选。

### 摘要区

- `result.bvid === null` → 不渲染 BV 行。
- `result.durationSeconds === 0` → 不渲染时长行。
- 分 P 计数保持现有（总单元数）。

### 空能力

- 下载设置区沿用既有“暂无可用选项”渲染；入队按钮保持禁用。选集仍可浏览。

## 组件与就近决策

- 保留 `PartSelector.vue` 为唯一 owner，不拆子组件：单调用方、行结构简单、拆分会引入不必要的 props 透传。
- 视图模型用组件内 `computed` 纯函数生成，不新增文件、不新增 store 派生状态。
- 只有“集头联动”需要写状态，落在 store 新 action `toggleGroup(cids)`。

## 样式约束

项目使用 `download-center.css` 的语义类（无 Tailwind 依赖），沿用该体系；新增类仅用于缩进与集头强调，禁止内联样式与新的全局类。

## 可访问性

原生 checkbox；集头半选通过 `el.indeterminate` 设置并提供 `aria-checked="mixed"`；禁用行 `disabled` + 原因 id；焦点顺序与 DOM 顺序一致。

## 验收标准

- AC-1：番剧单元显示 `EP{n}`；合集成员单 P 显示 `EP{n}`；普通视频/多 P 显示 `P{page}`。
- AC-2：多 P 合集成员渲染集头 + 子行；普通多 P 视频不渲染集头。
- AC-3：`n/m` 只统计可下载叶子；全选/反选跳过不可下载单元。
- AC-4：不可下载行禁用并显示对应原因；不进入选中。
- AC-5：集头勾选联动全部可下载子单元；部分选中呈半选；全不可下载时集头禁用。
- AC-6：`bvid=null` 隐藏 BV 行；时长 0 隐藏时长行。
- AC-7：空能力时显示“暂无可用选项”，入队禁用。
- AC-8：普通单视频/多 P 的解析、选中、入队行为不回归。
- AC-9：`pnpm typecheck`、`pnpm test`、`pnpm build` 通过。

## 风险

| 风险 | 处理 |
| --- | --- |
| 既有测试按平铺行数断言 | 同步更新测试为视图模型断言 |
| 半选态浏览器差异 | 用属性设置 + 显式 `aria-checked` |
| 大量单元渲染 | 沿用固定高度滚动，不做虚拟列表 |
