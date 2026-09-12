# 规格澄清：登录与认证

## Q1：Cookie 如何“加密存储”？

- question：PRD 同时提到 tauri-plugin-store 与加密，但普通 Store 是文件型 key-value persistence，不提供设备凭据保护。
- answer：使用 Rust `keyring 4` 默认 `v1` 接口，落入 macOS Keychain、Windows Credential Manager、Unix Secret Service；不把 secret 暴露给 WebView。
- final decision：安全存储不可用时 fail closed，不使用明文 JSON、sample store 或硬编码密钥回退。
- affected spec area：CredentialStorePort、Cargo、启动恢复、跨平台风险。

## Q2：每次解析前检查是否允许 TTL？

- question：远端 nav 每次请求有延迟，但 PRD 明写每次解析前检查 Cookie 有效性。
- answer：本版本不采用 60 秒 TTL；每次 parse 在读取结果 cache 前远端验证。
- final decision：网络失败保留 credential 并返回 E001/E002；明确未登录才删除并匿名降级。
- affected spec area：AuthContextProvider、parser cache、错误语义。

## Q3：二维码取消来自哪里？

- question：B 站已知网页 QR code 包含未扫码、待确认、确认、过期，PRD 另要求 cancelled。
- answer：cancelled 是本地生命周期结果：页面卸载、刷新替换或显式 cancel 停止当前 generation。
- final decision：外部未知 code 不映射 cancelled，进入 error；旧 generation 结果全部忽略。
- affected spec area：状态机、poll cancellation、UI。

## Q4：登录成功后如何“关闭页面或返回”？

- question：桌面应用的 `/login` 是同一窗口路由，不能关闭独立页。
- answer：显示成功 800ms 后返回进入前有效应用路由；无来源或来源仍是 login 时到 `/download`。
- final decision：固定返回行为，不关闭窗口、不创建第二窗口。
- affected spec area：LoginPage、router、TopBar。

## Q5：账户信息展示哪些字段？

- question：QR poll 不保证直接返回完整账户，nav 可提供 mid/uname/face。
- answer：确认后用 authenticated nav 获取；name 有 fallback，mid/avatar nullable，头像只允许 HTTPS B 站图片域。
- final decision：缺失/头像失败不影响认证；不展示 Cookie 或 refresh token 元数据。
- affected spec area：AuthAccount、账户组件、安全。

## Q6：哪些 Cookie 被保存？

- question：Set-Cookie 可能包含无关或未来新增字段。
- answer：只收集明确 allowlist，至少要求 `SESSDATA` 与 `DedeUserID`；可包含 `bili_jct`、`DedeUserID__ckMd5`、`sid`，refresh token 私有可选。
- final decision：不保存所有 header，不把 cookie 名值放入日志/错误/public DTO。
- affected spec area：auth adapter、credential blob、secret scan。

## Q7：二维码过期以本地还是远端为准？

- question：远端状态可能晚于 180 秒或请求迟到。
- answer：monotonic 本地 180 秒 deadline 为硬上限；远端 86038 可提前确认过期。
- final decision：deadline 后 confirmed 也不得保存；expiresAt 只用于 UI 倒计时。
- affected spec area：Clock/Sleeper、race tests、expired UI。

## Q8：二维码由哪一端绘制？

- question：Rust API 返回登录 URL，PRD 建议前端 qrcode 库。
- answer：前端使用成熟 `qrcode` package 绘制 224px canvas，Rust 不生成自定义 SVG/位图。
- final decision：qrContent 只在 waiting snapshot 临时跨 IPC；UI 不显示、复制或持久化原文。
- affected spec area：依赖、QrLoginPanel、安全。

## Q9：认证事件是否新建通用 Event Bus？

- question：TopBar、LoginPage 和 parser 失效都需同步状态。
- answer：Rust 只 emit `auth://state`；根级 Pinia store 单订阅并成为 Vue 状态源。
- final decision：不建应用通用 Event Bus；LoginPage/TopBar 不各自 listen。
- affected spec area：Observer 生命周期、composition root、测试。

## Q10：没有真实账号是否阻塞自动化验收？

- question：完整确认需要用户用手机扫码，CI 无权持有账号 credential。
- answer：默认用去敏 raw fixture、fake QR port、fake keyring 和 fake clock 验证全部分支；真实扫码为显式 opt-in smoke。
- final decision：没有授权时 smoke 标 ignored 并如实记录，不能把 fixture 描述为在线登录成功。
- affected spec area：测试、验证报告、外部风险。
