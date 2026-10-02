# 执行变更记录

## TDD 记录

### RED

- 新增 `download_progress_does_not_regress_when_the_known_total_expands`。
- 旧实现实际返回 `29`，预期为 `60`，测试按预期失败。
- 新增 `adding_audio_total_does_not_cancel_a_valid_video_attempt`。
- 旧实现使真实 `DownloadControl` 进入 `CancelRequested`，预期为 `Running`，测试按预期失败。

### GREEN

- 在 `download_percent` 的有效总量分支中，将原始折算比例与 `current.min(90)` 取最大值。
- 两条定向回归测试转绿。

### REFACTOR

- 未引入新抽象；仅运行 rustfmt 规范化测试排版。

## 实现摘要

- 生产代码仅修改共享下载百分比纯函数的一处返回表达式。
- 任务管理器的单调校验、reporter 拒绝语义和取消控制逻辑保持不变。
- 测试使用缺陷 BV 的已验证流大小，不包含签名 URL、cookie 或其他凭证。

## 验证快照

- 定向计算测试：通过。
- 定向双轨控制器行为测试：通过。
- 完整 `cargo test`：180 passed，1 ignored。
- 首次 `cargo fmt --check`：仅发现新增测试排版差异；已运行 `cargo fmt`，待 verify 阶段复查。

## 计划偏差

无行为或范围偏差。格式门禁按计划在发现排版差异后通过标准格式化修正。

## 契约、迁移、观测与安全

- 契约：无变更。
- 迁移：不适用。
- 观测：沿用任务进度与状态，无新增指标。
- auth/authz：不适用。
- secret/credential/PII：不适用，测试未固化敏感数据。
- abuse/replay：不适用。
- rollout/rollback：随常规桌面版本发布；回滚为恢复纯函数表达式，无数据动作。
