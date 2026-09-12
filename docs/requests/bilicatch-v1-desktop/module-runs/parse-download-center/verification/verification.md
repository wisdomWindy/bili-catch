# 验证报告：解析与下载中心

## Delivery Unit Identifier

`parse-download-center`

## Acceptance Coverage

| Acceptance item | Verification method | Result | Evidence | Follow-up if failed | Handoff status |
| --- | --- | --- | --- | --- | --- |
| AC-PARSE-01 | TS/Rust 输入表驱动测试；HTTPS/host/page 静态检查 | pass | `input.test.ts`；Rust `input.rs` tests；`evidence/commands.md` | 无 | ready |
| AC-PARSE-02 | 页面提交/paste/drop 测试；store token 与 busy 规则 | pass | `DownloadPage.test.ts`；`store.test.ts` | 无 | ready |
| AC-PARSE-03 | 结果组件测试；封面占位断言；三宽度截图 | pass | `DownloadPage.test.ts`；`bilicatch-1280.png`、`bilicatch-800.png` | 无 | ready |
| AC-PARSE-04 | requested/default、逐项、全选、反选、空选择测试 | pass | `store.test.ts`；Rust adapter 指定 P test | 无 | ready |
| AC-PARSE-05 | mode 清理、条件字段、能力来源与 draft 互斥测试 | pass | `store.test.ts`；`DownloadOptions.vue` | 无 | ready |
| AC-PARSE-06 | DASH stream quality 对照、requiresLogin 直接赋值拒绝；E005 登录路由测试 | pass | Rust adapter fixture；`store.test.ts`；`DownloadPage.test.ts` | 无 | ready |
| AC-PARSE-07 | 等价 BV ID/URL cache 与 stale promise 测试 | pass | `store.test.ts`；Rust parser result cache test | 无 | ready |
| AC-PARSE-08 | WBI 12h 边界、失效刷新、单次 retry 与连续两次失败终止测试 | pass | Rust `wbi.rs`、`parser.rs` tests | 无 | ready |
| AC-PARSE-09 | raw fixture adapter、TS/Rust serde shape、越界扫描 | pass | Rust adapter tests；`parse_contract.rs`；`evidence/commands.md` | 无 | ready |
| AC-PARSE-10 | 每个 part 一个 draft、字段互斥、通知桥与 `/tasks` 路由测试 | pass | `store.test.ts`；`DownloadPage.test.ts`；`task-drafts.test.ts` | 无 | ready |
| AC-PARSE-11 | invalid/E005/E004 inline alert、retry/clear/login 可观察测试；E004/E005/E006 API 映射测试 | pass | `DownloadPage.test.ts`；Rust `client.rs` tests；`bilicatch-error.png`；双语 locale | 无 | ready |
| AC-PARSE-12 | 1280/900/800 截图；label/fieldset/legend/live/alert/aria-describedby 测试与检查 | pass | 5 张截图；`DownloadPage.test.ts`；组件源码 | 无 | ready |
| AC-PARSE-13 | 前端 test/typecheck/build、Rust fmt/check/test、opt-in smoke | pass | `evidence/commands.md`；Rust response 上限边界 test | 无 | ready |
| AC-PARSE-14 | invoke/raw/cache/draft ownership 静态扫描与代码边界检查 | pass | `evidence/commands.md`；feature/Rust module structure | 无 | ready |

## Spec Constraint Compliance

- Result: `pass`。
- Checked constraints：五类输入与 SSRF 边界；匿名无 Cookie；raw Rust 私有化；稳定 TS/Rust DTO；12h WBI key 与单次 retry；五段 UI 与三宽度；store/service/page 副作用边界；task draft 不含 id/status/progress/persistence；目录选择、下载执行、认证持久化、FFmpeg 均未实现。
- Evidence：全部自动化门禁、静态扫描、公开匿名 smoke 和视觉截图见 `verification/evidence/commands.md`。
- Follow-up if failed：无。

## Spec-Plan Granularity Alignment

- Result: `pass`。
- PARSE-01 至 PARSE-07 按批准顺序交付；实现颗粒度与功能完整性保持一致。
- 首轮 verify 发现 failed-state clear、drop 显式测试、等价输入 cache key 与 ARIA/权限证据缺口；review 先后发现无 Content-Length 时的 body 上限不严格、匿名 quality 未对照实际 DASH stream，以及重定向/签名失败/错误映射缺少直接边界证据；均已按同一批准计划回流 execute 修复并重新全量验证。
- 未增加 task-management、authentication、settings 或下载执行模块的业务行为。

## API Contract Conformance

- 内部 command 保持 `parse_video({ input }) -> Result<ParseVideoResult, AppError>`，exact args 与 Rust serde contract tests 通过。
- B 站 raw response 仅存在 `infrastructure/bilibili/raw.rs` 与离线 fixture；Vue 只消费稳定 DTO。
- 2026-09-10 opt-in 公网 smoke 真实通过 metadata、nav-WBI、签名 playurl；无 Cookie/Authorization。
- Http client 具有 8 秒连接、20 秒总超时、4 MiB body 上限、手动 5 跳 HTTPS allowlist 重定向。

## TypeScript Context

- Result: `pass`。
- 根 `tsconfig.json` 的 ES2020、DOM、ESNext、bundler、strict、noUnusedLocals/noUnusedParameters 生效；`vue-tsc --noEmit` 通过。
- 实现使用相对导入，无猜测 alias/ambient globals；契约来自本模块 contracts、现有 AppError/IpcTransport 与 `vite/client` 声明。

## TDD Exceptions

- 可测试业务行为均有 red/green 记录和自动化覆盖。
- 响应式视觉与当前公网可用性不适合作为纯单测，分别以确定性开发 mock 截图和 ignored opt-in smoke 作为替代证据；默认回归不依赖公网。

## Workflow Handoff Readiness

- Result: `ready`。
- `useTaskDraftsStore` 已提供 session-only `DownloadTaskDraft[]`，字段不包含下游拥有的任务状态；`task-management` 可在其规格阶段定义消费、ID、状态与持久化。
- 当前模块 review 前无遗留失败或阻断项。

## Summary

`parse-download-center` 对 AC-PARSE-01..14 验证结果全部为 `pass`。规格约束、计划粒度、API 契约、TypeScript 上下文和跨模块 handoff 均通过，可进入 review。
