# 请求：设置页窗口缩小时无法滚动

- 请求标识：`bilicatch-settings-scroll-fix-20260914`
- 来源：用户于 2026-09-14 在当前对话中的直接缺陷报告
- 业务摘要：设置页内容超过窗口可视高度时，底部设置项被裁掉且无法滚动查看。
- 目标：恢复应用壳层的完整高度约束，使现有内容滚动容器在窗口高度不足时显示并支持纵向滚动。
- 初始完成信号：应用根 Provider 占满 `#app`；设置页内容高度超过窗口时 `.content-scroll` 形成可滚动溢出区域；现有页面结构和设置行为不变。
- 触发条件：打开设置页并缩小窗口高度，直至设置内容超过可视区域。
- 初始上下文：用户报告、`src/app/App.vue`、`src/styles/base.css`、`src/styles/layout.css`、Naive UI `ConfigProvider` 渲染实现。
- 人工交接点：实现、自动化验证和变更审查完成后交付。
- 受影响区域：应用根 Provider 与应用壳层高度链。
- 参与模块：`src/app`、`src/styles`；不涉及设置数据、IPC 或后端。

## 用户意图合同

- stated request：根据已定位的 Provider 高度链问题进行修改。
- practical goal：小窗口下可以滚动访问全部设置项。
- success criteria：根 Provider 具有明确的全高类；该类把可用高度传递给 `.app-shell`；`.content-scroll` 继续作为唯一主内容滚动容器。
- forbidden interpretations：启用 `body` 滚动、给设置页写固定高度、隐藏设置项或改动设置业务逻辑。
- acceptable approach：为 `NConfigProvider` 增加语义化根类，并在全局基础样式中补齐 `height: 100%` 与 `min-height: 0`。
