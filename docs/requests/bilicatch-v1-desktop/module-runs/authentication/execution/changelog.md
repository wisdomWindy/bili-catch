# 执行记录：登录与认证

## 执行基线

- 2026-09-11：用户批准认证实施计划，工作流进入 `execute`，从 AUTH-01 串行开始。
- 工作区不是 Git 仓库，无法创建 worktree/commit 检查点；采用任务板、red/green 命令输出和按任务文件边界记录回退点。
- code graph 不可用，沿用现有 `rg` 手工调用链；TypeScript 由根 `tsconfig.json` 管理，无 alias 或额外 ambient contract。
- 实施合同为已批准 spec、architecture-design、plan；本阶段不改变产品语义或开启后续模块。

## AUTH-01：公开合同、上下文闭包与依赖

- 状态：completed
- TDD red：`cargo test --test auth_contract` 因四个 auth public type 未定义而失败；`npm run typecheck` 因 `./contracts` 不存在而失败。Vitest 的 type-only import 被擦除而先通过，未将其误记为 red。
- TDD green：新增 Rust serde/TS exact contracts；Rust 2 tests、TS 2 tests、typecheck 通过。
- 依赖：锁定 `keyring 4.2.0`（Windows native store）、`secrecy 0.10.3`、Tokio 1.53.1、`qrcode 1.5.4`、`@types/qrcode 1.5.6`；npm audit 0 vulnerabilities。
- 门禁：Cargo check、cargo fmt check、npm typecheck、public model/TS secret boundary scan 通过；唯一 `qrContent` 为规格允许的临时公开字段。

## AUTH-02：B 站二维码与会话验证 Adapter

- 状态：completed
- TDD red：初始 unit compile 因 generate/poll/nav parser、QrPollState、Cookie extractor/host validator 缺失而失败；新增恶意 QR host 用例随后真实失败于未校验 URL。
- TDD green：7 tests 覆盖 generate、四个 raw code、未知码、nav valid/invalid、缺字段脱敏、Cookie allowlist/必填项及 HTTPS B 站 host；全部通过。
- 实现：新增 service-owned `QrAuthPort`/private credential types 与 `BilibiliAuthClient`，复用 4MiB bounded body reader；HTTP 固定 8s connect/20s total、redirect disabled。
- 安全：qrcode key、refresh token、Cookie 只在不实现 Serialize 的私有类型/SecretString；QR 与 authenticated nav URL 均通过 HTTPS B 站 host guard。
- 门禁：定向 7 tests、Cargo check、fmt check、日志/错误扫描通过。adapter 在 AUTH-04 组合前使用局部临时 `dead_code` 标注，届时移除。

## AUTH-03：系统凭据库 Port 与生产 Adapter

- 状态：completed
- TDD red：初始 compile 因 CredentialStorePort/SystemCredentialStore/fault backend 缺失而失败；新增 schema v1 但缺必填 Cookie 的用例随后真实失败。
- TDD green：5 tests 覆盖 versioned round-trip、not-found、unknown schema、缺必填 Cookie、get/set/delete unavailable 与 production construction。
- 实现：keyring v1 binary secret API 固定 `com.bilicatch.app / bilibili-session`；同步系统调用放入 `spawn_blocking`；临时 JSON bytes 使用 `Zeroizing<Vec<u8>>`。
- 错误：NoEntry 为 anonymous；NoStorageAccess/NoDefaultStore -> E007；其余平台/格式错误 -> E_INTERNAL，错误不带原始 bytes。
- 门禁：Cargo check/fmt 与 plaintext/sample/Tauri capability scan 通过；命中的 `sample_store` 仅为否定性测试名称。

## AUTH-04：AuthManager 状态机、轮询与凭据事务

- 状态：completed
- TDD red：manager compile 因 AuthManager/clock/sleeper/event ports 缺失而失败；首轮轮询测试稳定停在 WaitingScan，systematic debugging 证明为测试 helper 固定 yield 过早耗尽。
- TDD green：改用 1s 上限的条件等待后 8 tests 全过，覆盖 2s poll、180s deadline/无第91次、waiting-confirm/confirmed、save-before-auth、cancel、迟到 confirmed、restore valid/invalid/transient、logout delete failure。
- 实现：状态 Mutex 仅同步 commit；外部 QR/keyring I/O 全在锁外；credential `Semaphore(1)` 串行化 save/delete；generation guard 隔离迟到结果，event 在 commit 后 best-effort 发出。
- 重构：将 manager 拆为 338 行编排与 104 行 polling/confirmed 事务，满足 350 行阈值；移除临时诊断 sleep。
- 门禁：Cargo check/fmt、无 `lock().await` 扫描通过；未组合到 Tauri 前的局部 dead-code 标注留到 AUTH-06 移除。

## AUTH-05：Parser 认证上下文与缓存隔离

- 状态：completed
- TDD red：manager context tests 因 `AuthContextProvider`/`ValidatedAuthContext` 缺失而编译失败；parser tests 因 BilibiliPort 仍使用旧签名而失败；HTTP 边界测试同时证明 `authenticated_get` 尚不存在。
- TDD green：43 个 Rust unit tests 通过，覆盖 cache hit 前重复远端验证、revision cache 隔离、显式失效删除、瞬时失败保留会话并阻断媒体请求、Cookie 仅注入 authenticated request，以及校验晚于 logout 返回时不能恢复旧会话。
- 实现：新增窄 `ValidatedAuthContext`；Parser 固定 `normalize -> validate -> revision cache key -> lookup -> parse`；Bilibili client 统一在 Rust HTTP 边界注入 Cookie，short-link 请求保持匿名。
- 并发：logout/explicit invalid 会推进 generation；远程校验返回后再次检查 generation，迟到 valid 结果不能覆盖匿名状态。外部 I/O 均在同步状态锁外执行。
- 重构：会话恢复/退出移入 `context.rs`，`manager.rs` 287 行、`context.rs` 228 行，均低于 350 行强制拆分阈值。
- 门禁：Cargo check、fmt、43 passed/0 failed/1 ignored（仅 opt-in 公共端点 smoke）、无 `lock().await` 与凭据日志扫描通过；`production_with_auth` 的临时未使用 warning 将在 AUTH-06 组合根接入后消除。

## AUTH-06：Tauri command/event 与根级 Auth Store

- 状态：completed
- TDD red：三个 Vitest suite 因 authentication service/events/store 尚不存在而失败；Rust Cargo check 因已声明并注册的 `commands::auth` 模块及四个 handler 不存在而失败。
- TDD green：新增四命令无参 transport adapter、唯一 `auth://state` listener、Pinia 根 store 与 DEV-only demo runtime；7 个定向测试覆盖 exact names、payload unwrap/unlisten、subscribe-before-snapshot、buffer replay、revision 去重、listener failure fallback、独立 pending 与 dispose guard。
- 原生组合：Tauri setup 创建唯一 `Arc<AuthManager>`，同时 manage commands 与注入 Parser；使用真实 Bilibili auth client、系统 keyring、clock/sleeper 和 Tauri event sink，并异步启动 restore。
- 前端组合：`main.ts` 只在 `import.meta.env.DEV && demo=1` 时选择 demo runtime；`App.vue` 根级 initialize/dispose，页面和组件不直接 invoke/listen。
- 门禁：前端 25 files/96 tests、typecheck、Vite production build 通过；Cargo fmt/check、44 unit + 28 integration tests 通过，1 个 opt-in 公网 smoke ignored；直接 transport 与 public secret 扫描无命中。

## AUTH-07：LoginPage、二维码、TopBar 与退出交互

- 状态：completed
- TDD red：三个 component suite 因 QR/account/logout 组件不存在而失败；LoginPage 的自动 start、卸载 cancel、800ms 返回均失败；TopBar 仍固定显示匿名。退出焦点首轮还真实暴露 Naive focus-trap 与内容根节点时序冲突。
- TDD green：组件/page/shell 16 tests 通过，覆盖 canvas-only QR、224px 尺寸、绘制失败、头像 HTTPS B 站域名白名单、nullable UID、头像加载失败、退出安全焦点恢复、匿名自动 start、active unmount cancel、有效来源返回与 TopBar account 同步。
- 实现：LoginPage 只编排 root store；二维码原文不进入 DOM/aria，倒计时不进入 live region；退出使用 Naive Modal 并对两个操作实现确定性 Tab 循环；TopBar 只导航并携带来源路由。
- 视觉：本地 DEV demo 的 533x697 暗色窄屏实际截图无横向溢出/重叠，QR canvas 清晰非空；辅助树只含状态与操作文案，不含二维码 URL。
- 门禁：前端 29 files/104 tests、typecheck、production build 通过；Cargo check 无 warning。完整多状态/多视口证据在 AUTH-08 收口。

## AUTH-08：集成、安全、视觉与验收证据

- 状态：completed
- TDD red：不新增产品行为；失败回对应 owner task。
- Verify 回流：900px 暗色错误态暴露英文 adapter message；新增 LoginPage 本地化错误测试，观察真实 red 后改为只按稳定 AppError code 映射，重新截图通过。
- 自动化：前端 29 files / 106 tests，typecheck 与 Vite production build 通过；Cargo fmt/check、44 unit + 28 integration tests 通过，0 failed。
- 安全扫描：public TS/model/command 无 Cookie/token/qrcode_key/Set-Cookie；component/page 无 invoke/listen；auth scope 无 local/session storage、console/Rust debug log；capability 无 keyring/store/stronghold；无 dead-code allow。`plaintext/sample` 仅命中两条否定性测试名称。
- 视觉脚本：`scripts/verify-auth-visuals.cjs` 对 1280/900/800、requesting/waiting_scan/waiting_confirm/expired/error/authenticated、light/dark 共 7 视图通过；document width 等于 viewport，裁切/重叠/小于 36px 控件均为 0。
- QR 证据：waiting/confirm canvas 均为 224x224、21126 个非白像素、`rgb(255, 255, 255)` 底板；DOM 文本均不含 QR URL。截图位于 `verification/screenshots/`。
- 例外：`public_anonymous_parse_smoke` 需要显式公网 opt-in，本轮按批准规格保持 1 ignored；未用 fixture 冒充真实扫码或在线成功。
- 结果：AUTH-01..08 全部完成，执行阶段移交 verify，不在本记录中提前声明模块验收完成。

## AUTH-R01：Review 阻断项回流

- 状态：completed；同一 workflow run 由 review 以 `review_blocked` 回流 execute。
- 协议根因：实际 B 站 nav 匿名响应为顶层 `code=-101` + `data.isLogin=false`；原 adapter 在读取 data 前把所有非零码映射 E004。新增真实形状去敏测试后，将 `-101` 明确映射为 invalid session；头像域同步收敛为批准的 `hdslb.com`/`biliimg.com`。
- Observer 根因：stale `subscribe()` promise 在 lifecycle 检查前覆盖共享 unsubscribe。新增双 initialize deferred red，改为局部接收 unlisten，stale 时立即释放，只有当前 lifecycle 才提交所有权。
- UI 修复：logout 改为 `alertdialog` 并显式禁止 pending 时 Escape 关闭；TopBar restoring 使用 spinner/恢复标签；DownloadPage 观察 authenticated -> anonymous 并通过现有消息桥显示本地化非阻塞提示。
- QR 竞态：delayed `toCanvas` failure red 证明旧绘制会覆盖新 auth error；加入组件局部 render generation，状态变化/卸载后旧 promise 不再改写 UI。
- 安全补偿：新增 poll handle drop red，证明 stale credential 补偿删除失败时当前 poll 未 abort；错误提交现在原子推进 generation、取出并 abort task、清状态并 emit，后台结果不能覆盖安全错误。
- TDD 证据：初始前端 4 files/4 failures、Rust adapter 2 failures、QR race 1 failure、manager cleanup 1 failure均按预期观察；修复后相关前端 5 files/31 tests、Rust auth 12 tests、adapter 7 tests、typecheck、fmt check 通过。
- 外部构建信息：Rust test 在 Windows MSVC 链接时输出 linker 库创建提示；不是代码 warning，Cargo check/最终全量门禁在 verify 复核。
