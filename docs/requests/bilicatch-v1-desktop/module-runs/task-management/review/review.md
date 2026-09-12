# 代码评审：下载任务管理

## 交付单元标识

`task-management`

## Blocking issues

无。AC-TASK-01..18 均有可重复证据，最终评审未发现会破坏任务状态、持久化恢复、事件顺序、可信文件打开或响应式可用性的问题。

评审过程中发现并已解决：持久化失败污染内存、损坏 primary 覆盖健康 backup、普通进度未合并、延迟 event 覆盖较新 command revision、删除任务被旧 progress 复活，以及完成态打开命令未按规格归入同一菜单。修复均有直接回归测试或浏览器证据。

## Non-blocking issues

- Manager 343 行、mutations 299 行，已超过“约 250 行进入职责审查”阈值但未超过 350 行硬限制；两者分别保持命令/调度和 executor mutation 单一职责，当前再拆会增加跨文件事务跳转，暂不拆分。
- Tauri event adapter 当前记录不了前端是否成功消费 emit；前端通过启动快照与 sequence 恢复最终一致性。若未来需要后台长期下载，可增加可观测计数而不改变事件契约。

## Accepted risks

- 本模块只交付任务宿主，不实现 HTTP Range、B 站媒体流或 FFmpeg；`DeferredTaskExecutor::is_available=false`，生产任务保持 queued，真实执行由 audio/video 模块接入。
- 为同时遵守 persist-before-emit 与“普通进度最多每秒落盘”，后端普通进度事件同样最多每秒一次，低于 UI 250ms 上限；窗口内最新值在 explicit/terminal flush 提交，崩溃前不足一秒进度可丢，状态变化不可丢。
- 公网 Bilibili smoke 仍为既有 opt-in ignored 测试，不属于本模块完成证据。

## Follow-up items

- `audio-download` 与 `video-download` 实现 `TaskExecutorPort`，每次回报携带当前 `attempt_id`，结束、暂停或异常路径调用状态更新/flush。
- `settings` 使用已有 1..=10 scheduler setters 暴露最大并发与单任务连接数，不复制调度规则。
- `system-release` 可消费稳定 `download://progress` / `download://removed` 契约做系统集成检查。

## Clean-code assessment

- result: pass
- key findings：页面仅编排；Pinia 拥有前端集合与竞态；Rust manager 拥有权威状态和事务；JSON、emit、cleaner、opener、executor 分别位于明确边界。状态规则无散落副本，生产范围无 `any`、TODO/FIXME 或调试输出。超过 250 行的两个 Rust 文件经职责审查仍内聚，均低于 350 行硬限制。
- required follow-up if failed：不适用。

## Design-pattern assessment

- result: pass
- key findings：State/Strategy 由纯 action matrix 与 mutation rules 承载；Observer 仅跨 Tauri 两个稳定事件并具备 unsubscribe/sequence/buffer；Port/Adapter 只隔离 store、event、cleaner、executor 等真实变化轴；Composition Root 保持在 `main.ts`/`lib.rs`。ProgressCoalescer 解决已批准的写盘/通知频率问题，没有扩展为通用调度框架。
- required follow-up if failed：不适用。

## Code-context structural assessment

result: pass。代码图能力仍不可用，已更新共享 `artifacts/code-context.md` 并用 `rg`、入口读取、契约测试及故障注入恢复依赖图。前端为 composition -> page -> store/service/events -> components；Rust 为 command -> manager/rules -> ports/adapters/store。文件、JSON、事件和 opener 副作用集中，无跨层隐式写入。

## Spec-plan alignment

result: pass。TASK-01..09 保持批准规格的 function-complete 粒度，覆盖双端 DTO、状态矩阵、100 容量、3/8 调度、重试、恢复、进度合并、六 commands、两 events、四筛选、确认/菜单及五视图。review 回流只修复规格内正确性与可访问性偏差，未提前实现认证、真实下载、FFmpeg 或设置页。

## API integration findings

result: pass。Rust serde DTO 是权威 contract source；TS 在明确跨语言边界保留 camelCase 字段、枚举、nullability 与十进制字符串大整数。六个 invoke 使用 exact nested `{ request }` 或无参形式；组件不接触 transport；可信打开命令只接受 task id/target，路径来自 Rust 已持久化完成任务。

## Merge readiness summary

结论：ready。blocking issues 为 0；clean-code assessment: pass；design-pattern assessment: pass；spec-plan/API/structural assessment 全部 pass。`task-management` 可标记 completed，并提升 `authentication` 到 page-design 阶段。
