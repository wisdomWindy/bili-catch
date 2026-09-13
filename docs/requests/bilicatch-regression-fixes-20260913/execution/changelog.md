# 执行记录：BiliCatch 回归缺陷修复

## 执行基线

- 计划批准：2026-09-13。
- 执行方式：按 `REG-01` 至 `REG-06` 串行，遵循 red -> green -> refactor。
- 契约边界：不改变前端 TS contracts、Tauri IPC serde DTO、任务状态枚举和下载模式。
- 既有工作区改动：保留并复核 `http_downloader.rs` 的 Bilibili Referer、`video/executor.rs` 的完成态顺序及其测试。

## REG-01 认证刷新与手动 fresh parse

- 状态：完成。
- Red：前端重复解析测试显示 `parseVideo` 仅调用 1 次；Rust 同 revision 重复解析显示 `view_calls=1`，均不符合 fresh 语义。
- Green：移除 download store `resultCache` 和 `ParserService` 业务结果缓存，保留 request token 与 WBI key 缓存。
- 回归：新增 DownloadPage 登录升级测试，确认第二次 parse 后清晰度 option 解除 disabled 且移除“需登录”。
- 证据：前端相关 40 tests passed；Rust parser 6 tests passed。

## REG-02 登录页导航生命周期

- 状态：完成。
- Red：初始 authenticated 和 restoring -> authenticated 两条账户页停留测试均因旧 watcher 自动导航而失败。
- Green：增加页面内扫码启动标志；只有当前挂载周期调用 `startLogin` 后的 authenticated 才创建 800ms timer，离开/状态回退时清理。
- 回归：登录页 5 tests passed，包括有效 `from` 返回和 unmount 取消。

## REG-03 任务表格操作列边界

- 状态：实现完成，等待 REG-06 浏览器几何复核。
- 测试例外：Vitest 使用 JSDOM，无法提供可信的 grid bounding box；以静态组件测试加真实浏览器多 viewport 测量替代。
- 已确认根因：`<=1050px` 操作列 track 为 156px，但 `.task-row__actions` 强制 `min-width: 164px`。
- 实现：所有 grid 直接子项允许收缩；actions cell 使用 `min-width: 0`、`max-width: 100%` 和 cell 内换行；1050px 断点 gap 从 14px 调整为 12px，900px 可用宽度预算内不再超出。
- 回归：TaskRow/store 相关测试通过；多 viewport 实测纳入 REG-06。

## REG-04 普通音频源选择规则

- 状态：完成。
- Red：包含 65,971/85,411 bps 的匿名普通音频候选返回 E004。
- Green：移除 lossy selector 的认证 bandwidth 硬上限，继续按 tier 过滤并选择最高 bandwidth；lossless 的认证与可用性规则不变。
- 证据：audio source 5 tests passed；VideoAudio parser source test passed。

## REG-05 三模式执行与完成态一致性

- 状态：完成。
- 既有改动复核：保留 media request 的 Bilibili Referer，以及 video executor 在完成上报前完成临时区清理/最终路径检查的改动。
- 实现：audio/video executor 统一在 completed 前验证最终路径为非空文件；audio 临时区在上报前清理；VideoAudio 测试新增合流输出文件断言。
- 证据：audio executor 8 tests、video executor 3 tests、download runtime 1 test 全部通过。

## REG-06 全量验证、真实 URL smoke 与 NSIS 打包

- 状态：完成。
- 前端：Vitest 94 suites / 179 tests passed；`npm run typecheck` passed；`npm run build` passed（保留既有 500 kB chunk warning）。
- Rust：164 tests passed，常规全量中的公网 smoke 按设计 ignored；`cargo fmt --check` passed；仅保留既有 refresh token dead-code warning。
- 公网：显式运行 `public_anonymous_parse_smoke`，目标 `BV1FNb366EH2`，公开 metadata、WBI 和 playurl capability 获取成功。
- UI：Playwright DOM 几何测量 1280/900/800px；页面与每行均无横向 overflow，直接操作按钮全部为 36x36px 且位于 actions cell。
- 启动：新 release `bilicatch.exe` 启动 5 秒后仍存活且 `Responding=True`；随后结束测试实例释放文件锁。
- 打包：生成 `BiliCatch_0.1.4_x64-setup.exe`（30,244,668 bytes）；Tauri build 因当前进程未注入私钥在自动签名点退出，但 installer 已完整生成。
- 签名：使用本机 `C:\Users\yangjianlin\.tauri\bilicatch-updater.key` 通过 `tauri signer sign` 成功生成新 `.sig`（420 bytes）；未读取或输出私钥。
- Sidecar：`src-tauri/binaries/ffmpeg-x86_64-pc-windows-msvc.exe` 存在（102,856,192 bytes）。
