# 验证命令证据

日期：2026-09-10

## 自动化门禁

- `npm test`：12 个 test files，58 个 tests，全部通过。
- `npm run build`：`vue-tsc --noEmit` 与 Vite production build 通过；产物 JS 423.59 kB（gzip 133.22 kB），CSS 11.98 kB（gzip 3.03 kB）。
- `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`：通过。
- `cargo check --manifest-path src-tauri/Cargo.toml`：通过。
- `cargo test --manifest-path src-tauri/Cargo.toml`：Rust lib 17 passed、1 ignored；integration 1 passed；doc tests 通过。直接覆盖 4 MiB body 边界、恶意/第 6 次重定向拒绝、连续两次签名失败停止，以及 E004/E005/E006 映射。
- `cargo test --manifest-path src-tauri/Cargo.toml public_anonymous_parse_smoke -- --ignored --nocapture`：1 passed；公开匿名 metadata/nav-WBI/playurl 链路成功，不携带 Cookie。

## 静态边界扫描

- 扫描前端 feature/page 的 `invoke(`、`accept_quality`、`wbi_img`、`dash.video`、`any`、hex：无匹配。
- 扫描解析实现的 `Cookie`、`Authorization`、`println!`、`dbg!`、`console.`：无匹配。
- Tauri invoke 仅由 `src/features/download-center/service.ts` 经现有 `IpcTransport` 发起；页面没有直接 invoke。

## 视觉证据

- `bilicatch-1280.png`：1280x800，完整操作区，无重叠或横向溢出。
- `bilicatch-900.png`：900x700，壳层收窄、配置区进入纵向滚动，无横向溢出。
- `bilicatch-800.png`：800x650，标题自然换行，输入与分 P 区无重叠。
- `bilicatch-800-tall.png`：800 宽加高视图，验证滚动后的设置与操作栏完整。
- `bilicatch-error.png`：900x700，失败输入保留，retry/clear 同行无重叠。

截图由本机 Edge headless 访问 `http://127.0.0.1:1420/#/download?demo=1&autoparse=1` 的显式开发 mock 生成；生产与 Tauri 默认路径仍调用真实 IPC。
