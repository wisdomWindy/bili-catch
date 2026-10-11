# `audio-transcription` 代码评审

## 结论

- 阻断问题：无。
- 验收建议：通过，进入完成状态。

## 已修复问题

- 修复模型下载完成响应仍携带旧 revision、可能覆盖较新事件的问题；后端返回最终 revision，前端 store 同时拒绝较旧命令响应，并增加竞态回归测试。
- 修复联合任务投影把 `0..1` 转写进度当作 `0..100` 使用的问题。
- 将 whisper 进度回调接入任务 manager，补齐已处理秒数、速度和 ETA。
- 补齐生产 B 站字幕 adapter、失败回退、本地终态通知、自动转写创建失败记录、结果导出和重新转写入口。

## 评审范围

- 正确性：状态机、取消、缓存、导出 revision、模型下载原子性、事件顺序、任务联合动作。
- 安全与隐私：模型源 HTTPS、字幕 URL allowlist、cookie 不落日志、媒体无上传实现、诊断输出限长。
- 可维护性：下载与转写 manager/持久化保持隔离；Tauri command 无业务分支；前端组件通过 service/store 消费合同。
- 分发：固定 sidecar 版本、许可、externalBin/resource 映射、Windows NSIS 产物与跨平台目标名。

## 非阻断项

- Vite 产物约 583 kB，触发 500 kB chunk 警告；不影响模块功能，可在后续性能迭代拆包。
- 既有 auth 模块有 2 个 dead-code warning；与本模块无关。
- 既有 auth 取消测试在并发全量运行中观测到一次时序 flake；目标用例与串行全量复跑均通过，认证代码无本次改动。
- 正式 updater 签名和 macOS/Linux 实机包属于发布环境门禁，详见 verification 报告。
