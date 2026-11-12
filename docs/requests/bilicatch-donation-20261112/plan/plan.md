# 执行计划：设置页打赏功能

## Delivery Unit And Goal

- delivery unit：`bilicatch-donation-20261112`
- practical goal：用户从设置“关于”区域打开本地弹层，清晰扫描用户提供的支付宝或微信收款码，并可靠关闭返回。
- source：spec AC-01 至 AC-07；closed-world allowlist 仅含打赏/渠道/关闭与两张指定资产。

## Scope And Change Chain

- pre-change chain：`SettingsPage -> AboutSettings -> version/update/license rows`；modal 基线为 `UpdateConfirmDialog -> NModal -> .update-dialog`。
- target chain：`SettingsPage -> AboutSettings -> open-donation -> DonationDialog -> ESM asset URLs`。
- state owner：`AboutSettings.isDonationDialogVisible`；dialog 只 emit `close`。
- adjacent unchanged boundaries：settings store、release actions、router、IPC、update/license events。
- post-change chain check：定向 `rg` 确认 `.update-dialog` 基础类无残留、`.settings-dialog` 仅被两个 settings dialogs 使用、资产和 locale keys 均有生产调用者。

## TypeScript Context

- governing config：根 `tsconfig.json`，`strict`, `moduleResolution=bundler`, `noUnusedLocals`, `noUnusedParameters`。
- declarations：`src/vite-env.d.ts` -> `vite/client` 负责图片 module URL；Naive UI 本地 `Grid.d.ts` / `Image.d.ts` / modal declarations 负责 props。
- implementation constraint：精确 `defineProps<{ show: boolean }>()`、`defineEmits<{ close: [] }>()`；禁止 `any`、类型断言和新环境声明。

## Task 1 - TDD Red: Lock Behavior And Assets

- grounding：source-backed `USER-DIRECT-01`, `USER-ASSET-*`; spec AC-01~AC-05/07。
- product-content delta：只增加 allowlist 中的“打赏/渠道/关闭”和指定资产。
- files：
  - add `src/features/settings/components/DonationDialog.test.ts`
  - edit `src/features/settings/components/AboutSettings.test.ts`
  - edit `src/pages/SettingsPage.test.ts`
- exact assertions：
  - AboutSettings 行数从 3 变 4，`open-donation` 是 button，中文 aria/name 为“打赏”；点击后 dialog 出现，关闭后消失。
  - DonationDialog 在 `show=true` 时有单一 `role=dialog`、标题、支付宝/微信标签、两张 alt；图片 src 分别包含构建资产名且无 preview interaction。
  - 显式关闭与 Escape emit `close`；打开后关闭按钮获得焦点，关闭后恢复外部触发器（jsdom 若 transition 限制则以事件/浏览器验证补齐并记录）。
  - SettingsPage about row 期望更新为 4，现有 update/license 行为仍通过。
- red proof：在新增生产组件/入口前运行 scoped Vitest，预期因模块/按钮/行数缺失失败；记录失败是预期行为。
- rollback：若测试需要猜测未授权文案或行为，停止并回 spec，不以测试发明需求。

## Task 2 - Assets And Localized Contract

- grounding：source-backed assets/labels；spec Internationalization/AC-02/03。
- product-content delta：两张原图与等义双语键。
- files：
  - add `src/assets/donation/alipay-donation-qr.jpg`（从用户附件逐字节复制）
  - add `src/assets/donation/wechat-donation-qr.png`（从用户附件逐字节复制）
  - edit `src/locales/zh-CN.ts`
  - edit `src/locales/en-US.ts`
- steps：
  - `Copy-Item -LiteralPath` 复制二进制，不重编码、不裁切。
  - 复制前后计算 SHA-256，必须一致。
  - 在 `settings.about` 增加 `donation`, `alipay`, `wechatPay`, `alipayQrAlt`, `wechatQrAlt`, `closeDonation`，不增加感谢/推荐/金额文案。
- verification：hash、文件大小、locale tests/typecheck/build。

## Task 3 - Implement Dialog Lifecycle And About Entry

- grounding：spec Function-Complete Behavior / AC-01~06。
- product-content delta：allowlist 内的入口、dialog 与关闭交互。
- files/symbols：
  - add `src/features/settings/components/DonationDialog.vue`
  - edit `src/features/settings/components/AboutSettings.vue`
  - edit `src/features/settings/components/UpdateConfirmDialog.vue`
  - edit `src/features/settings/settings.css`
- steps：
  1. 先把 CSS 基础选择器 `.update-dialog` 改名为 `.settings-dialog`，只改基础/通用 heading selectors；保留 `.update-dialog__*` 更新专用子类。
  2. `UpdateConfirmDialog` 根 class 同步改为 `settings-dialog`，不得改变 props、emits、焦点或 update UI。
  3. 新建 DonationDialog：精确 props/emits、两张 ESM imports、`NModal` raw mode、`.settings-dialog` root、aria-labelledby/aria-modal、单列 `NGrid`、两个 `NSpace vertical` 渠道、`NText strong` 标签、`NImage width=100% preview-disabled object-fit=contain`。
  4. 关闭按钮使用现有 `secondary-button`、`data-testid=close-donation`；沿用 UpdateConfirmDialog 的 active element capture、两次 nextTick 后聚焦、after-leave restore、unmount restore 模式；Escape/遮罩/update:show 统一 `emit('close')`，避免额外状态。
  5. AboutSettings import `ref`, `Heart`, `DonationDialog`；增加 `isDonationDialogVisible`、`handleOpenDonationDialog`, `handleCloseDonationDialog`；新增第 4 行和 `open-donation` quiet button；挂载 dialog。
- forbidden interpretations：store/composable/service/API/IPC、支付结果、图片预览、金额/感谢文案、独立路由、inline style、新 style values。
- cleanup closure：无 orphan `.update-dialog` base selector；无 unused imports/keys/assets；AboutSettings 现有 emits 不变。

## Task 4 - Green, Refactor, And Chain Validation

- run scoped：`npm test -- DonationDialog.test.ts AboutSettings.test.ts SettingsPage.test.ts`（按 Vitest CLI 实际接受形式调整但不扩大语义）。
- fix only spec-covered failures；保持 direct `ref`/handlers，禁止新增 abstraction。
- run locale test；检查中文/英文 key shape 一致。
- `rg` chain：`open-donation|DonationDialog|settings-dialog|update-dialog|alipay-donation|wechat-donation`。
- inspect `git diff --check`, `git diff --stat`, scoped diff；清除 debug/dead/stale/format churn。

## Task 5 - Full Verification And Visual Evidence

- automated：`npm test`、`npm run build`。
- browser/dev：启动 Vite demo 模式，访问 settings，至少验证：
  - 1280x900 light zh-CN：入口、弹层、两图、scroll/close。
  - 800x900 dark zh-CN：窄窗口无水平溢出、二维码完整。
  - 800x900 light en-US：英文标签不溢出。
- interaction：click open、close button、Esc、mask、focus restoration。
- visual evidence：保存截图到 `verification/evidence/`；检查画布非空、图片加载、二维码四角定位区可见、无 UI 重叠。
- regression：启动更新弹层的 `.settings-dialog` 改名未改变尺寸/视觉/焦点；相关现有测试通过。

## Test Strategy And TDD Contract

- red -> green -> refactor：Task 1 必须在 Task 3 前产生预期失败；Task 3 后 scoped green；Task 4 做最小整理；Task 5 全量验证。
- deterministic units：component tests 覆盖 DOM、emits、状态；真实浏览器覆盖 teleport/transition/focus/响应式/主题。
- no fallback-by-assertion：不能通过弱化行数、删除焦点或图片断言使测试变绿。

## Pattern And Architecture Contract

- decision：Level 0 direct code；Vue component boundary 是 framework-native lifecycle boundary，不引入 named pattern。
- no new store/composable/factory/manager/helper/constants file。
- `.settings-dialog` 是 settings feature 内两个真实 modal 的共享基础表面；不导出到 shared，不再抽象。
- `DonationDialog` 单 caller extraction 仅由 interaction lifecycle + focused test exception 支撑。

## Code Review Checklist Contract

- robustness：closed/open 状态唯一；Esc/mask/button 关闭等价；focus restore；资源路径由 build 校验；无 silent async failure。
- maintainability：精确命名与 props/emits；图片/handlers 就近；无 magic/any/assertion；无错误抽象。
- performance/memory：无监听器、timer、request、cache；unmount 恢复焦点引用；两张原图保持约 329KB 总量，不复制更多版本。
- technology stack：仅 Vue 3、Naive UI、Lucide、vue-i18n、Vitest、Vite assets；不新增依赖/配置/样式技术栈。
- review readiness：每个 hunk 映射 Task 1-5；无无关格式化；命令、截图与残余风险记录齐全。

## Execute-Readiness Check

- behavior/fields/states/files/symbols/data contracts/cleanup/tests 均已明确。
- 无 blocking clarification，`spec_approved=true`，`architecture_design_required=false`。
- execute 不需要猜测文案、布局、状态、错误处理、样式或验证目标。
- plan verdict：framework automatic approval `pass`。

## Rollback Conditions

- Naive UI props 或 `.settings-dialog` 复用无法满足无溢出/扫码尺寸：回 spec/page-design。
- 需要新产品文案、支付状态、全局入口或 API：回 intake 前置确认。
- 出现共享状态/跨模块依赖/新 abstraction：回 spec 并路由 architecture-design。

## Expected Verification Verdicts

- verification：spec/source/design-pattern/user-intent/change-chain/production-quality/human-review/expert-frontend/field-semantics/frontend-styling/TypeScript/self-healing compliance 均 pass。
- review：对应 assessment、clean-code、code-review checklist 与 self-healing assessment 均 pass。
