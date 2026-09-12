# 代码评审：解析与下载中心

## 交付单元标识

`parse-download-center`

## Blocking issues

无。AC-PARSE-01..14 均有可重复证据，最终评审未发现会破坏输入安全、稳定契约、匿名能力判断、任务草稿交接或响应式可用性的问题。

评审过程中发现并已解决两个生产阻断项：无 `Content-Length` 响应曾可能在校验前完整缓冲，现改为逐 chunk 在追加前执行 4 MiB 上限；匿名清晰度曾直接来自 `accept_quality`，现与实际 `dash.video` quality id 对照。第三轮证据审计补齐了恶意/第 6 次重定向、连续两次签名失败停止及 E004/E005/E006 映射测试。

## Non-blocking issues

- 前端与 Rust 的会话解析结果缓存当前都不设条目上限。桌面单进程、用户主动输入的 v1 范围内风险较低；若未来支持批量导入或常驻后台，应增加小型 LRU/TTL。
- B 站 metadata/nav-WBI/playurl 属外部易变接口。当前通过 private raw DTO、stable adapter、离线 fixture 和 opt-in 公网 smoke 隔离变化，但上游字段或签名规则变化仍可能造成 E001/E002/E004。

## Accepted risks

- v1 匿名解析不携带 Cookie/Authorization，因此登录专享清晰度只展示并禁选；真实登录能力归 `authentication` 模块。
- 当前目录字段为默认 `Downloads` 且不可选择；原生目录选择归 `settings`/下载执行模块，不在本交付单元伪实现。
- demo service 只在 `import.meta.env.DEV` 且显式 query 参数下启用，用于确定性截图；production/Tauri 默认路径始终使用真实 IPC。

## Follow-up items

- `task-management` 消费 `DownloadTaskDraft[]`，负责分配 id、合法状态、进度、持久化和调度；不要把这些职责回写到 download-center store。
- `authentication` 在其规格内补充登录态 IPC 与受限清晰度重新解析，不改变当前 `requiresLogin` 稳定字段含义。
- 下载执行模块按 stable draft 消费 quality/codec 或 audioFormat/audioBitrateId，继续保持模式字段互斥。

## Clean-code assessment

- result: pass
- key findings：`DownloadPage` 仅编排组件、通知与路由；Pinia store 集中状态转换、竞态和草稿规则；service 隔离 exact invoke；Rust command/service/port/client/raw/adapter/wbi 分层单向。TS 输入 guard 负责即时 UX，Rust normalizer 负责安全边界，重复是职责必要而非业务逻辑漂移。没有 TODO/FIXME、生产 `any`、散落 invoke 或调试输出。
- required follow-up if failed：不适用。

## Design-pattern assessment

- result: pass
- key findings：B 站 raw 到稳定 DTO 使用 Adapter；`BilibiliPort` 与 `Clock` 隔离外部 I/O 和时间以支持确定性测试；`main.ts`/`lib.rs` 保持轻量 Composition Root；Pinia 是页面唯一状态源。未引入 Repository、Manager、Event Bus、DI container 或无变化轴的继承结构。
- required follow-up if failed：不适用。

## Code-context structural assessment

result: pass。代码图能力与仓库 bootstrap 均不可用，已在共享 `artifacts/code-context.md` 记录探测结果，并用 `rg` 完成 import/export、command 注册与 side-effect 人工追踪。前端依赖为 composition -> page -> store/components/service，Rust 为 command -> service -> port/client/adapter -> models；网络、进程缓存与会话草稿的副作用边界明确，本模块范围无剩余结构盲点。

## Spec-plan alignment

result: pass。PARSE-01..07 与批准规格保持 function-complete 粒度，覆盖五类输入、匿名解析、WBI/cache、五段 UI、分 P/模式、草稿交接、多宽度视觉与全量门禁。三次 review/verify 回流均在已批准范围内补强正确性和证据，未提前实现任务调度、认证、持久化、目录选择、下载或 FFmpeg。

## API integration findings

result: pass。内部契约保持 `parse_video({ input }) -> Result<ParseVideoResult, AppError>`；TS exact args、Rust serde integration 与公开匿名 smoke 均通过。raw B 站字段只存在 Rust infrastructure；allowlist HTTPS 重定向最多 5 次、8/20 秒超时与 4 MiB body 上限具备直接测试；WBI 签名失败只刷新并重试一次。

## Merge readiness summary

结论：ready。blocking issues 为 0；clean-code assessment: pass；design-pattern assessment: pass。`parse-download-center` 可标记 completed，并提升 `task-management` 到 page-design 阶段。
