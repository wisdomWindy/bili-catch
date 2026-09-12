# 工程规格：登录与认证

## 交付单元标识

`authentication`

## 背景与目标

现有应用只能匿名解析，TopBar 固定显示“未登录”，`/login` 仍是占位页。解析结果已保留 `requiresLogin`，但没有安全获得、恢复和验证 B 站会话的能力。

本模块交付完整扫码登录：二维码约 180 秒有效，Rust 每 2 秒轮询并处理未扫码、待确认、确认、过期和取消；确认后仅在 Rust 提取并安全存储 allowlist Cookie，启动可恢复，退出可清除；每次解析前远端验证 session，失效后降级匿名。Vue 展示无秘密 snapshot，TopBar 与登录页共享一个认证状态源。

## 范围内

- `/login` requesting、waiting_scan、waiting_confirm、authenticated、expired、cancelled、error/restoring/anonymous 完整页面状态。
- 二维码生成、固定 180 秒本地 deadline、2 秒 Rust 轮询、刷新/取消、迟到结果隔离。
- 登录确认后的 Cookie allowlist 提取、系统凭据库安全存储、启动恢复、退出清除。
- 每次 parse 前 session nav 验证；明确失效即清除并匿名解析，瞬时网络失败不误删。
- TopBar 匿名/已登录状态同步；受限清晰度登录返回后可重新解析。
- 中英文、暗色主题、响应式、键盘/ARIA、错误脱敏。
- 去敏 fixture、fake clock/poller/store、跨端 serde/TS contract 与 opt-in 真实扫码 smoke 入口。

## 范围外

- 账号密码、短信、验证码自动处理、Cookie 手工导入导出、多账号切换、云同步。
- 绕过 B 站风控、地区/版权/会员权限或内容保护。
- 固定承诺 8K/HiRes；能力仍以账号和具体视频解析结果为准。
- 将 Cookie 发送给 B 站以外域名、前端读取 Cookie、认证日志/遥测。
- 通用 secrets UI、Tauri Store/Stronghold WebView 权限、自动刷新失效 Cookie。

## 触发与开始条件

- 用户点击 TopBar 认证入口或解析错误/受限能力的登录引导进入 `/login`。
- 应用启动时 AuthManager 恢复系统凭据库中的单一 B 站会话。
- `parse_video` 每次执行前调用认证 context provider。

前置：foundation、parse-download-center、task-management 已通过 review；当前 page-design 与 architecture-design 已完成。

## 需求拆分摘要与来源

- 模块来自 `requirements/modules/authentication.md`，只负责认证状态与能力上下文，不实现设置、下载执行或系统发布。
- 原始来源为 `biliCatch_PRD.md` 1.4、3.2、模块五约 423-478 行、5.4；五个 QR 结果、180 秒、2 秒轮询、加密保存、恢复、退出、解析前检查和匿名可用全部保留。
- 页面结构承接 `design/page-design.md`；模块、secret、polling、parser integration 边界承接 `design/architecture-design.md`。

## 用户流程

### 启动恢复

1. AuthManager 初始 `restoring`，从系统凭据库读取 `com.bilicatch.app / bilibili-session`。
2. 没有 credential -> `anonymous`；读取错误 -> `error` 且不得生成/覆盖明文文件。
3. 有 credential -> 调 B 站 nav 验证。明确未登录时删除 secret 并进入 `anonymous`；有效时映射 account 并进入 `authenticated`；网络/超时保持 credential，snapshot 为 `error` 并允许重试恢复。
4. 根级 auth store 在 App 挂载时先订阅 `auth://state`、再取 snapshot、按 revision 重放，TopBar 不等待 LoginPage 才更新。

### 扫码登录

1. 匿名访问 `/login` 自动调用 `start_qr_login`，同一 pending 期间不得重复。
2. Rust 增加 generation，取消旧 poll，状态 `requesting`；generate 成功后返回 `qrContent`、`expiresAt=created+180s`，状态 `waiting_scan`。
3. 前端用 `qrcode` package 将 `qrContent` 绘制到 224x224 canvas；字符串不写 DOM 文本、日志、Pinia persistence 或 localStorage。
4. Rust 每 2 秒 poll：86101 -> waiting_scan；86090 -> waiting_confirm；0 -> confirmed；86038 或本地 deadline -> expired；未知码 -> error。
5. confirmed 时提取 allowlist Set-Cookie，调用 nav 得到账户信息，再安全存储 credential；三步全成功才 commit `authenticated` event。存储失败不得短暂显示成功。
6. 前端显示成功 800ms，先观察 TopBar 已更新，再返回进入 `/login` 前的有效应用路由；直接打开或来源是 `/login` 时到 `/download`。

### 刷新、取消与卸载

- expired/cancelled/error 展示“刷新二维码/重试”；点击调用 start，旧 generation 任何迟到响应忽略。
- LoginPage 卸载且仍为 requesting/waiting 状态时调用 `cancel_qr_login`；Rust abort poll 并流转 `cancelled`。authenticated/anonymous/terminal 状态的 cancel 是幂等 no-op。
- 刷新时旧二维码立即不可用并进入同尺寸 requesting skeleton，不并排保留多个二维码。

### 已登录与退出

- authenticated 页面显示昵称、可用时的 UID/头像、成功状态与“退出登录”；缺字段使用“已登录账号”/UserRound 占位。
- 头像加载失败只影响图片，不改变 auth status；URL 只允许 HTTPS 的 B 站图片域名，否则置 null。
- 退出需要确认，默认焦点“保留登录”，关闭后归还。确认后调用 `logout`；安全删除成功才进入 anonymous 并自动开始新 QR 流，失败保留 authenticated 和 account，显示 E007/E_INTERNAL。

### 解析前校验

1. `parse_video` 在规范化后、读取结果缓存前调用 `validated_context()`，每次都远端请求 nav，不用 TTL 跳过。
2. 有效 credential -> Bilibili request 由 Rust infrastructure 添加 Cookie；缓存 key 含 auth revision。
3. 明确失效 -> 安全删除、revision 增加、emit anonymous/expired 提示，然后继续匿名 parse；匿名基础能力仍可返回。
4. 网络/超时 -> 返回 E001/E002，不清除 credential，不用未验证 Cookie 发后续受限请求。
5. 登录/退出/recovery revision 变化使旧匿名/登录解析 cache 不再命中，不修改 ParseVideoResult 字段语义。

## 页面与模块设计

- 复用 AppShell；标题下是约 720-820px 左对齐认证工具区。>=760px 为 224px QR + 状态双列，<760px 单列，<520px QR 以 `min(224px, 100%)` 保持 1:1。
- QR 固定白底/高对比，无渐变/插画；requesting 使用同尺寸 skeleton。状态图标+文本共同表达，倒计时等宽数字但不进入 live region。
- 状态变化用 `aria-live=polite`，不朗读每秒倒计时；按钮 >=36px。完成账户态替换二维码，不嵌套卡片。
- TopBar 宽屏显示昵称并省略，窄屏只显示图标但 aria-label 含状态/名称。
- LoginPage 只编排生命周期和返回路由；Auth store 合并 snapshot；组件 props/emits；service/events 独占 transport。

## Function-complete 行为拆分

### Auth store 初始化与合并

- 状态：`idle | subscribing | loading | ready | failed`，另有 `snapshot/error/pending/dispose`。
- 先 subscribe 后 snapshot，缓冲 event；只接受 revision 大于当前的 snapshot/event。命令响应 revision 不小于当前才合并。
- `start/cancel/logout` 各有独立 pending，finally 必须清除；destroyed 后 response 不写 store。
- 倒计时由 `expiresAt - Clock.now` 派生并 clamp 0..180；只用页面 interval 更新显示，不产生 auth 状态，卸载清 timer。

### QR 登录面板

| Status | QR 区 | 状态/操作 |
| --- | --- | --- |
| restoring/requesting | 224px skeleton | 恢复/生成中，无重复按钮 |
| anonymous | 自动进入 requesting | 若自动请求失败转 error |
| waiting_scan | canvas + mm:ss | 使用 B 站 App 扫描，无按钮 |
| waiting_confirm | canvas，视觉降级 | 已扫码，请在手机确认 |
| expired | 不可用覆盖 | E010 文案 + 刷新二维码 |
| cancelled | 不可用覆盖 | 已取消 + 刷新二维码 |
| error | 稳定占位 | 脱敏错误 + 重试 |
| authenticated | 不渲染本组件 | 账户组件 |

- QR renderer 输入为空、过长或编码失败进入局部 error，不向 Rust 回传 URL。
- refresh/retry 点击即禁用；成功替换 canvas，失败保留错误和可重试入口。

### Account 与确认框

- 字段：name 必有 fallback，mid 可空，avatarUrl 可空；不展示 Cookie 过期时间或 secret 标识。
- logout dialog 为 alertdialog、aria-modal，Escape/遮罩取消，初始与归还焦点具备测试。
- logout 成功后账户 UI 清除；失败不得先 optimistic anonymous。

### TopBar 与跨页反馈

- restoring 显示 spinner +“检查登录状态”；anonymous/error 显示 UserRound +“未登录”；authenticated 显示头像/占位 + nickname。
- 点击仍只导航 `/login`；TopBar 不发 start/logout command。
- parse 检测 session 明确失效时 auth event 更新 TopBar，并由 DownloadPage 显示本地化“登录已失效，已切换匿名”非阻塞提示；匿名解析结果仍正常展示。

### Rust polling 与凭据事务

- `start_login`、restore、logout 不在 mutex 内 await；每次外部结果回写前比较 generation。
- 本地 deadline 优先于远端迟到 confirmed；超过 `expiresAt` 的成功不得存储。
- allowlist cookie 至少要求 `SESSDATA` 与 `DedeUserID`；缺失返回 E_INTERNAL，不持久化部分 credential。
- credential save/delete 是认证 commit 前置条件；event 只在状态 commit 后发送完整 public snapshot。
- credential 类型不实现 Serialize/Debug；错误 details 禁止 secret/qrcode_key/qrContent/response body。

## 设计约束

### 责任、规则与副作用

- AuthManager 是 auth transition/generation/secure persistence 权威；Vue 不解释外部状态码或 Cookie。
- raw QR/nav/Set-Cookie 只在 Rust Bilibili adapter；system credential I/O 只在 CredentialStorePort adapter；emit 只在 AuthEventSink。
- 每次 parse 检查、失效清理和 auth revision 由 AuthContextProvider 单点拥有；TopBar/DownloadPage 不复制认证规则。

### 命名与复杂度

- 稳定名：AuthStatus、AuthSnapshot、AuthAccount、AuthStateEvent、AuthContextProvider、QrAuthPort、CredentialStorePort、generation、revision。
- command/manager 采用 external I/O -> generation check -> secure persist -> commit -> emit；锁内不得 await。
- 文件约 250 行审查，超过 350 行拆分；LoginPage 约 160 行内；生产 TS 禁止 any。
- 禁止通用 Event Bus/Repository/DI container、状态 class、前端 polling、明文 fallback、前端 secret、任意 cookie/header 持久化。

## 项目脚手架与允许偏离

- 复用已交付 create-tauri-app Vue/TS/Tauri 2 scaffold、Pinia/Router/i18n/Naive UI/Lucide/Vitest、现有 Rust services/infrastructure/commands 分层。
- 允许新增 `qrcode` 及类型声明、`keyring 4` 默认 `v1` feature、`secrecy/zeroize`、poll cancellation 所需最小 Tokio 能力。
- 不安装 Tauri store/stronghold WebView plugin，不扩大 frontend capability 读取 secret；不替换现有 HTTP client、store 或 UI 框架。

## 变化轴与 Pattern 决策

- State：九个 public lifecycle 状态用 enum/table/guard，拒绝 class hierarchy。
- Observer：单一 `auth://state` 支持根 store 订阅/取消/revision replay，不建 Event Bus。
- Port/Adapter：QR API、credential store、clock/sleeper、event 与 parser auth context 均有外部变化或确定性测试需求，使用小接口。
- Generation token：直接隔离 refresh/cancel/restore 迟到结果，不把 AbortController 跨 IPC 暴露。
- 无多 provider/多账号变化轴，拒绝 credential factory、Repository 和 Strategy 层。

## 代码上下文与影响假设

- 根 `tsconfig.json` governs：strict、ES2020、DOM、ESNext、bundler、isolatedModules、noEmit、无 alias/ambient 自定义类型；使用相对导入。
- 声明闭包：本模块 contracts、AppError/IpcTransport、Vue/Pinia/Router/i18n、Tauri event、qrcode package types；Rust serde DTO 为非 TS 权威源，无 protobuf/OpenAPI/backend TS。
- 影响：LoginPage、TopBar、App composition、DownloadPage/session notification、parser service/client/cache key、Rust models/services/infrastructure/commands/Cargo/tests/locales/styles。
- 回归：匿名 parse、WBI cache、任务事件、AppShell navigation/theme 不得改变。
- code graph unavailable 记录继续有效；人工依赖图已更新到 architecture artifact。

## API 与数据合同

### 稳定 Tauri contract

| Command | JS args | Result | 语义 |
| --- | --- | --- | --- |
| `get_auth_snapshot` | none | `AuthSnapshot` | 当前公开状态 |
| `start_qr_login` | none | `AuthSnapshot` | 取消旧周期并生成新 QR |
| `cancel_qr_login` | none | `AuthSnapshot` | 停止活跃 polling，幂等 |
| `logout` | none | `AuthSnapshot` | 安全删除成功后匿名 |

Event：`auth://state`，payload `AuthStateEvent`。前端 service 直接消费 camelCase stable DTO，无展示性 rename；events adapter 负责 listen/unlisten。

```ts
type AuthStatus = "restoring" | "anonymous" | "requesting" | "waiting_scan" |
  "waiting_confirm" | "authenticated" | "expired" | "cancelled" | "error";
interface AuthAccount { mid: string | null; name: string; avatarUrl: string | null }
interface AuthSnapshot {
  revision: number;
  status: AuthStatus;
  qrContent: string | null;
  expiresAt: string | null;
  account: AuthAccount | null;
  error: AppError | null;
}
interface AuthStateEvent { snapshot: AuthSnapshot }
```

- revision 为应用会话内非负安全整数；时间为 RFC3339 UTC。
- waiting_scan/waiting_confirm 必须有 qrContent/expiresAt；authenticated 必须有 account；其余状态这三项按语义为 null。
- error 是 public AppError；非 error 状态 error=null。列表/nullable 语义不能由组件猜测。

### 外部 B 站 adapter contract

| Purpose | Method/path | Request | Raw success | Stable mapping |
| --- | --- | --- | --- | --- |
| QR generate | GET `https://passport.bilibili.com/x/passport-login/web/qrcode/generate` | optional `source=main_web` | data.url, data.qrcode_key | ephemeral qrContent + private key |
| QR poll | GET `.../qrcode/poll` | qrcode_key | data.code/message/url/refresh_token + Set-Cookie | QrPollResult enum + private credential |
| Session validate/account | GET `https://api.bilibili.com/x/web-interface/nav` | Rust-only Cookie header | isLogin, mid, uname, face | Valid(account) / Invalid / transient error |

- 这是无正式 OpenAPI/protobuf 的外部 conventional contract；raw structs +去敏 fixture 是 contract source，adapter 保留上游字段，manager 只消费稳定 enum。
- 请求 connect timeout 8s、total 20s、response body <=4MiB；只允许 HTTPS Bilibili hosts，redirect 不得把 Cookie 带到非 allowlist host。
- poll raw code：86101 waiting_scan、86090 waiting_confirm、0 confirmed、86038 expired；未知/缺字段 fail closed。
- Set-Cookie 仅提取 allowlist；日志和 AppError details 不含 secret。`qrContent` 临时跨 IPC 只用于绘制，不持久化。

### Credential contract

- `keyring = "4"` 默认 `v1` feature：macOS Keychain、Windows Credential Manager、Unix Secret Service；service/user 固定且单账号。
- adapter 保存一个版本化 secret blob（schemaVersion=1、cookie allowlist、可选 refresh token），使用 keyring binary/text secret API；blob 只在 Rust 解析。
- NotFound 等价 anonymous；PlatformFailure/NoStorageAccess 映射 E007/E_INTERNAL 并 fail closed；测试使用内存 fake，禁止 sample store 进入 production。

## Context 与依赖来源

- PRD、module requirement、page/architecture design、现有 parser/client/TopBar/LoginPage、root tsconfig、Cargo/package manifests。
- Tauri 官方 Store/Stronghold 文档用于确认文件 store 与 password-derived secret store 边界；keyring 4 官方 docs/repository用于 v1 platform mapping。
- B 站端点无稳定官方文档；实现以最小 raw DTO、fixture 和 opt-in 网络探测防漂移。

## 边界与异常

- generate/poll/nav 非 2xx、body 超限、JSON 缺失、未知 code、超时/离线均为可重试 error，不泄漏 raw body。
- start 重复、refresh、unmount、restore/poll 并发只允许最新 generation commit。
- 本地 180 秒 deadline 到达后，迟到 confirmed 无效；系统时钟回拨用 monotonic elapsed 判定，expiresAt 只供展示。
- secure save 失败不 authenticated；logout delete 失败不 anonymous；corrupt credential 删除失败时 error 且不回传 secret。
- event subscribe 失败时 TopBar/LoginPage 仍可用 command snapshot，显示非阻塞 banner；卸载释放 listener/timer。
- QR canvas 生成失败不改变 Rust 周期；允许刷新，不显示原始 qrContent。
- account/avatar 缺失有稳定 fallback；昵称超长省略，完整值通过 title/aria-label。
- parse 失效降级匿名后，若内容本身需要登录仍按既有 E005；基础内容正常返回。

## 验收标准

- AC-AUTH-01：根 store 先订阅/快照/重放并按 revision 忽略重复/倒序，TopBar 在未访问 LoginPage 时也能显示恢复后的账户。
- AC-AUTH-02：匿名进入 `/login` 自动生成 QR；requesting 固定尺寸；waiting_scan 有 224px 可扫 canvas、180 秒倒计时和状态文本。
- AC-AUTH-03：fake clock/port 证明 Rust 每 2 秒 polling，86101/86090/0/86038 映射正确，180 秒停止且无第 91 次请求。
- AC-AUTH-04：refresh/cancel/unmount 中止旧 generation，迟到 waiting/confirmed 不能覆盖新 QR 或保存 credential。
- AC-AUTH-05：confirmed 仅在 allowlist Cookie 完整、nav valid、keyring save 成功后 authenticated；任一步失败无 secret 残留/成功 event。
- AC-AUTH-06：credential 不可 Serialize/Debug 到 public DTO，TS contract/event/log/错误均不包含 Cookie、token、qrcode_key、Set-Cookie 或 raw response。
- AC-AUTH-07：启动无 secret -> anonymous；有效 secret -> authenticated account；明确失效删除后 anonymous；瞬时验证错误保留 secret 并可重试。
- AC-AUTH-08：logout 有安全焦点确认；delete 成功清 account/secret 并匿名，失败保留 authenticated 且解除 pending。
- AC-AUTH-09：每次 parse 在 cache 前远端验证；auth revision 进入 cache key；登录/退出/失效后不复用旧能力结果。
- AC-AUTH-10：失效 session 自动更新 TopBar 并匿名解析基础内容；登录受限内容仍按既有 E005，不破坏 ParseVideoResult。
- AC-AUTH-11：四 commands 与 `auth://state` exact args/字段、Rust serde、TS types/listener cleanup 测试通过；组件不直接 invoke/listen。
- AC-AUTH-12：二维码状态表全部可观察；状态 live region 不包含倒计时；刷新/重试 pending 防重复；QR error 不展示原文。
- AC-AUTH-13：authenticated 页面显示稳定 account fallback、头像失败降级和退出；成功 800ms 后返回有效来源或 `/download`。
- AC-AUTH-14：TopBar 宽/窄状态、LoginPage 1280/900/800px 及 error/expired/authenticated 无溢出、重叠或小于 36px 控件，明暗主题 QR 保持可扫白底。
- AC-AUTH-15：keyring production adapter 与 fake store fault tests 覆盖 get/set/delete/not-found/platform failure；无 plaintext/sample/Tauri webview secret permission。
- AC-AUTH-16：`npm test`/typecheck/build、cargo fmt/check/test、静态 secret/import/event scan 全通过；真实扫码 smoke 若无用户授权明确 ignored，不伪造。

## 人工评审与交接

- 用户批准本规格后进入 plan；未批准不得写认证代码。计划需按 AC-AUTH-01..16 给出 red/green、fixture、fault injection、视觉与 security scan。
- review 必须检查 secret spread、外部状态码、poll cancellation、parser cache/auth revision、TopBar subscription 和跨平台 keyring 风险。
- 完成后 settings 不读取认证 secret；audio/video 通过 parser/auth context 获得能力，不自行保存 Cookie；system-release 验证启动恢复/窗口退出。

## 风险

- B 站 QR/raw code 无正式版本承诺，线上变化可能导致 error；adapter/fixture/opt-in smoke 必须使变化可定位。
- Unix Secret Service 取决于桌面会话；不可用时登录持久化失败并保持匿名，发布阶段需三平台实机验证。
- 每次 parse 远端 nav 增加延迟/请求量，这是满足 PRD“每次检查”的批准安全口径；后续若引入 TTL 必须改规格。
- 登录成功需要真实手机与账号，自动化只证明协议/存储/状态，不宣称真实账号在线可用。
- QR login URL 不是长期 credential 但可在有效期内授权；仅临时跨 IPC 绘制，禁止持久化/日志/复制入口。
