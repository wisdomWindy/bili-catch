# 验证报告

## 验收映射

| 验收项 | 证据 | 结果 |
| --- | --- | --- |
| 多模型组可收起/展开 | `ModelManager.test.ts`，含默认展开、收起、恢复断言 | 通过 |
| 单模型组无折叠入口且始终可见 | 同一组件测试含动态缩减到单模型场景 | 通过 |
| 404 后切换备用地址 | Rust 测试 `retries_a_backup_url_after_the_primary_source_is_not_found` | 通过 |
| 所有地址尝试后才报错 | Rust 测试 `reports_an_error_only_after_every_source_url_has_failed` | 通过 |
| 既有行为无回归 | 前端全量 222 项；Rust 240 项通过、2 项忽略 | 通过 |

## 命令证据

- `npm test -- --run`：56 个测试文件、222 项测试全部通过。
- `npm run build`：`vue-tsc --noEmit` 与 Vite 生产构建通过；保留既有动态导入和包体积提示。
- `cargo test --manifest-path src-tauri/Cargo.toml`：240 通过、2 忽略。
- `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`：通过。
- `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets`：0 错误、11 个既有警告。
- `git diff --check`：通过。

## 限制与安全

- 纯浏览器预览可打开弹窗，但无法调用 Tauri 模型仓库，实际有数据状态由组件测试验证。
- auth/authz：不适用；未改变认证或权限边界。
- secret/PII：不适用；未新增数据采集、日志或外传。
- abuse/replay：不适用；候选 URL 每个只尝试一次，没有无限重试。

规格约束符合性: 通过

安全验证: 不适用

spec-plan 粒度对齐: 通过
