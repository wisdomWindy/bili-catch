# 架构设计：登录与认证

## 交付单元标识

`authentication`

## 架构目标

在现有 Vue/Tauri 工程中建立唯一认证事实源：Rust 负责 B 站二维码协议、2 秒轮询、180 秒生命周期、Cookie 捕获与安全存储、启动恢复和解析前有效性校验；Vue 只呈现不含秘密的认证快照并管理页面订阅。任何 Cookie、refresh token 或完整登录响应都不得进入 webview、Pinia 持久化、日志或 Tauri event。

## 架构范围与触发条件

- 新增 `/login` 完整状态页并让 TopBar 消费全局认证摘要。
- 新增 QR generate/poll/logout/session commands 与 `auth://state` 事件。
- 新增 Rust credential store、二维码 API adapter 和认证 manager。
- 将已完成 parser 的匿名认证占位替换为 `AuthContextProvider`；每次 parse 在读取缓存前取得已验证认证 revision。
- 不实现账号密码登录、短信登录、验证码绕过、多账号、Cookie 导入导出或云同步。

## 上游输入与假设

- 页面结构以本模块 `page-design.md` 为准，既有 `AppShell`、TopBar、hash route 与 i18n 不重做。
- B 站网页二维码接口是无正式版本承诺的外部契约；raw DTO 只在 Rust infrastructure，稳定 DTO 由本模块拥有。
- QR 状态已知映射：未扫码、已扫码待确认、已确认、已过期；本地 refresh/unmount/显式 cancel 产生 `cancelled`。未知码必须进入 error，不猜测成功。
- Tauri 官方 Store 是文件型 key-value persistence，不作为加密凭据库；Cookie 使用系统凭据库 adapter。安全存储不可用时 fail closed，不回退明文。
- 真实账号 smoke 需要用户授权扫码，只能 opt-in；默认自动化使用去敏 raw fixture 和 fake ports。

## 模块边界设计

### Frontend

- `features/authentication/contracts.ts`：稳定 snapshot、status、account、command result/event 类型；不出现 cookie/login response raw 字段。
- `features/authentication/service.ts`：四个 exact invoke 的 request layer。
- `features/authentication/events.ts`：唯一 `auth://state` 订阅与 unsubscribe。
- `features/authentication/store.ts`：Pinia 唯一 UI 状态源，拥有先订阅/快照/重放、revision 合并、pending/error、倒计时派生和 start/cancel/logout 编排。
- `features/authentication/injection.ts`：production/demo service 与 event source 注入键。
- `features/authentication/components/QrLoginPanel.vue`：二维码舞台、状态和刷新；只收 props/emits。
- `features/authentication/components/AuthenticatedAccount.vue`：账户摘要与退出意图。
- `features/authentication/components/LogoutConfirmDialog.vue`：确认、初始安全焦点、归还焦点。
- `pages/LoginPage.vue`：装配 store、组件、来源路由与成功返回；不直接 invoke/listen。
- `components/layout/TopBar.vue`：只读 auth store 派生 label/avatar，占位与路由行为仍由 TopBar 拥有。

### Rust

- `models/auth.rs`：公开 serde DTO；秘密类型不得实现 Serialize。
- `infrastructure/bilibili/auth_raw.rs`：QR generate/poll/nav raw envelope 与状态码。
- `infrastructure/bilibili/auth_client.rs`：端点、header/cookie 提取、大小/超时/error mapping；实现 `QrAuthPort`。
- `infrastructure/auth/credential_store.rs`：实现 `CredentialStorePort`，只在 Rust 使用系统凭据库；service name 固定 `com.bilicatch.app`，entry 固定 `bilibili-session`。
- `services/auth/ports.rs`：`QrAuthPort`、`CredentialStorePort`、`AuthEventSink`、`Clock/Sleeper` 小接口。
- `services/auth/manager.rs`：权威 snapshot、revision、poll generation、取消、持久化事务和启动恢复。
- `services/auth/context.rs`：实现 parser 消费的 `AuthContextProvider`，返回内存中的 opaque/secret auth context 与 revision。
- `commands/auth.rs`：薄 Tauri commands；`lib.rs` 只组装 manager、parser provider 与 event adapter。

## 文件与目录结构

```text
src/
  features/authentication/
    contracts.ts service.ts events.ts store.ts injection.ts
    components/QrLoginPanel.vue
    components/AuthenticatedAccount.vue
    components/LogoutConfirmDialog.vue
    authentication.css
  pages/LoginPage.vue
  components/layout/TopBar.vue
src-tauri/src/
  models/auth.rs
  services/auth/{mod.rs,manager.rs,context.rs,ports.rs}
  infrastructure/auth/{mod.rs,credential_store.rs}
  infrastructure/bilibili/{auth_raw.rs,auth_client.rs}
  commands/auth.rs
```

允许依据 250/350 行阈值拆分 manager mutation/polling 文件；不得为单一 adapter 创建通用 Repository、DI container 或应用级 Event Bus。

## 代码关系与依赖方向

```text
main.ts -> auth service/events -> LoginPage -> auth store -> auth components
                              -> TopBar (read-only derived snapshot)

Tauri commands -> AuthManager -> QrAuthPort -> Bilibili auth client
                            -> CredentialStorePort -> OS credential store
                            -> AuthEventSink -> auth://state

ParserService -> AuthContextProvider -> AuthManager validated context
             -> BilibiliPort(request context) -> authenticated/anonymous request
```

组件不得反向依赖 store/service。Bilibili raw DTO 不得穿过 infrastructure。Parser 不读取 credential store，也不决定 Cookie 失效后的删除规则。

## 职责切分

| 单元 | 拥有 | 不拥有 |
| --- | --- | --- |
| LoginPage | 页面生命周期、来源路由、成功返回 | 轮询 timer、Cookie、API code mapping |
| Auth store | 快照合并、倒计时显示、pending/error | secret persistence、HTTP、raw status |
| AuthManager | 状态机、generation、2s/180s、secure save/delete、revision | QR 绘制、页面布局、parser media rules |
| QrAuthPort | QR API 与 Set-Cookie 捕获 | 应用状态、存储、event |
| CredentialStorePort | secret bytes set/get/delete | account UI metadata、业务状态 |
| AuthContextProvider | parse 前 validate 与 auth revision | media解析、前端通知文案 |

## 函数设计与公开入口

Tauri commands：

- `get_auth_snapshot() -> AuthSnapshot`
- `start_qr_login() -> AuthSnapshot`
- `cancel_qr_login() -> AuthSnapshot`
- `logout() -> AuthSnapshot`

Manager 入口：

- `restore_on_startup()`：读 secure credential -> nav validate -> authenticated 或 anonymous；损坏/失效凭据先删除。
- `start_login()`：取消旧 generation -> requesting -> generate -> waiting_scan -> spawn poll loop。
- `apply_poll_result(generation, result)`：generation 不匹配即忽略；状态映射后先安全存储，再发布 authenticated。
- `cancel_login(reason)`：终止 poll handle；仅活跃 QR 流转 cancelled。
- `logout()`：先删除 secure credential，成功后切 anonymous；删除失败保留 authenticated 并返回 error。
- `validated_context()`：每次 parse 前验证当前 credential；有效返回 `AuthRequestContext { revision, secret }`，失效清除并返回 anonymous revision。

start/refresh/poll/restore 是 async，但锁内不等待网络或系统凭据 I/O；使用 generation + derive candidate + commit guard 避免迟到结果覆盖新会话。

## 状态归属与数据流

Rust public status：`restoring | anonymous | requesting | waiting_scan | waiting_confirm | authenticated | expired | cancelled | error`。PRD 的五个二维码结果完整保留，额外 lifecycle 状态用于真实 loading/恢复/失败 UI。

1. App setup 创建 AuthManager，先暴露 restoring snapshot，再异步 restore。
2. 前端在应用根级初始化 auth store：先 subscribe、再 snapshot、重放更高 revision event；LoginPage 不重复创建全局订阅。
3. start 命令返回新 revision snapshot；event 用于 TopBar 与跨页更新。
4. poll loop 每 2 秒读取一次；本地 deadline 到 180 秒时直接 expired 并停止，不依赖远端晚到响应。
5. confirmed 响应只把 allowlist Cookie 组装进 `SecretString/zeroizing` 内存对象；secure store 成功后才 commit authenticated snapshot/event。
6. parse 先调用 validated_context，再读以 `auth_revision + canonical input` 为键的结果缓存；登录、退出或过期后的 revision 变化自然隔离旧能力结果。

## 数据结构与类型策略

```ts
type AuthStatus = "restoring" | "anonymous" | "requesting" | "waiting_scan" |
  "waiting_confirm" | "authenticated" | "expired" | "cancelled" | "error";

interface AuthAccount { mid: string; name: string; avatarUrl: string | null }
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

- `qrContent` 仅在 waiting 状态出现，是 180 秒临时登录 URL；禁止日志/持久化，前端仅交给 QR renderer。
- `mid` 用字符串保持大整数/外部标识语义。
- Rust 私有 `StoredCredential` 含 allowlist cookie header 与可选 refresh token，不实现 Serialize/Debug；用 secrecy/zeroize 限制意外暴露。
- event/command DTO 使用 serde camelCase，与 TS 字段原样一致。

## Contract 与 Adapter 边界

- QR 外部端点：generate 返回 `url/qrcode_key`；poll 用 key，状态码由 raw adapter 映射，不在 manager/template 解释数字。
- confirmed 只接受 HTTPS allowlist 响应，提取 `SESSDATA`、`bili_jct`、`DedeUserID`、`DedeUserID__ckMd5`、`sid` 等明确 cookie 名；拒绝把所有 header 原样存储。
- session validation 使用 B 站 nav 的登录标志与 account 字段；网络暂时失败返回 E001/E002，不把“无法验证”直接当“已失效”。明确未登录才删除 credential。
- Tauri event 只含 `AuthSnapshot`；所有前端输入命令均无 Cookie/path/url 参数。
- 普通 Tauri Store 可在后续 settings 模块保存非敏感设置；认证模块不开放 `store:default`/stronghold WebView 权限。

## Pattern 决策与拒绝方案

- **State**：二维码与 session 状态有九个可观察分支和严格转换，使用 enum + table/纯 guard，拒绝散落模板条件；不创建状态 class hierarchy。
- **Observer**：一个 `auth://state` 让 TopBar/LoginPage 共享状态；由根级 store 单订阅并清理，拒绝通用 Event Bus。
- **Port/Adapter**：外部 QR API、系统凭据库、时间/睡眠和 Tauri emit 都有真实变化/测试替换需求；接口保持最小。
- **Generation token**：解决 refresh/cancel 与旧 poll response 竞态；比跨层 AbortController 或锁内 await 更直接。
- 拒绝前端轮询：会让页面卸载、后台恢复、secret capture 与跨页状态分裂。
- 拒绝 JSON/tauri-plugin-store 明文或应用内硬编码密钥加密：不能满足“加密存储”安全语义。
- 拒绝多账号 Repository/credential factory：v1 只有一个本机 B 站会话。

## 可读性与维护护栏

- 状态转换、raw code mapping、credential allowlist 各只有一个 ownership point。
- command/manager 按 validate -> external I/O -> generation check -> secure persist -> commit -> emit 阅读；禁止锁内 await。
- 单文件约 250 行进入拆分审查，超过 350 行必须拆分；LoginPage 约 160 行内。
- 生产日志与错误 details 禁止 Cookie、token、qrcode_key、qrContent、Set-Cookie、完整 response body；测试 fixture 使用假值并扫描这些字段。
- TopBar 只消费公开 snapshot，不让认证模块重做壳层或导航。
- governing TS config 保持 strict/ES2020/bundler/relative imports；没有可复用后端 TS/protobuf，Rust serde DTO 是 contract source。

## 架构风险

- B 站 QR API 无正式版本承诺：raw adapter 与 fixture 必须隔离字段/状态码变化，未知码 fail closed。
- 不同 Linux 桌面可能无可用 Secret Service：安全存储失败时允许本次认证不落盘并呈现明确错误，不得明文 fallback；跨平台实机列为发布验证。
- 每次 parse 都做 nav validation 增加请求延迟；可在 AuthManager 内使用短 TTL（建议 60 秒）验证缓存，但每次 parse 必须经过 provider 检查且不能绕过明确过期。
- 外部头像 URL 会失败；其加载不进入认证成功条件，UI 使用本地占位。
- 启动 restore 与用户 start 竞态通过同一 generation 解决，需 fake clock/delayed port 测试。

## 未决架构问题

- 系统凭据库 crate 在实现计划前需固定版本/features，并确认 Windows Credential Manager、macOS Keychain、Linux Secret Service 构建依赖；若当前 CI 缺 Linux secret-service 依赖，应用 feature-gated adapter 加明确不支持错误，不能静默明文。
- B 站当前 generate/poll 响应及 Cookie allowlist 需通过一次去敏网络探测或用户 opt-in 扫码 smoke 验证；没有真实账号时以 raw fixture 为阻断外依赖而非伪造成功。
- 60 秒 session validation TTL 是性能建议；规格需决定是否采用，并明确“每次解析检查”是 provider 检查还是每次强制远端请求。
