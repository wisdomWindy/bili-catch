# 验证报告：应用基础、壳层与共享契约

## 交付单元标识

`foundation-shell-contracts`

## 验收覆盖

| 验收项 | 方法 | 结果 | 证据 | 失败跟进 | 交接状态 |
| --- | --- | --- | --- | --- | --- |
| AC-FND-01 | 检查官方配置/依赖/生成资源并构建 | pass | `package.json`、`src-tauri/`、`artifacts/code-context.md`、commands evidence | 不适用 | ready |
| AC-FND-02 | Router 单测 + 本地浏览器 hash 路由 | pass | `router.test.ts`；浏览器 download/tasks/404 交互 | 不适用 | ready |
| AC-FND-03 | 1280/900/800 三档截图检查 | pass | `verification/evidence/commands.md` 浏览器证据 | 不适用 | ready |
| AC-FND-04 | 壳层组件测试 + 浏览器导航/ARIA 树 | pass | `AppShell.test.ts`；浏览器 AX 状态 | 不适用 | ready |
| AC-FND-05 | 主题纯函数、系统事件与清理测试 | pass | `stores/app.test.ts` | 不适用 | ready |
| AC-FND-06 | locale 默认/切换与消息树同形测试 | pass | `locales/index.test.ts`、`stores/app.test.ts` | 不适用 | ready |
| AC-FND-07 | 生产源码静态扫描 | pass | `verification/evidence/commands.md` | 不适用 | ready |
| AC-FND-08 | TS transport 测试 + Rust DTO/service 测试 | pass | `client.test.ts`；cargo test 6/6 | 不适用 | ready |
| AC-FND-09 | 完整错误码与未知 reject 测试 | pass | `client.test.ts` 8/8 所在 suite | 不适用 | ready |
| AC-FND-10 | 前端与 Rust 全量测试 | pass | Vitest 29/29；Rust 6/6 | 不适用 | ready |
| AC-FND-11 | typecheck/build/fmt/check/test | pass | `verification/evidence/commands.md` | 不适用 | ready |
| AC-FND-12 | 目录、依赖方向、静态清洁性检查 | pass | code-context；invoke/color/any 扫描 | 不适用 | ready |

## Spec constraint compliance

- result: pass
- checked constraints：官方 Vue TS Tauri scaffold；Hash Router；AppShell 单一组合；页面/布局不 invoke；错误、路由 meta、导航和颜色单一来源；三态主题；双语；无伪业务；无 manager/event bus/DI container。
- evidence reference：`artifacts/code-context.md`、`execution/changelog.md`、`verification/evidence/commands.md` 与源码测试。
- follow-up if failed：不适用。

## Spec-plan 粒度对齐

result: pass。FND-01 至 FND-06 均按顺序单独推进，功能闭环与批准规格的 AC-FND-01..12 一一对应；没有提前实现解析、任务、认证、设置持久化或下载业务。

## API 契约符合性

result: pass。`get_app_info` 与 `health_check` 命令名未改写；Rust serde 输出与 TS `AppInfo`/`HealthStatus` 同字段；错误码 E001-E010/E_INTERNAL 完整；页面不消费 Rust 内部形状。

## TypeScript 上下文符合性

result: pass。实现前已读取实际 `tsconfig.json`、reference、`vite-env.d.ts` 和依赖版本；实现遵守 strict/bundler/ES2020 且未猜测 alias、ambient global 或 generated contract。

## TDD 与例外

- FND-02 至 FND-05 均记录了先失败后通过的测试证据。
- FND-01/FND-06 是 scaffold/验证任务，不适用行为 TDD；以模板原始构建、命令门禁和浏览器视觉证据替代，理由与批准计划一致。

## Summary

结果：pass。12/12 验收项有可回溯证据，规格约束、API 契约、TypeScript 上下文和 plan 粒度均通过，可交接 review。
