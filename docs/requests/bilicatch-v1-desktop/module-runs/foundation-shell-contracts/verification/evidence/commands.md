# 验证命令证据

验证日期：2026-09-10。

| 门禁 | 结果 | 摘要 |
| --- | --- | --- |
| `npm test` | pass | Vitest 7 files / 29 tests passed |
| `npm run build` | pass | vue-tsc 无错误；Vite 8.2.2 production build 成功 |
| `cargo fmt --all -- --check` | pass | rustfmt 无差异 |
| `cargo check` | pass | `bilicatch` dev profile 完成，无代码警告 |
| `cargo test` | pass | Rust 6 passed / 0 failed；main/doc tests 0 failed |
| IPC 边界扫描 | pass | production `@tauri-apps/api/core` 仅在 `src/services/ipc/client.ts` |
| 清洁性扫描 | pass | `any`/localStorage 无命中；十六进制颜色仅在 `styles/tokens.css` |

## 浏览器证据

- 本地 URL：`http://127.0.0.1:1420/#/download`。
- 1280x800：208px 完整侧栏、56px 顶栏、健康错误横幅与内容区无重叠。
- 900x700：72px 图标侧栏，tooltip/可访问名称保留。
- 800x650：60px 图标轨道，顶栏冗余设置按钮隐藏，登录入口与标题无冲突。
- 真实交互：下载中心 -> 任务列表 -> 返回；未知 hash -> 404 -> 返回下载中心。
- 浏览器环境不提供 Tauri runtime，因此健康检查按设计归一为错误横幅；Rust command 成功形状由 6 个实际 Rust 测试验证。
