# 任务板：登录与认证

## 执行规则

- 固定顺序：AUTH-01 -> AUTH-02 -> AUTH-03 -> AUTH-04 -> AUTH-05 -> AUTH-06 -> AUTH-07 -> AUTH-08。
- 状态域：`pending | in_progress | completed | blocked`；同一时间仅一个任务可为 `in_progress`。
- 每项必须完成可信 red、最小 green、定向门禁、执行记录后才可 completed。
- 单 agent 串行；不启用 workflow/subagent。目录非 Git 仓库，不创建虚假 commit 检查点。

## 总览

| ID | 名称 | 状态 | 模式 | 前置 |
| --- | --- | --- | --- | --- |
| AUTH-01 | 公开合同、上下文闭包与依赖 | completed | 串行 | 计划批准 |
| AUTH-02 | B 站二维码与会话 Adapter | completed | 串行 | AUTH-01 |
| AUTH-03 | 系统凭据库 Port/Adapter | completed | 串行 | AUTH-02 |
| AUTH-04 | AuthManager 状态机与轮询事务 | completed | 串行 | AUTH-03 |
| AUTH-05 | Parser 认证上下文与缓存隔离 | completed | 串行 | AUTH-04 |
| AUTH-06 | Tauri 入口与根级 Auth Store | completed | 串行 | AUTH-05 |
| AUTH-07 | LoginPage、QR、TopBar 与退出 | completed | 串行 | AUTH-06 |
| AUTH-08 | 集成、安全、视觉与验收证据 | completed | 串行 | AUTH-07 |
| AUTH-R01 | Review 阻断项修复与回归 | completed | 串行 | AUTH-08/verify |

## AUTH-01

- 任务名称：跨端公开合同、上下文闭包与依赖
- 状态：completed
- 执行模式/说明：串行；contract-first TDD，依赖只做最小增量。
- 触发/前置：用户批准计划。
- 规格映射：AC-AUTH-06/11/15/16；API 与数据合同、脚手架、代码上下文。
- 功能单元：Rust serde DTO、TS contracts、dependency manifests/locks。
- 页面/模块范围：`models/auth.rs`、`auth_contract.rs`、`features/authentication/contracts*`、manifests。
- 整洁性：public DTO 仅批准字段；private secret 不进 model/TS。
- Pattern/边界：Rust contract source -> TS direct translation；无 mapper/通用 schema 层。
- 上下文影响：strict/no alias；复用 AppError/IpcTransport；不升级无关依赖。
- 交互/状态：无 UI/IPC；冻结九状态和 nullable invariant。
- API/类型：四 command/event 共享 snapshot；mid nullable string、revision safe integer。
- 测试切入：Rust exact JSON red、TS fixture red、typecheck、dependency build、secret field scan。
- 已批准澄清：Q1/Q8/Q9；qrcode 前端绘制、keyring v1、唯一 auth event。

## AUTH-02

- 任务名称：B 站二维码与会话验证 Adapter
- 状态：completed
- 执行模式/说明：串行；去敏 fixture 与边界表驱动。
- 触发/前置：AUTH-01 completed。
- 规格映射：AC-AUTH-03/05/06/07/15/16；外部 B 站 adapter contract。
- 功能单元：generate/poll/nav raw DTO、status mapping、Cookie allowlist、redirect/timeout/body guard。
- 页面/模块范围：`infrastructure/bilibili/auth_raw.rs`、`auth_client.rs`、fixtures/tests。
- 整洁性：raw code/Set-Cookie 单一 owner；不进入 manager/Vue。
- Pattern/边界：QrAuthPort/Adapter；复用 bounded decode，不复制完整 client。
- 上下文影响：现有 reqwest/WBI/匿名 client 回归；Cookie 只对 HTTPS allowlist host。
- 交互/状态：无 UI；只产 stable enum/account/private credential。
- API/类型：86101 waiting_scan、86090 waiting_confirm、0 confirmed、86038 expired；未知 fail closed。
- 测试切入：四码/未知码/缺字段、nav、非 2xx、4MiB、redirect、防泄漏。
- 已批准澄清：Q3/Q5/Q6/Q7；cancelled 不来自外部 code。

## AUTH-03

- 任务名称：系统凭据库 Port 与生产 Adapter
- 状态：completed
- 执行模式/说明：串行；fake fault matrix + 当前平台 production build。
- 触发/前置：AUTH-02 completed。
- 规格映射：AC-AUTH-05/06/07/08/15/16；Credential contract。
- 功能单元：CredentialStorePort、keyring v1、schemaVersion=1 secret wire、fake store。
- 页面/模块范围：`services/auth/ports.rs`、`infrastructure/auth/*`、store tests。
- 整洁性：secret owner 不 Debug/Serialize；adapter 不决定 auth status。
- Pattern/边界：小 Port/Adapter；拒绝 Repository/factory/plaintext/sample fallback。
- 上下文影响：固定 service/user；不增加 WebView secret capability。
- 交互/状态：NotFound anonymous；platform failure 错误；delete failure 可区分。
- API/类型：private bytes/text secret；零化临时缓冲，不跨 IPC。
- 测试切入：get/set/delete/not-found/corrupt/platform failure、源码/构建扫描。
- 已批准澄清：Q1/Q6/Q10；自动化用 fake store，不冒充真实 keyring runtime。

## AUTH-04

- 任务名称：AuthManager 状态机、轮询与凭据事务
- 状态：completed
- 执行模式/说明：串行；fake clock/sleeper/delayed ports，禁止真实 sleep。
- 触发/前置：AUTH-03 completed。
- 规格映射：AC-AUTH-03..08/11/12；Rust polling 与凭据事务。
- 功能单元：restore/start/refresh/poll/cancel/confirmed/logout、generation/revision/event。
- 页面/模块范围：`services/auth/{manager,state_machine,polling,context}.rs` 与 manager tests。
- 整洁性：external I/O -> guard -> secure persist -> commit -> emit；锁内不 await。
- Pattern/边界：State enum/guard、Observer sink、generation token；无 state classes。
- 上下文影响：后续 parser 与 Tauri 的唯一 auth 权威；无 UI timer/polling。
- 交互/状态：2s、180s、无第91次；cancel 幂等；deadline 后 confirmed 无效。
- API/类型：完整 cookie + valid nav + secure set 后才 authenticated；revision 单调。
- 测试切入：全状态矩阵、调用时刻、迟到 response、save/delete fault、event ordering。
- 已批准澄清：Q3/Q7/Q9；cancelled 本地、monotonic deadline、单 event。

## AUTH-05

- 任务名称：Parser 认证上下文与缓存隔离
- 状态：completed
- 执行模式/说明：串行；调用顺序与回归测试优先。
- 触发/前置：AUTH-04 completed。
- 规格映射：AC-AUTH-07/09/10/16；解析前校验。
- 功能单元：validated_context、request context、auth revision cache key、失效降级。
- 页面/模块范围：`services/auth/context.rs`、`services/parser.rs`、BilibiliPort/client、composition/tests。
- 整洁性：parser 不读 store/删 secret；AuthContextProvider 单一 owner。
- Pattern/边界：窄 context provider；不复制 parser/client，不引入 TTL cache。
- 上下文影响：保留 ParseVideoResult、匿名 E005、WBI 与任务 handoff 语义。
- 交互/状态：explicit invalid emit anonymous 后匿名 parse；transient error 停止后续请求。
- API/类型：normalize -> remote validate -> revision cache key -> lookup -> parse。
- 测试切入：cache hit 前 nav、valid/invalid/transient、revision separation、host containment、匿名回归。
- 已批准澄清：Q2/Q5；每次强制远端验证、账户由 nav 提供。

## AUTH-06

- 任务名称：Tauri command/event 与根级 Auth Store
- 状态：completed
- 执行模式/说明：串行；exact transport + lifecycle TDD。
- 触发/前置：AUTH-05 completed。
- 规格映射：AC-AUTH-01/06/11/12；初始化、API、Observer 生命周期。
- 功能单元：四 commands、event sink/service/listener、injection/demo、Pinia store。
- 页面/模块范围：`commands/auth.rs`、`lib.rs`、`features/authentication/{service,events,injection,demo,store}*`、`main.ts`、`App.vue`。
- 整洁性：invoke/listen 只在 adapter；root 单订阅；demo DEV-only。
- Pattern/边界：Observer revision replay；拒绝通用 Event Bus 和页面各自 listen。
- 上下文影响：TopBar 未访问 LoginPage 也恢复；保留 task event lifecycle。
- 交互/状态：先 subscribe、缓冲、snapshot、重放；pending finally；destroyed response 忽略。
- API/类型：四个 invoke 无 args；event exact `auth://state` + `{ snapshot }`。
- 测试切入：registration、exact calls、partial subscribe cleanup、倒序/重复/destroy/pending。
- 已批准澄清：Q9/Q10；根 store 唯一状态源、demo 不等于 native 成功。

## AUTH-07

- 任务名称：LoginPage、二维码工具、TopBar 与退出交互
- 状态：completed
- 执行模式/说明：串行；component/page TDD + browser visual。
- 触发/前置：AUTH-06 completed。
- 规格映射：AC-AUTH-01/02/08/10/12/13/14；页面设计全部状态。
- 功能单元：QrLoginPanel、AuthAccountPanel、LogoutConfirmDialog、page lifecycle、TopBar、失效提示、locales/CSS。
- 页面/模块范围：LoginPage/TopBar/auth components/tests/styles/locales/必要的 DownloadPage 提示。
- 整洁性：组件 props/emits，不 import store/service；LoginPage 约160行。
- Pattern/边界：Lucide + tooltip、Naive dialog；无嵌套卡片/渐变/原文 URL。
- 上下文影响：复用 AppShell/tokens/router；不改变导航结构和匿名能力。
- 交互/状态：九状态、224px、倒计时非 live、unmount cancel、800ms return、logout focus restore。
- API/类型：只消费 public snapshot/actions；HTTPS B 站 avatar allowlist。
- 测试切入：自动 start、全部状态、QR error、timer cleanup、route source、focus、1280/900/800/narrow/light/dark。
- 已批准澄清：Q4/Q5/Q8；固定返回逻辑、nullable account、canvas 绘制。

## AUTH-08

- 任务名称：集成、安全、视觉与验收证据
- 状态：completed
- 执行模式/说明：串行收口；失败回 owner task。
- 触发/前置：AUTH-07 completed。
- 规格映射：AC-AUTH-01..16 全部；测试、人工评审与交接。
- 功能单元：全量门禁、security scan、visual matrix、evidence/changelog、opt-in smoke 状态。
- 页面/模块范围：全部 auth scope + parser/AppShell/tasks 回归 + execution artifact。
- 整洁性：claim 与 evidence 一一对应；warning/ignored 不伪装 pass。
- Pattern/边界：复核 State/Observer/Port/generation 仍保持轻量。
- 上下文影响：为 verify/review 提供原始证据；不提前开始 settings。
- 交互/状态：只有全部自动化与视觉 blocker 清零才移交 verify。
- API/类型：exact commands/event/serde/TS 全链；public/secret 边界静态扫描。
- 测试切入：npm test/typecheck/build、cargo fmt/check/test、security scans、多状态视觉、canvas pixels。
- 已批准澄清：Q10；无授权真实扫码 smoke 必须 ignored 并说明。

## AUTH-R01

- 任务名称：Review 阻断项修复与回归
- 状态：completed
- 执行模式/说明：review 回流；逐项 red -> green，完成后重跑 verify/review。
- 规格映射：AC-AUTH-01/07/08/10/14；外部 B 站 adapter contract、Observer 生命周期、可访问交互。
- 修复范围：nav `-101` 明确失效映射、并发 initialize listener 清理、alertdialog/Escape、TopBar restoring、DownloadPage session-invalid 提示、QR 绘制迟到结果隔离。
- 测试切入：raw adapter fixture、deferred subscribe、dialog role/keyboard、TopBar 恢复态、认证状态跨页提示。
