# 验证报告：下载任务管理

## 结论

- 验证日期：2026-09-11
- 总结论：PASS
- spec constraint compliance: pass
- spec-plan granularity alignment: pass
- API contract conformance: pass
- workflow handoff readiness: pass

## 门禁证据

| 门禁 | 结果 | 证据 |
| --- | --- | --- |
| Frontend tests | PASS | `npm test`：21 files / 87 tests，0 failed |
| TypeScript + production build | PASS | `npm run build`：`vue-tsc --noEmit` + Vite 4593 modules |
| Rust format | PASS | `cargo fmt -- --check`，exit 0 |
| Rust compile | PASS | `cargo check`，exit 0 |
| Rust tests | PASS | 17 lib passed / 1 opt-in public smoke ignored；26 integration passed |
| Visual | PASS | 1280/900/800/empty/error；document width = viewport，overflow/clipped/undersized = 0 |
| Structural scan | PASS | components 无 Tauri/Pinia；Manager 343 行，mutations 299 行，coalescing 63 行，TasksPage 91 行 |

## Acceptance Coverage

| Acceptance item | Verification method | Result | Evidence | Follow-up | Handoff |
| --- | --- | --- | --- | --- | --- |
| AC-TASK-01 | Manager + store handoff tests | PASS | idempotent atomic create；store 成功后 `clearIfMatches` | none | TS service/store |
| AC-TASK-02 | validation + UI getter/visual | PASS | 99+2 `TASK_QUEUE_FULL`；90/100 derived capacity state | none | settings may surface limits |
| AC-TASK-03 | table-driven Rust rules | PASS | invalid/control chars, fallback, CON, 200 Unicode, mode/outputDir | none | executor consumes filename |
| AC-TASK-04 | Rust/TS action matrix + manager rejection | PASS | seven statuses and processing interrupt capability | none | executor reports capability |
| AC-TASK-05 | fixed scheduler tests | PASS | default 3/8, fourth waits/releases, FIFO, validated setters | none | settings consumes setter |
| AC-TASK-06 | fixed-time retry test | PASS | 1/2/4 seconds; fourth network failure terminal; manual retry reset | none | executor reports network/terminal kind |
| AC-TASK-07 | attempt/progress mutation tests | PASS | stale attempt, percent/bytes regression, terminal invalidation, save-before-emit；1s coalescing 与 explicit/terminal flush | none | executor uses attemptId/flush |
| AC-TASK-08 | serde/event/store race tests | PASS | full task event DTO；buffer/snapshot/replay；sequence + revision guard；delete tombstone 防复活 | none | system module observes events |
| AC-TASK-09 | tempdir fault injection | PASS | restart transform, backup recovery, unknown schema, two corrupt copies no overwrite | none | app data store ready |
| AC-TASK-10 | cleaner/opener file tests | PASS | attempt invalidated; cleanup failure -> failed; completed delete keeps real file | none | executor registers temp paths |
| AC-TASK-11 | store/formatter/action/page tests | PASS | four filters/counts, active-first sort, seven visual statuses, BigInt/null/error | none | none |
| AC-TASK-12 | action-policy/row/dialog tests | PASS | allowed controls, row pending, cancel/completed/clear confirm, safe focus + return | none | none |
| AC-TASK-13 | Manager transaction tests | PASS | pause only downloading with per-item failures; clear rollback on save failure; outputs untouched | none | none |
| AC-TASK-14 | exact TS invoke + real path tests | PASS | request only taskId/target; completed/existing stored path required | none | none |
| AC-TASK-15 | page state + locale tests | PASS | skeleton, empty, load error, listener banner with row, command error/pending, locale parity | none | none |
| AC-TASK-16 | component + Playwright/CUA checks | PASS | five screenshots；本地化 progress ARIA；36px controls；dialog focus；open menu；no clipping | none | screenshots directory |
| AC-TASK-17 | full commands above | PASS | all required frontend/Rust/visual gates exit 0 | none | review |
| AC-TASK-18 | static imports + regression suite | PASS | adapters own invoke/listen; components own no store; existing DownloadPage/parse tests pass | none | downstream modules |

## 视觉证据

- `verification/screenshots/tasks-1280.png`
- `verification/screenshots/tasks-900.png`
- `verification/screenshots/tasks-800.png`
- `verification/screenshots/tasks-empty.png`
- `verification/screenshots/tasks-error.png`
- 可重复脚本：`scripts/verify-task-visuals.cjs`

## TDD 与例外

- 契约、规则、持久化、Manager、adapter、store、行组件和焦点归还均观察过预期 red 后 green。
- `TasksPage` loading/empty/error/listener 四项是 verify 阶段补充证据，未额外制造失败；它们运行真实页面/store，仅替换外部 service/event ports。
- 公网 Bilibili smoke 为既有 opt-in 测试，与本模块无关；本次 `cargo test` 保持 1 ignored，不冒充执行。

## 边界与交接

- 本模块不实现真实媒体下载/FFmpeg；`DeferredTaskExecutor::is_available=false`，因此 production 任务保持 queued。
- 后续 audio/video 模块实现 `TaskExecutorPort`，使用 `TaskExecutionSpec.attempt_id` 回报 `ExecutionUpdate`。
- settings 模块使用 `set_scheduler_limits(1..=10, 1..=10)`；system-release 可消费稳定 download events。
