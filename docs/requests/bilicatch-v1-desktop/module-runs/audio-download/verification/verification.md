# 验证报告：音频独立下载

## 结论

- 验证日期：2026-09-11
- 总结论：PASS
- spec constraint compliance：pass
- spec-plan granularity alignment：pass
- API contract conformance：pass
- workflow handoff readiness：pass（真实FFmpeg sidecar交由system-release）

## 门禁证据

| 门禁 | 结果 | 证据 |
| --- | --- | --- |
| Frontend tests | PASS | `npm run test -- --run`：45 files / 166 tests，0 failed |
| TypeScript + production build | PASS | `npm run build`：vue-tsc与Vite production build通过；4665 modules |
| Rust format | PASS | `cargo fmt --all -- --check`，exit 0 |
| Rust compile | PASS | `cargo check --all-targets -j 1`，exit 0 |
| Rust tests | PASS | 66 unit + 69 integration = 135 passed；1 public opt-in smoke ignored |
| Visual | PASS | 1280匿名浅色、1280登录浅色、800登录深色英文；overflow/clipped/undersized均为0 |
| Static security | PASS | WebView capability仅core/opener/dialog；Vue不渲染raw error；media URL、secret、stderr不进入公开合同 |
| Structural | PASS | executor生产341行；HTTP生产317行；cover/support/runtime/progress已拆分 |

## Acceptance Coverage

| Acceptance item | Verification method | Result | Evidence | Follow-up | Handoff |
| --- | --- | --- | --- | --- | --- |
| AUD-AC-01 | options纯规则与组件测试 | PASS | MP3/M4A/FLAC顺序、profile矩阵、audio-only隐藏video字段 | none | none |
| AUD-AC-02 | store/component + visual | PASS | 匿名FLAC disabled“需登录”；authenticated lossless启用 | none | none |
| AUD-AC-03 | store与executor测试 | PASS | 入队前一次M4A回退；执行期E005/E006不改profile | none | none |
| AUD-AC-04 | TS/Rust contract + task rules | PASS | 固定profile union；非法组合整批原子拒绝 | none | video复用任务合同 |
| AUD-AC-05 | audio source/executor测试 | PASS | anonymous<=64K、auth<=192K或明确lossless；320输出独立于source tier | none | none |
| AUD-AC-06 | filename/workspace测试 | PASS | 单/多P、非法字符、200字符、fallback、同名(n)不覆盖 | none | video复用workspace |
| AUD-AC-07 | runtime/task manager测试 | PASS | supported strategy、动态并发/连接数、非阻塞start；video保持queued | none | video实现新strategy |
| AUD-AC-08 | parser/source/executor + static scan | PASS | 每attempt fresh auth/view/playurl；持久化对象无cookie/media URL | none | none |
| AUD-AC-09 | loopback HTTP测试 | PASS | 206、200 fallback、416、ETag/长度重置、无Range单连接、完整性 | none | video复用downloader |
| AUD-AC-10 | downloader/task manager测试 | PASS | 流式写盘、0-90、单调bytes、unknown total、speed/ETA、旧attempt保护 | none | none |
| AUD-AC-11 | FFmpeg argv测试 | PASS | libmp3lame固定码率、M4A copy、FLAC tier guard；Command不经shell | none | none |
| AUD-AC-12 | fake runner/cover测试 | PASS | title/artist/cover argv、非空输出probe、E008分类 | real smoke deferred | system-release |
| AUD-AC-13 | executor/workspace测试 | PASS | Processing -> 非空atomic finalize -> Completed/100/outputPath，不覆盖 | none | none |
| AUD-AC-14 | downloader/executor/task UI测试 | PASS | pause刷checkpoint；新attempt恢复；processing无pause动作 | none | none |
| AUD-AC-15 | control/executor/manager竞态测试 | PASS | cancel intent优先；writer/process确认；迟到Completed转Cancelled并清理 | none | none |
| AUD-AC-16 | executor/workspace/startup测试 | PASS | success/cancel清workspace；失败保留source/checkpoint且删除processed | none | none |
| AUD-AC-17 | task manager/executor测试 | PASS | E009 1/2/4最多3次；其他错误不自动重试；stale update忽略 | none | none |
| AUD-AC-18 | locale/page tests + static scan | PASS | 中英文安全code文案；Vue不展示message/details/stderr/path/URL | none | none |
| AUD-AC-19 | URL/workspace/argv/capability扫描 | PASS | containment、redirect重验、argv array；无shell/fs/http WebView权限 | none | none |
| AUD-AC-20 | component tests + Playwright截图 | PASS | label/focus/aria-live；1280/800无溢出、重叠、裁切、小控件 | none | screenshots |
| AUD-AC-21 | 全量命令 | PASS | 前端166、Rust135、fmt/check/build全部通过 | none | review |
| AUD-AC-22 | fake端到端与fault tests | PASS | 三profile、metadata、E007/8/9、pause/resume/cancel/retry/cleanup | none | none |
| AUD-AC-23 | trusted binary inventory | PASS with explicit skip | 仓库资源无可信ffmpeg.exe；deferred adapter稳定E008/FFMPEG_UNAVAILABLE | 必须跑真实三容器smoke | system-release |
| AUD-AC-24 | clean-code/pattern review | PASS | runner/manager/executor/source/downloader/workspace/processor职责独立 | none | review |

## 视觉证据

- `verification/screenshots/audio-anonymous-1280-light.png`
- `verification/screenshots/audio-authenticated-1280-light.png`
- `verification/screenshots/audio-authenticated-800-dark-en.png`
- 可重复脚本：`scripts/verify-audio-visuals.cjs`
- 三张截图已人工检查：内容可读，无控件/文本重叠、截断或横向溢出；英文窄视图保持可操作。

## TDD 与例外

- 合同、source选择、Range恢复、workspace、FFmpeg、executor、UI和取消/完成竞态均观察过预期red后green。
- 第一次并行Rust全量测试因Windows页面文件不足（OS 1455）无法mmap编译产物；改用 `-j 1` 后全量135项通过，判定为环境资源噪声。
- public Bilibili smoke为opt-in且默认ignored；不携带凭据，也不替代fixture/loopback验证。
- 仓库资源中没有可信FFmpeg executable；真实FFmpeg 7 smoke按批准规格明确skip，system-release必须以实际随包binary重新验收。

## 边界与交接

- 音频模块已可在production composition中认领audio-only任务；video任务仍保持queued。
- HTTP下载器未把未证实的Range能力当作多连接许可，因此当前B站音频源保守使用单连接；连接数合同与范围仍由runner传递并校验。
- `system-release` 负责FFmpeg 7 sidecar、签名/资源路径、真实MP3/M4A/FLAC封面smoke及安装包退出行为。
