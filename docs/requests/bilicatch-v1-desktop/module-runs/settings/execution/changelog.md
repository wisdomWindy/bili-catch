# 执行记录：设置管理

## 审批与执行约束

- 规格批准：2026-09-11，用户回复“批准规格”。
- 计划批准：2026-09-11，用户回复“批准计划”。
- 执行顺序：SET-01 -> SET-02 -> SET-03 -> SET-04 -> SET-05 -> SET-06 -> SET-07。
- 工作区：当前目录不是 Git 仓库，无法创建隔离 worktree；按批准计划原地执行，不伪造 commit 检查点，不覆盖用户既有文件。
- 方法：所有可测试行为遵循 red -> green -> refactor；每项完成后记录定向门禁。

## SET-01：跨端合同、类型闭包与最小依赖

- 状态：in_progress。
- 开始：2026-09-11。
- 前置：计划已批准；governing TypeScript 配置和 Rust composition 已确认。
- 基线：前端 29 files / 120 tests PASS，TypeScript PASS；Rust 47 unit + 28 integration PASS，1 个公开网络 smoke ignored，仅有既有 Windows linker message。
- RED：新增 TS/Rust exact contract 测试，尚未添加生产类型。
- GREEN：新增共享 media contract、settings TS/Rust DTO 与选项；download-center 保持 `AudioFormat` re-export 兼容。
- 依赖：安装 `@tauri-apps/plugin-dialog` 2.x、`tauri-plugin-dialog` 2.7.3、`tauri-plugin-store` 2.4.4；未安装 JS store/updater。
- 门禁：settings/download contract 11 tests PASS，Rust settings contract 2 tests PASS，typecheck 与 Cargo check PASS。
- 状态：completed。

## SET-02：Rust 设置迁移、校验、事务存储与 Commands

- 状态：in_progress。
- 开始：2026-09-11。
- RED：`settings_manager` 首次编译因 `services::settings` 缺失而失败；plugin cache 故障注入首次编译因 transaction adapter 缺失而失败。
- GREEN：实现运行时默认目录、V1 完整文档迁移、字段级回退、数值/路径校验、candidate transaction、单调 revision 与 no-op 去写。
- 存储：`settings.json` / `document`，禁用 auto-save；显式保存失败时恢复 plugin cache 旧值，首次保存失败时删除未落盘候选值。
- 集成：注册 dialog/store plugins、`SettingsManager` state、`get_settings_snapshot` 和 `update_setting`；整体损坏/未来 schema 不覆写原数据。
- 门禁：SET-02 contract/manager 8 tests PASS，plugin cache fault 2 tests PASS；Rust 全量 49 unit PASS + 30 integration PASS，1 network smoke ignored；`cargo check` 和 `cargo fmt --check` PASS。
- 状态：completed。

## SET-03：任务调度与下载中心设置消费

- 状态：in_progress。
- 开始：2026-09-11。
- RED：Rust 因 `SchedulerLimits/TaskSettingsPort` 缺失而编译失败；download store 3 cases 因 `configureDefaults` 缺失而失败。
- GREEN：移除 task manager 内部可写 scheduler config/setter，每次 claim 读取注入的窄 `TaskSettingsPort`；生产直接复用 `SettingsManager`。
- 下载默认：新增窄 `DownloadDefaults`，目录只在仍跟随旧默认时更新，媒体偏好只影响下次解析/模式切换，不覆写当前选择；不可用能力回退首个免登录项。
- 回归：Rust settings/task 20 tests PASS，download-center 25 tests PASS，TypeScript、Cargo check 与 fmt PASS。
- 状态：completed。

## SET-04：前端 Service、Store 与效果协调

- 状态：in_progress。
- 开始：2026-09-11。
- RED：service/store/effect sink suites 因三个生产模块缺失而失败。
- GREEN：实现 exact IPC service、setup-style Pinia store、根级 effect sink；worker、desired patch、generation 和 timer 不进入 Pinia state。
- 并发：同字段 A-B-C 合并到最新意图，不同字段并行，旧 revision/旧失败/旧 timer 不覆盖新状态。
- 效果：主题/语言乐观应用且当前失败回滚；下载默认只消费 confirmed snapshot；load error 应用 `system/zh-CN` 安全外观。
- 门禁：6 files / 29 tests PASS，TypeScript PASS。
- 状态：completed。

## SET-05：目录、Release Port、Demo 与根 Composition

- 状态：in_progress。
- 开始：2026-09-11。
- RED：4 suites 因 dialog/release/demo/injection 模块缺失而失败；根组件用例随后暴露 jsdom `matchMedia` 环境缺失并以测试 stub 修正。
- GREEN：dialog 只调用 single-directory exact options，数组/插件异常统一为本地稳定错误；production release 只返回 deferred details。
- Demo：hash 可确定 load/save error、picker select/cancel、update latest/available/error，设置只在内存更新。
- Composition：`main.ts` 唯一组装 production/demo ports 和 effect sink；`App.vue` 无论路由根级 hydrate，卸载时清理 lifecycle/timers。
- 权限：WebView 仅新增 `dialog:allow-open`，无 store/updater/shell 权限。
- 门禁：17 files / 61 related tests PASS，12 lifecycle/fallback tests PASS，TypeScript、production build 与 Cargo check PASS。
- 状态：completed。

## SET-06：四组设置页面与无障碍

- 状态：in_progress。
- 开始：2026-09-11。
- RED：3 component imports 缺失，SettingsPage 仍为 EmptyState，3 page cases 均失败。
- GREEN：新增 SettingSection/Row/Status、Path/Range、Download/Appearance/System/About 组件，替换 `/settings` 占位页。
- 交互：11 个字段即时保存，slider input preview/change commit，picker cancel no-op/选择保存/焦点返回，About 在页内展示结果。
- A11y：11 个 label/control 关联，2 个真实 switch，range min/max/step/value text，icon command 具有 tooltip/aria-label，行状态 `aria-live=polite` 且不暴露 raw error。
- 样式：880px 连续无卡片表单，三列在 `<760px` 转单列，固定状态宽度和 36px 控件，无渐变/装饰元素。
- 双语：补齐 zh-CN/en-US settings tree，locale parity PASS。
- 门禁：UI targeted 13 tests PASS，full frontend 43 files / 155 tests PASS，TypeScript 与 production build PASS。
- 状态：completed。

## SET-07：全量回归、视觉、安全与验收证据

- 状态：completed。
- 开始：2026-09-11。
- 增量：为 demo 增加 `settings=loading` 可稳定观察骨架态；修正 Path/Range 在无说明文案时引用不存在 ARIA description 的问题，并补回归测试。
- 前端门禁：43 files / 156 tests PASS；TypeScript 与 production build PASS。Vite 仅保留 505.63 kB 主 chunk 非阻断警告。
- Rust 门禁：fmt/check PASS；49 unit + 36 integration PASS，1 个需显式联网的 smoke ignored。
- 安全边界：Tauri 直接引用仅位于 dialog adapter；无旧 scheduler setter、console/println/dbg；capability 仅新增 `dialog:allow-open`，无 WebView store/updater/shell。
- 视觉/交互：真实应用内浏览器确认窄屏单列无溢出/重叠、四组与 14 项完整；light 和 en-US 即时生效并显示 Saved；AX 树确认 labels、switch、range 与 live status。
- 受控限制：外部宽屏浏览器检测到用户输入后停止自动化；1280/900/800 采用 CSS 断点、页面/组件测试和 build 证据，不伪造截图。真实 updater/license 仍按 spec deferred 至 system-release。
- 验收：SET-AC-01..20 全部 PASS；原始命令、静态扫描与视觉记录见 `verification/evidence/commands.md`。

## 默认目录调整

- 默认下载目录改为安装目录下的 BiliCatch 子目录。
- 默认临时目录改为安装目录下的 Temp 子目录。
- 安装启动时创建两个目录；已有持久化且有效的用户路径保持不变，不会被默认值覆盖。
- 组合根复用 Tauri resource_dir 的父目录解析安装目录。
- 验证：新增安装目录默认路径测试，设置管理器 7 tests PASS；Rust 全量 73 passed / 1 ignored。

## 默认路径命名澄清

- 默认下载目录为用户选择的安装目录下 `download` 子目录。
- 默认临时目录为用户选择的安装目录下 `temp` 子目录。
- 已存在且有效的用户自定义目录继续保留，不会被默认值覆盖。
