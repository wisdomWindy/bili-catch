# 任务板：系统集成与发布

## 执行规则

- 依赖顺序：SR-01 -> SR-02/SR-03 -> SR-04 -> SR-05 -> SR-06。
- 状态域：`pending | in_progress | completed | blocked`。
- 当前阶段为计划审批；批准后一次只推进一个实现任务。
- 可信 sidecar、签名材料和目标平台 runner 未提供前，SR-04/05/06 的真实发布 smoke 只能 deferred。

| ID | 任务 | 状态 | 模式 | 前置 |
| --- | --- | --- | --- | --- |
| SR-01 | 系统生命周期、托盘与窗口行为 | completed | 串行 | 计划批准 |
| SR-02 | 通知、文件对话框与拖拽边界 | completed | 可与 SR-03 并行 | SR-01 |
| SR-03 | Updater 状态机与签名校验 | completed | 可与 SR-02 并行 | SR-01 |
| SR-04 | FFmpeg externalBin 与发布清单 | in_progress | 串行 | SR-03 |
| SR-05 | 三平台 bundler 与签名产物 | pending | 串行 | SR-04 |
| SR-06 | 全量验证、发布审查与交接 | pending | 串行 | SR-01..05 |

## Windows 发布决策

- GitHub Releases 固定产出未签名 NSIS `.exe`，不以 Windows Authenticode 证书作为前置条件。
- 发布页必须披露 SmartScreen “未知发布者”提示以及企业设备可能阻止安装的限制。
- `TAURI_SIGNING_PRIVATE_KEY` 仍是 updater 产物签名的必要输入，不属于 Windows 代码签名。

## 入口与出口

- 入口：`state.json.stage=execute` 且 `system-release.approvals.plan_approved=true`。
- 出口：verification/review 记录 SR-AC-01..11；外部材料缺失时记录明确 deferred，不伪造发布通过。
