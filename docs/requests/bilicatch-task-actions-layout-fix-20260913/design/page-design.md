# 页面设计：任务操作列响应式修复

- delivery unit：`bilicatch-task-actions-layout-fix-20260913`
- page objective：让任务行操作控件始终位于任务列表边界内，并保持完整可操作。
- target scenario：桌面应用侧栏存在时，在 800-1280px 窗口查看任意状态的任务。

## Source And Upstream Constraints

- `USER-DEFECT-01` 授权修复操作按钮越界。
- 既有 TaskRow DOM、36px 图标按钮、动作矩阵、菜单和任务状态语义保持不变。

## Product-Content Allowlist Mapping

- 操作控件、菜单与现有文案：既有 code fact，保持不变。
- 响应式重排：`USER-DEFECT-01` 的表现层实现。
- 不新增字段、文案、动作、状态、导航或反馈。

## User Journey And Task Success

用户进入任务列表、扫描任务信息并使用行尾操作。成功表现为操作控件完整可见、可聚焦和可点击；确认、取消、失败恢复等既有路径均不改变。

## Information Architecture And Content Strategy

保持文件、状态、进度、速度、剩余/大小、操作的既有阅读顺序。内容宽度不足时隐藏速度列的既有策略保留；再不足时进入两列卡片式行布局，让操作区位于右列而不是延续宽桌面网格。

## Layout Structure And Visual Hierarchy

- 宽内容区：保留六列任务表头和任务行。
- 中等内容区：保留五列并隐藏速度。
- 窄内容区：隐藏表头，任务行改为 `minmax(0, 1fr) auto`，文件/大小在左、状态/操作在右、进度跨列。
- 操作按钮仍为行级次操作；危险操作的既有样式与确认不变。

## Visual Design And Styling Direction

不改变颜色、边框、字体、间距节奏或按钮视觉。修复仅调整响应式布局触发依据和边界。仓库当前为语义 CSS；本缺陷沿用既有样式文件做最小修复，不引入第二套样式技术。

## Tailwind CSS-Style Styling Constraints

框架偏好 utility classes，但仓库既有任务模块由集中 CSS 管理且未配置 Tailwind。本次仅修改既有规则值，不新增 semantic class、scoped CSS、CSS module、预处理器、内联样式或隐藏 class 常量。

## Class Length And Structure Strategy

不新增 class 值或组件层级；保持现有类名和 Vue 模板。

## Interaction Skeleton And State Model

点击、键盘焦点、pending disabled、完成任务打开菜单、确认弹窗及 loading/empty/error 状态均保持原样。布局不得裁剪控件或依赖横向页面滚动。

## Responsive Behavior

断点必须以任务内容区的实际可用宽度为依据。当前页面有固定侧栏和内容 padding，窗口宽度不是任务列表宽度；因此窄布局应在任务内容区不足以容纳五列最小轨道时触发。验证覆盖 1280、900、800px 和断点邻近宽度。

## Accessibility And Keyboard Considerations

保留原生 button/summary、ARIA 标签、36x36 目标尺寸、焦点顺序和菜单操作。不得通过 `overflow: hidden` 隐藏按钮。

## Usability Heuristics And Risk Controls

- 风险：按窗口断点导致侧栏挤压后的内容区仍使用宽网格。缓解：使用容器查询或等效内容区感知布局。
- 风险：按钮换行导致行高/对齐不稳定。缓解：窄布局为操作列提供显式右侧轨道。
- 风险：只检查 viewport 会漏掉相对列表越界。缓解：验证按钮边界必须与 `.task-list` 边界比较。

## Component Reuse And Split Strategy

保留 `TasksPage`、`TaskRow`、`AppIconButton` 边界。无新组件；抽取会成为单调用方样式包装，不成立。

## Design Risks And Open UI Questions

- 风险：容器查询需由现代 WebView 支持；项目运行于当前 Edge/WebView2 和 Vite 浏览器目标，可直接验证。
- open UI questions：无。
