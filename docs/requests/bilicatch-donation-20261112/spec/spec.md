# 规格：设置页打赏功能

## Delivery Unit Identifier

- 请求：`bilicatch-donation-20261112`
- 类型：S1 local 直接变更，请求级交付单元
- 当前设计输入：`design/page-design.md`

## Background And Goals

用户要求添加打赏功能，并提供支付宝与微信收款码图片。目标是在现有设置页“关于”区域提供稳定入口，以本地弹层完整展示两张原图，不接入交易或支付服务。

## In Scope

- 在 `AboutSettings` 末尾增加一行“打赏”入口。
- 新增 `DonationDialog`，展示支付宝与微信两张本地收款码。
- 支持按钮打开、显式关闭、遮罩关闭、Esc 关闭和焦点恢复。
- 支持现有中英文与明暗主题；所有窗口使用利于扫码的单列渠道布局，窄窗口不横向溢出。
- 新增组件测试并适配现有关于区域/设置页测试。

## Out Of Scope

- 金额选择、订单、支付接口、支付状态、成功/失败确认、统计、持久化或 IPC。
- 修改二维码像素内容、裁切图片或生成新二维码。
- 新路由、全局导航、首页推广、设置页其他重构。

## Trigger And Start Conditions

- 前置：设置页已成功加载并显示“关于”区域。
- 触发：用户点击“打赏”按钮。
- 终止：用户通过关闭按钮、遮罩或 Esc 关闭弹层。

## Requirement Split Summary

不适用：这是稳定的直接变更请求，`module_flow=null`，跳过 PRD-only requirement-analysis / requirement-splitting。

## Source Grounding And Traceability

| Spec item | Label | Product-facing | Authority | Plan obligation |
| --- | --- | --- | --- | --- |
| 打赏入口和标题 | source-backed | 是 | `USER-DIRECT-01`“添加打赏功能” | 增加入口、打开弹层 |
| 支付宝收款码与标签 | source-backed | 是 | `USER-ASSET-ALIPAY-01` | 原图导入与完整展示 |
| 微信收款码与标签 | source-backed | 是 | `USER-ASSET-WECHAT-01` | 原图导入与完整展示 |
| 关闭弹层 | source-derived + code-fact-backed | 是 | 可用弹层的必要撤销路径；现有 `NModal` 模式 | 覆盖关闭与焦点恢复 |
| 入口位于关于区 | code-fact-backed constraint | 是 | `AboutSettings` 现有产品信息所有权 | 仅改关于区域，不扩导航 |
| 双语等义标签 | code-fact-backed constraint | 是 | 现有 `vue-i18n` 双语合同 | 同步 zh-CN/en-US |
| 不接入支付流程 | source-backed scope boundary | 是 | 用户只提供收款码，无支付接口要求 | 禁止新增异步/持久化逻辑 |

Closed-world product content allowlist：`打赏/Donate`、`支付宝/Alipay`、`微信支付/WeChat Pay`、`关闭/Close`、打开/关闭弹层、展示两张指定图片。其他字段、文案、控件、状态、默认值、验证、权限、导航、API、分析事件与反馈均禁止实现。

## Workflow Efficiency Profile

- level：`S1 local`。
- density：compact；保留所有硬门禁，非适用项简述。
- context：scoped，仅覆盖设置关于区、对话框基线、语言包、图片类型声明和相关测试。
- upgrade condition：若实现需要跨页面入口、共享 store、IPC、网络支付、全局样式系统或跨模块抽象，立即升级 S2 并回到 spec。

## Self-Healing Completion Bar

- 需求一致性：入口、两资产、关闭能力、双语/主题/响应式均与本规格一致；无范围扩张。
- 业务逻辑：只有 closed/open 两个本地 UI 状态，不出现支付状态或交易推断。
- 静态检查：组件测试、相关设置页测试、完整 `npm test`、`npm run build` 必须通过。
- 运行时安全：图片路径可由 Vite 构建解析；弹层无异步、竞态、订阅或清理泄漏。
- 代码组织：组件边界、imports、tests、locales、资产引用无孤儿或重复状态。
- 可维护性：命名清晰、事件表达用户意图、无 debug/dead/stale 内容、diff 可审查。
- 完成必须记录 `self-healing loop compliance: pass`、`self-healing loop assessment: pass`，remaining blockers 为 none。

## User Intent Contract

- literal request：`添加打赏功能，上传的图片为收款码`。
- practical goal：用户能方便找到并扫描指定收款码支持项目。
- success criteria：入口可发现，两张图片完整清晰，弹层能关闭并恢复焦点，桌面/窄窗口、明/暗主题可用。
- forbidden interpretations：二维码重制、金额/订单/支付状态、联网或持久化支付能力、营销扩写。
- acceptable approaches：关于区次级按钮 + 本地模态展示。
- verification/review：检查真实资产、DOM 语义、键盘路径、响应式截图、diff 范围与无支付副作用。

## User Flow

`设置页 -> 关于 -> 点击打赏 -> DonationDialog open -> 查看/扫码任一渠道 -> Close/Esc/遮罩 -> 返回触发按钮`。

## Page And Module Design

- `AboutSettings.vue` 持有 `isDonationDialogVisible: boolean`，默认 `false`。
- 打赏按钮设置状态为 `true`；`DonationDialog` 的 `close` intention 设置为 `false`。
- `DonationDialog.vue` 只负责 modal 生命周期和静态展示，不访问 store/service/router。
- 两张图片放在 `src/assets/donation/`，通过 ESM import 交给 Vite 处理，避免运行时路径猜测。

## UI / UX Design Contract

- 关于区新增第 4 行，使用 quiet command button 与 Lucide `Heart` 图标及可见“打赏”文字。
- 弹层使用 raw `NModal` 与设置模块现有对话框容器，标题为“打赏”，提供显式 close control。
- 收款码内容使用单列 `NGrid`：支付宝与微信依次显示，每个渠道包含标签与完整图片；短窗口交由 modal 外层纵向滚动。
- `NImage` 禁用 preview；图片 `object-fit=contain`，保持纵横比，不裁切。
- 不显示推荐顺序、说明、感谢、金额或支付结果。

## Frontend Styling Constraints

- 入口复用 `.setting-row`、`.command-button`、`.command-button--quiet`。
- 把 `settings.css` 现有 `.update-dialog` 基础容器类改名为 `.settings-dialog`，现有 style values 不变；`UpdateConfirmDialog.vue` 同步改用新类，`DonationDialog.vue` 作为第二个真实调用者复用它。
- 新弹层的内部间距、单列网格与文本仅使用 Naive UI 组件 props；不新增新的 style declaration、scoped CSS、CSS module 或 inline style object。
- 不为本局部功能引入 Tailwind 依赖；由于没有新增 authored styling，Tailwind utility 规则无新增表达面。
- class 值保持现有短类名，不隐藏到常量/computed/helper。

## Field Display Semantics And Interaction Visual-State Contracts

| Field/control | Meaning and source | Display | Empty/invalid | Interaction states | Evidence |
| --- | --- | --- | --- | --- | --- |
| 打赏入口 | 支持项目的入口；`USER-DIRECT-01` | quiet button，Heart + 文本 | 不适用 | 默认可见；hover/focus 由现有按钮类提供；无 disabled | AboutSettings component test + screenshot |
| 支付宝图片 | 支付宝收款渠道；`USER-ASSET-ALIPAY-01` | 原图完整 contain | 资产缺失应由 build/test 阻断；alt 保留渠道语义 | 非点击、preview disabled | asset assertion + screenshot |
| 微信图片 | 微信收款渠道；`USER-ASSET-WECHAT-01` | 原图完整 contain | 同上 | 非点击、preview disabled | asset assertion + screenshot |
| 关闭控件 | 退出查看并恢复焦点 | Naive UI 标准关闭控件，含可访问名称 | 不适用 | hover/focus/active 由组件提供；Esc/遮罩等价关闭 | component keyboard/focus test |

## Function-Complete Behavior Breakdown

### `AboutSettings`

- import `ref`、`Heart`、`DonationDialog`。
- 声明 `isDonationDialogVisible = ref(false)`。
- `handleOpenDonationDialog()` 设为 true；`handleCloseDonationDialog()` 设为 false。
- 新增第 4 个 `[data-about-row]`，按钮 `data-testid=\"open-donation\"`，aria label 与可见文字均来自 `settings.about.donation`。
- 在 section 内挂载 `<DonationDialog :show=... @close=... />`。
- 不改变现有 props/emits、更新与许可动作。

### `DonationDialog`

- props：`show: boolean`；emits：`close: []`。
- import 两张源图片为模块 URL，import `NModal`, `NGrid`, `NGridItem`, `NSpace`, `NText`, `NImage`（不能引入新库）。
- modal：绑定 `show`，启用 auto-focus、trap-focus、mask close、Esc close；关闭统一 emit `close`。
- 标题：`settings.about.donation`；渠道标签与 alt 分别来自 `settings.about.alipay` / `wechatPay` 与 `alipayQrAlt` / `wechatQrAlt`。
- 网格：固定 `cols=1`，gap 使用组件数值 props。
- 图片：`width=100%`、preview disabled、object fit contain；不得更改文件内容。
- footer：文字关闭按钮 `data-testid=\"close-donation\"`，作为初始安全焦点；关闭后焦点回到打开按钮。

### Internationalization

- zh-CN：`donation: 打赏`、`alipay: 支付宝`、`wechatPay: 微信支付`、`alipayQrAlt: 支付宝收款码`、`wechatQrAlt: 微信收款码`、`closeDonation: 关闭`。
- en-US：等义 `Donate`、`Alipay`、`WeChat Pay`、`Alipay payment QR code`、`WeChat Pay QR code`、`Close`。

## Expert Frontend Engineering Constraints

- state ownership：唯一 writable state 在 `AboutSettings`；`DonationDialog` 不复制 `show`。
- data lifecycle：两张 build-time asset URL -> presentational props/DOM；无 DTO、adapter、form 或 payload。
- async correctness：不适用，无请求/竞态/重试。
- interaction resilience：语义 button、focus trap、Esc/遮罩/显式关闭、focus restore、非颜色渠道标签。
- performance：仅在弹层打开时渲染两张约 146KB/182KB 的本地图片；无需缓存、watch、memoization 或懒加载抽象。
- evolution safety：新增内容局限于 settings feature；删除时可由组件、语言键、资产和测试的闭包完整移除。
- testability：父组件覆盖入口与状态，弹层组件覆盖内容、关闭与键盘/焦点。

## Frontend Architecture Design Routing Inputs

- `architecture-design_required: false`。
- 原因：不改变模块边界、共享状态、依赖方向、数据拓扑、公共 API 或跨文件业务规则；新增 SFC 是已在 page-design/spec 批准的交互生命周期边界。
- 若实现需要共享 store、跨页面入口、服务/adapter 或共享抽象，必须回到 spec 并改为 architecture-design required。

## Production Code Quality Constraints

- type-first：props/emits 使用精确 TypeScript 类型；静态资源 URL 由 `vite/client` 声明，禁止 `any`/assertion。
- fail-fast：缺失资源由 import/build 失败阻断；不吞掉异步错误，因为无异步逻辑。
- nullability：`show` 是必需 boolean；无 nullable product data 或 fallback。
- maintainability：直接 ref/event handler；禁止 computed/watch/cache/debounce/class。
- naming：boolean 以 `is` 开头，内部事件处理使用 `handle` 前缀。
- no magic：图片路径由组件内 ESM import 明确拥有；不抽出单用常量文件。
- boundary states：closed/open 均明确；loading/error/form states 在当前同步静态功能中不可发生。

## Human Review Readiness Constraints

- 遵循现有 `<script setup lang=ts>`、Lucide icon、Naive UI、vue-i18n 和 Vitest/Vue Test Utils 习惯。
- 允许改动：AboutSettings 组件/测试、一个 feature-local dialog 组件/测试、UpdateConfirmDialog 的容器类名、settings.css 对应类选择器、两份 locale、两张资产、请求工件。
- 明确不改：SettingsPage 行为、stores、services、release actions、router、IPC、全局导航和无关 CSS。
- reviewer risk：Naive modal teleport/focus 测试、图片原图完整性、现有测试对 about row 数量的硬编码。
- execute 自检：每个 hunk 映射到计划任务；无格式噪音、debug、unused import、stale expectation；记录命令和视觉证据。

## Functional-Programming Constraints

不适用：无业务转换、validation、mapper、payload 或 derived data。布尔 UI 状态用 Vue `ref` 明确更新，不引入函数式工具库或高阶抽象。

## Architecture Reuse And Shared Ownership Constraints

- 无重复业务规则、adapter、mapper、validation、状态映射或共享 helper 候选。
- 复用现有 Naive UI modal 基础设施与设置按钮样式，不抽取新的共享组件。
- `DonationDialog` 保持 feature-local，不能提升到 shared；其支付渠道语义不应泛化为通用图片 modal。

## Locality And File-Extraction Constraints

- `handleOpenDonationDialog` / `handleCloseDonationDialog`：单调用者，留在 `AboutSettings.vue`。
- 图片模块 URL：只由 DonationDialog 使用，留在组件 import，不建常量文件。
- `DonationDialog.vue`：允许独立文件，理由是 modal 焦点/关闭生命周期与 focused test boundary，不是复用或缩短父文件。
- `.settings-dialog`：由同一 settings feature 内两个真实 modal 调用，拥有共同表面/宽度机制；只泛化现有类名，不建立跨 feature design-system abstraction。

## Vue Component Extraction Constraints

- candidate：`DonationDialog`；decision：extract；caller count：1。
- single-caller exception：interaction lifecycle boundary + focused test boundary。
- state owner：`AboutSettings` 持有 visibility；dialog 不复制 writable state。
- props：`show: boolean`；emits：`close`；slots：none。
- normalization：none；静态资产/文案在 feature-local view 中直接引用。
- test surface：内容、两图 alt/src、preview disabled、关闭 emits、Esc/focus behavior。
- 现有关于行 keep inline，禁止抽成透传 row component。

## Design Constraints

- 一个入口、一个弹层、一个可见状态，保持单向数据流。
- 不增加共享 store、service、composable、manager、factory 或通用 donation abstraction。
- 依赖方向：`AboutSettings -> DonationDialog -> local assets/Naive UI/i18n`。

## Project Bootstrap And Scaffold Decision

不适用：现有应用内局部功能，不创建项目、package 或新 frontend surface。

## Change Axes And Pattern Decision

- concrete change：一个稳定的本地 open/close UI 行为和一个具名 modal component boundary。
- depth：Level 0 direct code（状态与事件）；组件边界属于 Vue 本地组合语法，不引入 named pattern layer。
- candidate signals：无 provider 选择、API adaptation、对象创建、命令队列、跨模块副作用协调、状态机扩散或共享规则。
- chosen：`direct code` / pattern name `none`；使用 `ref<boolean>`、明确事件处理器和 props/emits。
- lightest fit：只有两个 UI 状态和单一 caller；任何 store/composable/strategy/factory 都会增加无收益间接层。
- later signal：只有出现第二个同语义生产入口或共享 modal policy 时才重新评估复用；当前禁止预留扩展层。

## Code Context And Impact Assumptions

- `tsconfig.json` 以 `strict`, `moduleResolution=bundler`, `noUnused*` 约束 `src/**/*.vue/ts`。
- `src/vite-env.d.ts` 引用 `vite/client`，支持图片 ESM import。
- `AboutSettings` 仅由 `SettingsPage` 使用；现有测试预期 3 行，需要改为 4 行。
- `SettingsPage` 事件合同不变；新增功能完全在 AboutSettings 子树内。
- Naive UI 本地声明确认 `NGrid` cols、`NImage` width/previewDisabled/objectFit、raw modal 的 focus trap/scroll wrapper 可用；现有 `.update-dialog` 提供适合单列二维码的 420px 响应式宽度。

## API And Data Contracts

不适用：无 backend/API/DTO/payload。唯一数据是两个 build-time asset URL。

## Frontend / Server Responsibility Boundary

- frontend：入口、弹层、静态图片、i18n、可访问性和测试。
- server：无任务、无依赖。
- external interface：无。

## Context And Dependency Sources

- Vue 3 `<script setup>`、`vue-i18n`、`@lucide/vue`、`naive-ui`、Vite asset imports。
- 不新增 dependency，不修改 package 或 tsconfig。

## Edge Cases

- repeated click：visibility 已为 true，不创建重复 modal。
- narrow viewport：dialog 宽度为 viewport 减 32px，grid 仍为单列，图片 width 100%，不横向溢出。
- short viewport：modal content scrollable，关闭入口可达。
- dark theme：容器由 Naive theme 适配，图片保持原始白/彩色背景。
- asset failure：构建期路径错误应失败；运行时不新增无来源错误流程，alt 保底渠道识别。
- focus：关闭后回到打开按钮；组件卸载时不保留引用或监听器。

## Acceptance Criteria

- AC-01：设置“关于”区域有 4 行，新增按钮显示 Heart 图标和本地化“打赏/Donate”，现有 3 行行为不变。
- AC-02：点击入口后只出现一个 dialog，标题为打赏，包含支付宝和微信支付两个渠道标签及对应原始资产。
- AC-03：两张图片 `src` 均由本地构建资产产生，alt 正确、preview disabled、完整 contain 展示且无裁切。
- AC-04：关闭按钮、Esc 与遮罩均可关闭；打开时焦点进入 dialog，关闭后恢复到入口。
- AC-05：所有窗口单列展示；桌面和窄窗口无水平溢出，明暗主题下文字、按钮、关闭控件可辨识，二维码四角定位区完整可见且展示尺寸利于扫码。
- AC-06：不出现 API/IPC/store/persistence、金额、订单、支付结果或新增营销文案。
- AC-07：新增/适配组件测试先红后绿；相关测试、完整测试和生产构建通过。

## Human Review And Handoff

- 交付时提供变更文件摘要、测试/构建结果、桌面与窄窗口视觉证据、未改变范围。
- reviewer 可从 `AboutSettings -> DonationDialog -> assets` 一跳追踪完整功能链。

## Risks

- 泛化 `.update-dialog` 为 `.settings-dialog` 时必须确认启动更新弹层视觉与行为不变，避免共享基础容器改名产生回归。
- 源图会增加约 329KB 未压缩资产；二维码完整性优先，不做有损优化。
- jsdom 对 modal teleport/focus 的行为与浏览器可能有差异，需组件测试加 Playwright/浏览器视觉与键盘验证。
