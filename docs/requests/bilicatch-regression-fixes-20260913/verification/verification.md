# 验证报告：BiliCatch 回归缺陷修复

## 交付单元标识

`bilicatch-regression-fixes-20260913`

## 验收覆盖

| 验收项 | 验证方法 | 结果 | 证据引用 | 失败跟进 | 交接状态 |
| --- | --- | --- | --- | --- | --- |
| REG-AC-01 | DownloadPage + store 测试模拟匿名解析后 auth upgrade | pass | `DownloadPage.test.ts` 登录能力刷新；store auth refresh 调用 2 次 | 无 | ready |
| REG-AC-02 | LoginPage fake timer 覆盖初始 authenticated、restoring、当前页扫码成功 | pass | LoginPage 5 tests passed；已登录路径 801ms 后仍为 `/login` | 无 | ready |
| REG-AC-03 | 同输入和等价 URL 连续手动解析 | pass | store 测试断言 `parseVideo` 调用 2 次、第二 result/part 替换；parser 同 revision 重新 fetch | 无 | ready |
| REG-AC-04 | Playwright 在 1280/900/800px 读取 DOM 几何 | pass | 三种 viewport 均 `pageOverflow=false`、所有 `rowOverflow=false`、直接按钮 36x36 且 `controlsWithinCell=true` | 无 | ready |
| REG-AC-05 | Rust selector fixture 使用 65,971/85,411 bps 普通 lossy 候选 | pass | audio source 5 tests passed，匿名选择 85,411 bps 候选；lossless/URL guards 仍通过 | 无 | ready |
| REG-AC-06 | executor/parser/runtime 专项测试 | pass | AudioOnly/VideoAudio/VideoOnly resolver 与 mux 边界；audio executor 8、video executor 3、runtime 1 tests passed | 无 | ready |
| REG-AC-07 | workspace + executor 最终输出断言 | pass | audio/video finalization 拒绝空文件；VideoOnly/VideoAudio/AudioOnly completed 路径为非空文件 | 无 | ready |
| REG-AC-08 | 全量门禁 | pass | Vitest 94 suites/179 tests；Rust 164 tests；`npm run typecheck`、`npm run build`、`cargo fmt --check` passed | 无 | ready |
| REG-AC-09 | 目标 BV 公网 opt-in smoke、release 启动、NSIS/签名/sidecar 检查 | pass | `BV1FNb366EH2` live smoke passed；进程 5s 后 Responding；exe 30,244,668 bytes；sig 420 bytes；FFmpeg 102,856,192 bytes | 无 | ready |

## Spec Constraint Compliance

- 结果：`pass`。
- 责任边界：页面只编排；fresh/选择状态在 store；Bilibili raw 仍由 Rust adapter 隔离；源选择在 selector；I/O 在 executor/infrastructure。
- 契约：`ParseVideoResult`、task draft、auth event 和 IPC serde DTO 字段均未变化。
- 模式：保留 Pinia command、auth Observer、executor Strategy registry；未引入新 manager/factory。
- 副作用：未新增 Vue 网络请求、持久 Cookie 或跨层文件操作。
- 安全：普通 lossy 放宽不影响 FLAC/HiRes 登录限制；媒体类型、HTTPS/CDN 和非空输出校验继续生效。
- 证据：执行 changelog、全量 tests、typecheck/build、联网 smoke 与产物检查。
- 失败跟进：无。

## Spec/Plan 粒度对齐

- 结果：`pass`。
- 6 个计划任务均已完成，未新增下载模式、登录方式、转码需求、推送或 Release 发布。
- 5 个用户缺陷均落到对应任务和验收项，没有把外部公网稳定性当作确定性单元测试。
- `REG-03` 的 JSDOM 几何限制已在执行记录中明确，替代证据为真实 Playwright viewport 测量，符合计划约定。

## API 与 TypeScript 契约符合性

- 结果：`pass`。
- governing `tsconfig.json` 为 strict、ES2020、bundler resolution、noEmit；正确使用 `vue-tsc --noEmit` 验证 Vue SFC。
- 前端继续复用现有 TS contracts；Rust command/serde DTO 为 IPC 权威来源；外部 Bilibili 字段未泄漏到前端。
- `npm run typecheck` 通过，无 guessed alias、ambient 或 generated type 依赖。

## TDD 与例外

- REG-01、REG-02、REG-04 均留下先失败后通过的明确 red/green 证据。
- REG-05 纳入前序未提交的 downloader/executor 修复并强化非空最终文件回归。
- REG-03 因 JSDOM 不实现布局，未伪造 bounding box 单测；用组件测试和 1280/900/800px Playwright DOM 几何替代。

## 总结

- 总体结果：`pass`。
- 阻断项：0。
- 外部限制：Bilibili 公网与账号高清能力仍可能随服务端变化；已用 fixture 固定业务规则并单独记录 live smoke。
- 构建说明：Tauri bundle 在未自动注入私钥时于签名步骤返回非零，但 NSIS 已成功生成；随后通过同一 Tauri signer 和本机私钥成功生成新 `.sig`，最终产物完整。
- Review handoff：ready。
