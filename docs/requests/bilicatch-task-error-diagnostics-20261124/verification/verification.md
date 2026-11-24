# 验证：任务页错误提示与进度订阅恢复

## 自动化

| 检查 | 结果 |
| --- | --- |
| `pnpm run typecheck` | 通过 |
| `pnpm run test` | 58 个文件 / 244 个用例通过 |
| `pnpm run build` | 通过（Vite built in 1.13s） |
| `cargo test --lib` | 144 passed，1 ignored |
| `cargo clippy --all-targets` | 通过，无 error |

新增/更新用例：

- `retries a failed progress subscription before surfacing an error`：首次订阅失败、重试成功，`eventError` 保持为空。
- `falls back to polling and stops it on dispose when the stream stays unavailable`：三次失败后 `eventError.code === "E021"`、`pollTimer` 存在，`dispose` 后清空。
- `exposes the complete stable error-code set`：错误码集合加入 E019–E021。
- `keeps snapshot rows visible when event subscription fails`：订阅失败时快照行仍可见，横幅文案为本地化 E021，不泄漏原始错误。

## 手工验证

- 本地未构造出可稳定复现的订阅失败场景；已通过单元测试覆盖首次失败重试、持续失败轮询与清理。
- 0.4.0 发布由 GitHub Actions 构建，Windows 仅 NSIS。

## 未覆盖

- 真实 IPC 在启动瞬间失败的时序无法在 jsdom 中完整复现。
- `E019`/`E020` 的端到端点击验证未单独截图。
