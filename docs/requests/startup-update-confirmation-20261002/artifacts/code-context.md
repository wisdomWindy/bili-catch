# 代码上下文

## 上下文要求

本功能跨根组件、设置 store、发布端口和新弹窗组件，需要确认现有依赖注入与副作用边界。该结构在 spec 阶段首次需要；代码图仅为偏好，不是必需。

## 图可用性检查

- 状态：missing。
- 检测方式：当前工具与仓库未暴露代码图命令或配置。
- 时间：2026-10-02，spec iteration 5。

## 安装或初始化记录

- attempted：no。
- 方法：不适用；仓库无团队指定的图工具或 bootstrap 脚本。
- result：not_needed。
- 原因：受影响入口和依赖均由直接 import/provide/inject 串联，`rg` 与文件读取能闭合范围。

## 回退记录

- fallback used：yes。
- 方法：`rg` 搜索 update/updater/autoCheckUpdates，读取根组件、设置页、release actions、store、injection、对话框和测试。
- 残余置信度：高；没有动态注册或不可见生成代码。真实更新网络仍需发布冒烟验证。

## 相关入口

- 用户入口：应用启动、设置页手动检查、托盘检查菜单。
- 内部入口：`App.vue` onMounted、`useSettingsStore.initialize`、`ReleaseActionsPort.checkForUpdates/installUpdate`。
- 异步触发：设置加载完成、updater check Promise、安装 Promise。

## 关键符号与模块

- `useSettingsStore`：可信设置状态和值。
- `ReleaseActionsPort` / `createTauriReleaseActions`：更新边界与待安装缓存。
- `App.vue`：根生命周期与全局 provider。
- `SettingsPage.vue`：邻接手动更新流程。
- `LogoutConfirmDialog.vue`：现有可访问模态参考。

## 依赖与副作用边界

- Tauri `check`、`downloadAndInstall`、`relaunch` 只存在于 `release.ts`。
- 根级桥接组件调用端口并管理自动检查状态。
- 展示组件不依赖设置 store 或 Tauri 插件。
- 设置页继续使用同一注入实例。

## 影响范围

- 预计新增两个 settings 组件及测试。
- 修改 `App.vue`、locale 和 settings 样式。
- 回归面：根初始化、设置开关、手动检查、托盘检查、模态焦点。

## 后续检查

- 执行阶段确认 Naive UI `NModal` 在测试环境的 teleport 与焦点行为。
- 发布冒烟确认 updater manifest 存在更高版本时可完整安装重启。

