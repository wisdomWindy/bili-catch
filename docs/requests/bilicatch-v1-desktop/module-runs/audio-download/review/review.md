# 代码评审：音频独立下载

## 交付单元标识

`audio-download`

## Blocking issues

无。AUD-AC-01..24均有可重复证据或规格允许的明确skip，最终评审未发现会破坏音频profile、fresh source、断点恢复、原子落盘、任务状态、取消确认、安全错误或响应式可用性的问题。

评审过程中发现并已解决：ETag/总长变化未可靠重置旧partial、空processed可能被finalize、失败残留partial output、启动清理可能误删resume文件、raw backend error进入UI，以及CancelRequested与迟到Completed竞争时取消意图被覆盖。每项都有直接测试或静态/视觉证据。

## Non-blocking issues

- Vite主JS为508.89 kB，超过500 kB提示线8.89 kB；当前应用仍为单窗口小型桌面工具，构建与运行通过。后续页面继续增长时可按路由拆分，不在音频模块引入无收益的异步边界。
- `http_downloader.rs` 共700行，但第318行后全部为按Range/cover场景分组的loopback测试；生产段317行。`executor.rs` 341行。二者均满足批准规格的生产职责阈值，测试夹具按允许例外保留场景内聚。

## Accepted risks

- 当前B站source合同没有证明稳定Range支持，下载器因此保守降级为单连接；`connection_count`仍由settings/runner传入并验证，但不会在未证实能力时强行分段，避免错误拼接与数据损坏。
- 仓库没有可信FFmpeg 7 sidecar，real三容器/封面smoke明确skip；production缺失时稳定返回E008/`FFMPEG_UNAVAILABLE`并保留source。打包、签名和真实smoke由 `system-release` 负责。
- public Bilibili smoke默认ignored，fixture与loopback是默认CI证据；公网接口漂移需显式opt-in复核。
- Tauri Exit设置runner shutdown flag，Tokio process启用kill-on-drop；安装包关闭、托盘及sidecar最终退出行为仍需system-release在真实bundle上验证。

## Follow-up items

- `video-download` 复用runner、fresh source、downloader、workspace与processor边界，新增video strategy，不改变已批准的audio profile、checkpoint和cancel优先语义。
- `system-release` 随包提供可信FFmpeg 7，并执行MP3/M4A/FLAC metadata/cover real smoke、资源定位、签名和安装包退出E2E。
- 若B站adapter未来能提供可信Range探测结果，可在既有ByteDownloaderPort内部启用有界并行分段；不得仅依据用户连接数假定服务端支持。

## Clean-code assessment

- result：pass
- key findings：TaskManager唯一拥有状态/attempt/终态；AudioExecutor只编排attempt；source、byte download、cover、workspace、FFmpeg、runtime controls和progress adapter各自隔离。profile/source/error规则使用typed enum与单owner函数，没有在页面复制。生产文件均不超过350行。
- required follow-up if failed：不适用。

## Design-pattern assessment

- result：pass
- key findings：Strategy只用于按mode选择执行器；Adapter只包围B站/HTTP/filesystem/FFmpeg/Tauri副作用；State/Command由TaskManager mutation承载；Observer沿用稳定task events。没有为未来provider/format建立插件框架。
- required follow-up if failed：不适用。

## Code-context structural assessment

result：pass。代码图能力仍不可用，已更新共享 `artifacts/code-context.md`，并用 `rg`、composition root、contract tests、loopback和fake恢复调用链。production路径为 `lib -> runner -> manager -> executor -> adapters -> reporter`，前端为 `page -> store/service -> typed components`。task/event/checkpoint不携带credential或时效URL，无剩余结构blocker。

## Spec-plan alignment

result：pass。AUD-01..08严格覆盖批准的AUD-AC-01..24；review回流只修复规格内竞态、安全反馈和职责阈值，没有提前实现video下载或发行打包。真实FFmpeg明确延后符合AC-23，不伪造完成。

## API integration findings

result：pass。Rust serde DTO与TS镜像保持camelCase、fixed union、nullability与十进制字符串大整数；WebView没有获得media URL、credential、filesystem、HTTP或shell能力。components不直接调用Tauri/Pinia；production composition复用同一ParserService和TaskManager Arc。

## Merge readiness summary

结论：ready。blocking issues为0；clean-code、design-pattern、spec-plan、API、structural assessment全部pass。`audio-download` 可标记completed，并提升 `video-download` 到architecture-design阶段。
