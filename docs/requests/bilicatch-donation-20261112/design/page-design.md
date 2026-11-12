# 页面设计：设置页打赏入口与收款码弹层

## Delivery Unit Identifier

- 请求：`bilicatch-donation-20261112`
- 交付单元：非拆分直接变更，请求级交付单元

## Page Objective

在不改变现有设置页信息架构的前提下，让用户能从“关于”区域发现打赏入口，并在一个专注、可关闭的弹层中完整查看支付宝与微信收款码。

## Target Users Or Scenarios

- 希望支持 BiliCatch 项目的桌面端用户。
- 用户已进入设置页，浏览到“关于”区域后主动选择打赏。
- 用户可使用手机扫描桌面应用中展示的收款码。

## Source And Upstream Constraints

- `USER-DIRECT-01` 授权增加打赏功能。
- `USER-ASSET-ALIPAY-01` 与 `USER-ASSET-WECHAT-01` 是唯一允许展示的收款码资产。
- `artifacts/prd-snapshot.md` 限制范围为入口、两张图片与标准关闭能力，不得增加金额、交易状态、联网接口或成功反馈。
- `artifacts/code-context.md` 确认 `AboutSettings.vue` 是现有关于区域所有者，`NModal` 是仓库既有弹层能力。
- 两张图片必须保持完整内容和比例，不裁切、不重绘、不套用会遮挡二维码的叠层。

## Product-Content Allowlist Mapping

| 设计内容 | 依据 | 结论 |
| --- | --- | --- |
| “打赏”入口与弹层标题 | `USER-DIRECT-01` | source-backed，沿用用户原词 |
| 支付宝渠道标签与图片 | `USER-ASSET-ALIPAY-01` | source-backed |
| 微信渠道标签与图片 | `USER-ASSET-WECHAT-01` | source-backed |
| 打开与关闭弹层 | “打赏功能”的可用最小交互 + 现有 `NModal` 约束 | source-derived / code-fact-backed |
| 英文 `Donate`、`Alipay`、`WeChat Pay`、`Close` | 现有双语能力的等义兼容 | source-derived，不增加业务语义 |
| 金额、感谢语、支付结果、交易确认、网络错误 | 无来源 | out-of-scope，不进入设计 |

## User Journey And Task Success

1. 用户进入设置页并滚动到“关于”。
2. 用户识别带心形图标的“打赏”次级按钮并点击。
3. 弹层打开，标题首先说明当前任务；支付宝与微信支付以同级渠道展示。
4. 用户使用手机扫描任一完整收款码。
5. 用户点击关闭按钮、遮罩或按 Esc 退出，焦点回到“打赏”触发按钮。

- 主操作：打开打赏弹层。
- 次操作：关闭弹层。
- 危险操作：无。
- 禁用条件：无；资源随应用打包，本地静态展示。
- 完成信号：用户能完整看到并扫描所选渠道的收款码；应用不推断或显示支付结果。
- 取消/恢复：关闭弹层即取消查看；可再次打开，不保留额外状态。

## Information Architecture And Content Strategy

- “打赏”位于“关于”分区末尾，与版本、更新、开源许可组成产品信息与支持操作组。
- 弹层阅读顺序：标题 -> 两个同级支付渠道 -> 关闭。
- 两渠道不设置默认推荐或主次，避免新增用户未要求的支付偏好。
- 图片自身已有支付平台提示，因此界面只补充简短渠道标签，不增加指导或营销文案。

## Layout Structure

- 设置页：新增一行，左侧为“打赏”，中部为次级命令按钮，右侧保留设置行对齐占位。
- 弹层：使用现有 raw `NModal` 与设置模块对话框容器形成单一工具面板，不嵌套装饰卡片。
- 内容区：单列展示两个渠道。每张纵向原图获得接近弹层全宽的展示面积，优先保证二维码可扫描；短窗口由 modal 外层纵向滚动承接。
- 每个渠道：标签在上，完整收款码在下；图片使用固定上限尺寸且保持原始纵横比。
- 弹层内容超出可视高度时由模态容器滚动，不压缩或裁切二维码。

## Visual Hierarchy

- “打赏”入口使用现有 quiet command button，层级低于检查更新的主命令，不抢占设置页主任务。
- 心形图标与文字共同表达入口，不依赖颜色。
- 弹层标题为最高文字层级；渠道标签为次级层级；收款码是主要视觉内容。
- 背景、边框与暗色主题由 Naive UI 主题接管；不从收款码蓝/绿色扩展页面配色。

## Visual Design Direction

- 维持当前设置页安静、工具化的密度与 6-8px 圆角体系。
- 弹层为中性表面，避免渐变、装饰光效、嵌套卡片或多余说明。
- 收款码图片使用完整原图作为主要视觉资产，清晰度优先于装饰。
- 不添加动画；仅保留 `NModal` 的现有进入/退出过渡。

## Section Breakdown

- `AboutSettings`：新增打赏设置行、触发按钮、弹层可见状态与对话框挂载。
- `DonationDialog`：拥有弹层结构、关闭交互、焦点管理、响应式渠道布局与两张图片展示。

## Styling Direction

- 复用 `.setting-row`、`.command-button`、`.command-button--quiet`，不新增设置页入口样式。
- 弹层结构使用 Naive UI 的 `NModal`、`NGrid`、`NGridItem`、`NSpace`、`NText` 与 `NImage`，并把现有 `.update-dialog` 基础容器重命名为 feature-local `.settings-dialog`，由更新确认与打赏弹层共同复用；不增加新的样式声明。
- 图片尺寸通过 `NImage` 的组件输入约束；禁用图片预览，避免引入未授权的额外交互。

## Tailwind CSS-Style Styling Constraints

- 本仓库未安装或配置 Tailwind，现有生产样式由共享 CSS 与 Naive UI 主题共同提供；本请求不得为一个局部功能引入新样式技术栈。
- 本次不新增 authored style values、scoped CSS、CSS module、Sass/Less 或 inline style object；只把已有设置对话框容器类从 update-specific 名称泛化为 `.settings-dialog`，其声明保持不变并获得两个真实生产调用者。
- 入口继续复用现有短类；渠道内容使用组件 props，不为布局增加新的 CSS 声明。

## Class Length And Structure Strategy

- `AboutSettings` 仅复用短的既有类值。
- `DonationDialog` 仅使用短的 `.settings-dialog` 复用类；内部布局由具名 Naive UI 组件结构表达。
- 不使用常量、computed、helper 或导入值隐藏样式。

## Interaction Skeleton

- click：打赏按钮把本地 `showDonation` 设为 `true`。
- modal close：显式关闭按钮、遮罩点击或 Esc 触发 `close` intention。
- keyboard：打开后焦点进入弹层安全关闭按钮；Tab 留在弹层；关闭后恢复到触发按钮。
- image：不可点击放大，不提供 hover-only 行为。
- retry/loading/disabled：不适用；没有异步动作。

## State And Feedback Model

- closed：默认状态，仅显示打赏入口。
- open：显示两张本地收款码。
- loading/pending/success/payment failure：不适用；本功能不执行支付或异步请求。
- image load failure：不新增产品错误文案或重试操作；保留有意义的图片 alt 文本，使失败时仍能识别渠道，构建和视觉验证负责发现缺失资产。
- readonly/permission/validation/stale data：不适用。

## UX Writing And Feedback Copy Stance

- 只使用已授权的“打赏”、渠道名称和标准“关闭”标签。
- 不新增感谢、推荐、金额、扫码说明、成功或失败文案；图片中原有文字按原图保留。

## Responsive Behavior

- 所有宽度：两张图片单列展示，避免把二维码压缩到不利于手机扫描的尺寸。
- 弹层宽度继续使用现有 `min(420px, calc(100vw - 32px))` 容器约束，避免横向滚动。
- 图片宽度为容器可用宽度并保持比例；不得使用 `object-fit: cover`。
- 弹层最大可视高度不足时允许纵向滚动，关闭入口持续可达。

## Accessibility And Keyboard Considerations

- 触发器使用原生 `button`，图标 `aria-hidden`，可见文字给出名称。
- 弹层使用 `role=dialog`、`aria-modal=true` 与标题关联。
- `NModal` 启用 focus trap、Esc 与遮罩关闭；组件显式管理初始焦点和关闭后的焦点恢复。
- 两张图片使用渠道明确的 alt 文本；渠道含义不只依赖品牌颜色。
- 关闭操作使用文字按钮，触控目标沿用现有至少 36px 高度基线。

## Usability Heuristics And Risk Controls

- 风险：两张纵向原图导致弹层内容较高。缓解：单列完整展示并由 modal 外层纵向滚动，优先保持扫码尺寸。
- 风险：二维码被裁切或遮挡导致不可扫描。缓解：完整比例展示、禁用 cover、视觉验证原图四角定位点均可见。
- 风险：入口过度突出影响设置主任务。缓解：使用 quiet 次级按钮并置于关于区域末尾。
- 风险：误以为应用会确认支付。缓解：不显示交易状态、成功反馈或支付流程控件。

## Component Reuse And Split Strategy

### `DonationDialog`

- owning feature：`settings`。
- decision：extract。
- trigger：`interaction lifecycle boundary` + `focused test boundary`；组件独立拥有 modal 焦点、关闭、响应式图片布局和可访问语义。
- real production caller count：`1`。
- single-caller exception：明确的对话框交互生命周期，符合 `vue-component-extraction.md`。
- state owner：可见状态由 `AboutSettings` 持有；对话框内部仅持有焦点恢复引用。
- props：`show: boolean`。
- emits：`close`，表达用户关闭意图。
- slots：无。
- data normalization：无；静态资源路径与渠道语义由组件内受控常量/模板引用拥有。
- styling impact：复用现有设置 dialog 样式值并泛化类名；渠道布局使用 Naive UI 组件 props，不新增样式声明。
- verification surface：组件测试覆盖两张图片、标题/alt、关闭事件与键盘行为；父组件测试覆盖入口与开关。
- why better than inline：避免让已包含更新动作的 `AboutSettings` 同时承担 modal 焦点和复杂内容结构，且形成可聚焦的可访问性测试边界。

### 关于区域现有三行与新增入口

- decision：keep inline。
- reason：这些行由同一 `SettingSection` 所有，抽取只会形成单调用者透传组件并隐藏简单本地结构。

## Design Risks

- 原始图片分辨率较高，打包体积会增加；应保留二维码像素内容并通过构建产物检查体积，不做有损重编码。
- Naive UI `NImage` 的 DOM 包装与焦点行为需通过测试和浏览器验证确认预览确实禁用。
- 泛化后的设置 dialog 宽度必须在实际 800px 应用宽度和窄窗口截图中确认，且启动更新弹层不得出现回归。

## Open UI Questions

无阻塞问题。未授权的营销文案、推荐顺序、金额和支付反馈均明确排除。
