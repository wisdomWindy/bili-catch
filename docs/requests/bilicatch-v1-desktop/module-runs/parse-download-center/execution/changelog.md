# 执行记录：解析与下载中心

## 执行上下文

- 规格批准：2026-09-10，用户回复“批准规格”。
- 计划批准：2026-09-10，用户回复“批准计划”。
- 执行模式：PARSE-01 至 PARSE-07 严格串行，不启用并行 agent/workflow。
- TypeScript：根 `tsconfig.json`，strict、ES2020、bundler，无 alias/ambient 自定义类型；继续使用相对导入。
- Rust：stable 1.98.1，Tauri 2；双端契约由 TS feature contracts 与 Rust serde tests 共同证明。

## 任务记录

### PARSE-01

- 状态：completed
- TDD：先添加 exact command/args、稳定 DTO 与 task draft shape 测试，再实现 contracts/service/models。
- Red：前端 2 个 suite 因 service/draft store 缺失失败；Rust integration test 因 parse DTO 未导出失败。
- Green：前端 2 文件/3 测试通过，Rust parse contract 1/1 通过，TypeScript typecheck 通过。
- 结果：稳定 TS/Rust DTO、exact `parse_video` request service 与最小 task draft handoff store 完成。

### PARSE-02

- 状态：completed
- TDD：先覆盖五类输入、trim/内部空白、HTTPS、伪子域和 page 边界，再实现权威 Rust normalizer 与 UI guard。
- Red：前端 guard 测试因 input 模块缺失失败；Rust normalizer 测试在实现前无法解析对应导出。
- Green：前端 12/12 输入测试通过；Rust 2/2 输入测试通过；TypeScript typecheck 通过。
- 结果：支持 BV/AV、标准/移动端/B23 输入与指定 P；HTTPS/host/page 边界和内部空白均在双端受控，Rust 保持权威校验。

### PARSE-03

- 状态：completed
- 范围：离线 raw fixture/adapter、WBI 签名与 12 小时 key cache、有限重定向、`parse_video` 命令与 Tauri state。
- Red：新增模块入口后，Cargo 因 adapter/client/raw/wbi 文件缺失失败；依赖下载首次受沙箱网络限制，获批后完成。
- Green：Rust 13 个单元测试与 1 个集成测试通过，`cargo check` 和 `cargo fmt --check` 通过。
- 结果：真实 metadata/nav-WBI/playurl client、4 MiB 响应上限、8/20 秒超时、5 跳 allowlist 重定向、12 小时 WBI key、单次签名重试与会话结果缓存完成。

### PARSE-04

- 状态：completed
- TDD：覆盖默认能力选择、登录能力跳过、指定 P、全选/反选、空选择、模式互斥、竞态丢弃、缓存和 clear。
- Green：下载中心定向前端测试 19/19 通过，TypeScript typecheck 通过。
- 结果：Pinia 成为唯一解析状态源，页面层无需理解 IPC 或 B 站 raw 数据。

### PARSE-05

- 状态：completed
- 结果：输入、视频摘要、分 P、下载选项、操作栏五个组件与响应式工作台完成；Enter/paste/invalid/result/ARIA 组件测试通过。

### PARSE-06

- 状态：completed
- 结果：每个选中分 P 生成一个互斥字段正确的 `DownloadTaskDraft`，写入会话 store，通知后导航 `/tasks`；未引入 id/status/progress/persistence。

### PARSE-07

- 状态：completed
- Verify 回流：逐条映射 AC 时发现失败态 clear、drop 显式测试、等价 BV 输入 cache key 与部分 ARIA/权限证据不足；均属于批准计划内，回流 execute 修复。
- 回流修复：新增 canonical input cache key、失败态 clear、drop/E005/requiresLogin/cover/aria-describedby 测试；定向 26/26 与 typecheck 通过，重新进入 verify。
- Green：前端 12 文件/58 测试、typecheck/build、Rust fmt/check、13 lib + 1 integration tests 全部通过；ignored 公网 smoke 单独执行 1/1 通过。
- 视觉：1280x800、900x700、800x650、800 宽完整滚动区与 900x700 error 共 5 张截图通过，无重叠或横向溢出。

## 偏差与决策

- 无批准规格或架构偏差。
- Review 回流：HTTP response 在无 `Content-Length` 时先完整缓冲再校验 4 MiB，不满足严格 body 上限；作为 security blocker 回流 execute，改为逐 chunk 限制。
- 回流修复：response 改为 `chunk()` 逐块读取并在追加前限制累计 4 MiB；新增边界测试，Rust 14 lib + 1 integration 通过。
- Review 二次回流：adapter 将全部 `accept_quality` 标为匿名可用，未与实际 `dash.video` stream quality 对照；存在提交匿名不可用清晰度风险，回流 execute 修复。
- 回流修复：raw video stream 增加 quality id；adapter 仅把实际 DASH stream 清晰度标为匿名可用，其余响应内清晰度标记 requiresLogin；离线测试与公网 smoke 通过。
- Review 证据回流：批准计划要求的第 6 次重定向拒绝与连续两次 WBI 签名失败尚无直接测试；回流 execute 补齐边界证据。
- 回流修复：提取纯重定向目标校验，直接覆盖恶意域名与第 6 次跳转拒绝；签名 mock 支持拒绝前 N 次，证明两次失败后以 E002 终止且不再重试；补齐 E004/E005/E006 API 映射断言。
- 最终 Green：前端 12 文件/58 测试及 production build 通过；Rust fmt/check、17 lib + 1 integration tests 通过，1 个 opt-in 公网 smoke 单独执行通过。
