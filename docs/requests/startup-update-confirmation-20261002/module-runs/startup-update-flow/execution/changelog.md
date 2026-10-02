# 执行变更记录

## TDD 记录

### RED

- 新增 `src/app/StartupUpdateFlow.test.ts`，从根组件真实挂载验证启动检查、设置开关、静默分支、版本弹窗、安装锁定与失败重试。
- 首次聚焦运行：4 个测试中 2 个失败、2 个通过。
- 预期失败 1：自动检查 mock 调用次数为 0，证明启动触发尚未实现。
- 预期失败 2：安装 mock 调用次数为 0，证明确认弹窗与安装入口尚未实现。

### GREEN

- 新增 `StartupUpdateBridge.vue`，在设置 ready 且开关开启时检查一次，并管理 available/installing/error。
- 新增 `UpdateConfirmDialog.vue`，实现版本展示、安全焦点、pending 锁定与错误重试。
- 在 `App.vue` 的全局 provider 内挂载 bridge。
- 增加中英文文案与响应式 settings 样式。
- 聚焦测试 4/4 通过；相关 App/Settings/About 测试 13/13 通过；类型检查通过。

### REFACTOR

- 补充“稍后”关闭、焦点恢复和 pending Escape 不关闭断言。
- 移除不存在的阴影 token，只使用仓库现有设计变量。
- 保持 `ReleaseActionsPort`、设置 schema 与 IPC 不变；仅强化 Tauri adapter 内部的检查合并和待安装版本绑定。

## 结构与边界

- 协调副作用归 bridge，纯展示与可访问交互归 dialog。
- 根 App 只挂载入口，不持有更新状态机。
- 更新插件对象和私有错误不进入 UI。
- 未新增 store、事件总线、通用 modal manager 或依赖。

## 计划偏差

无产品或范围偏差。测试关闭断言根据 Naive UI `display-directive="show"` 的真实行为，从“DOM 移除”修正为“保留节点但 display:none”；用户可见语义不变。

## Review 回流

- 阻断 1：安装时两个按钮禁用，原手写 Tab 循环在无可用按钮时放行焦点；需要原生 focus trap 与无按钮兜底。
- 阻断 2：共享 release adapter 的可变 `pendingUpdate` 可被设置页/托盘的再次检查覆盖；需要让已发现的 update 在安装前保持绑定，并处理 install Promise 正常返回以避免永久 pending。
- 处理方式：回流 execute，先增加焦点和并发回归红灯，再实施最小修复并重新跑完整门禁。

### Review 回流 RED / GREEN

- RED：聚焦运行出现 2 个预期失败，分别证明 pending 状态的 Tab 未被阻止，以及第二个检查调用会返回并覆盖更新版本。
- GREEN：启用 Naive UI 原生 focus trap，并在无可用按钮时聚焦弹窗容器；并发检查共享同一 Promise，已发现的 `Update` 保持绑定直至安装。
- 补充卸载后迟到检查结果测试，以及安装 Promise 正常返回时关闭并解除 pending 的断言。
- 修复后聚焦测试 7/7 通过；独立复审确认两个 Important 均关闭，无新增阻断或重要问题。

## 最终验证

- 完整 `npm test`：49 个文件、195 个测试全部通过。
- `npm run typecheck`：通过。
- `npm run build`：通过；仅有既有的单 chunk 超过 500 kB 提示。
- `git diff --check`：通过。
- 桌面与 375×667 窄屏视觉检查：通过；文本、版本值和按钮无重叠或溢出。
- 安装中自动化状态：两个按钮均禁用，Tab 与 Escape 不会把交互带出或关闭弹窗。
- 浏览器控制台：无 warning/error。
