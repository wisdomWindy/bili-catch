# 验证报告：登录与认证

## 交付单元

- request：`bilicatch-v1-desktop`
- module：`authentication`
- 验证日期：2026-09-11
- 总结论：PASS
- spec constraint compliance：pass
- spec-plan granularity alignment：pass
- API contract conformance：pass
- workflow handoff readiness：pass，可进入 review

## 门禁证据

| 门禁 | 结果 | 证据 |
| --- | --- | --- |
| Frontend tests | PASS | 29 files / 120 tests / 0 failed |
| TypeScript context | PASS | 根 `tsconfig.json`：strict、noUnused、ES2020+DOM、bundler、无 alias；`vue-tsc --noEmit` 通过 |
| Production build | PASS | Vite 4633 modules；JS 481.38 kB，CSS 23.44 kB |
| Rust format/compile | PASS | Cargo fmt/check exit 0，无 code warning |
| Rust tests | PASS | 47 unit + 28 integration passed；1 opt-in public smoke ignored |
| Security scans | PASS | public secret、direct transport、storage/log/capability/dead-code 扫描无生产命中 |
| Visual | PASS | 7 张截图；1280/900/800、light/dark、6 状态；overflow/clip/overlap/undersized=0 |
| QR pixel | PASS | 224x224、21126 非白像素、白底、DOM 无 QR URL |

完整命令与扫描结果见 `verification/evidence/commands.md`；截图见 `verification/screenshots/`。

## Acceptance Coverage

| Acceptance item | Verification method | Result | Evidence | Follow-up | Handoff |
| --- | --- | --- | --- | --- | --- |
| AC-AUTH-01 | auth store buffer/revision/stale-listener tests + root composition + TopBar test | PASS | subscribe-before-snapshot、倒序/重复忽略、stale subscribe 自行 unlisten、未进 LoginPage 也显示 account | none | root auth store |
| AC-AUTH-02 | LoginPage/QR tests + 1280 visual/pixels | PASS | anonymous 自动 start；requesting 同尺寸；waiting canvas 224、180s clamp/status | none | LoginPage |
| AC-AUTH-03 | Rust adapter/manager fake-time tests | PASS | 86101/86090/0/86038 映射；2s 边界；180s 且不超过90次 poll | none | AuthManager |
| AC-AUTH-04 | generation/cancel/late-result/cleanup tests + unmount test | PASS | cancel 先推进 generation；迟到 confirmed 不 save；cleanup failure 推进 generation 并 abort 当前 poll | none | AuthManager/LoginPage |
| AC-AUTH-05 | Cookie/store/manager transaction tests | PASS | allowlist+必填 Cookie、nav valid、save-before-auth、失败无 authenticated event | none | auth adapters |
| AC-AUTH-06 | serde exact tests + TS contracts + secret/log scans | PASS | public event 仅 snapshot；StoredCredential 无 Serialize/Debug；无 secret 公共命中 | none | stable IPC DTO |
| AC-AUTH-07 | restore/invalid/transient manager + live-shape adapter tests | PASS | not-found anonymous；valid account；`-101`/isLogin=false invalid delete；transient 保留 credential/error | none | startup restore |
| AC-AUTH-08 | logout transaction/store/dialog tests | PASS | delete failure 保持 authenticated/account；pending finally；alertdialog、安全焦点与归还 | none | logout action |
| AC-AUTH-09 | parser cache/auth tests | PASS | cache hit 前仍 validate；revision 分隔；transient 在 media request 前短路 | none | ParserService |
| AC-AUTH-10 | invalid context + parser/DownloadPage/TopBar regression | PASS | invalid emit anonymous；authenticated -> anonymous 本地化提示；匿名 ParseVideoResult/E005 保留 | none | download flow |
| AC-AUTH-11 | exact service/event/Rust compile tests + import scan | PASS | 四个无参 command；唯一 `auth://state` payload/unlisten；组件无 invoke/listen | none | Tauri bridge |
| AC-AUTH-12 | 8 状态表驱动 + late-render component test + visual matrix | PASS | live region 无倒计时；pending 防重复；迟到 canvas failure 不覆盖当前 auth error | none | QR panel |
| AC-AUTH-13 | account/avatar/page route tests | PASS | fallback/UID/host/error；800ms 有效来源与 direct `/download`；authenticated visual | none | account page |
| AC-AUTH-14 | TopBar state test + headless Edge geometry/pixel suite | PASS | restoring spinner/标签；1280/900/800、light/dark、error/expired/authenticated；无裁切/重叠 | none | responsive UI |
| AC-AUTH-15 | keyring fake-fault/production construction + capability scan | PASS | get/set/delete/not-found/platform fault；v1 fixed key；无 plaintext/WebView fallback | none | system keyring |
| AC-AUTH-16 | full gates + explicit ignored record | PASS | 本报告门禁全绿；真实扫码未授权且明确 ignored，不以 demo 代替 | none | review |

## Spec Constraint Compliance

- 结果：pass。
- Rust 是认证事实源；公开 snapshot/event 不含 secret，Cookie/refresh/qrcode_key 只存在于私有 Rust owner。
- 二维码 polling 为 2 秒/180 秒 monotonic deadline；generation 隔离迟到 async 结果；安全存储成功前不提交 authenticated。
- 每次 parse 在结果 cache lookup 前远端校验；cache key 含 auth revision；瞬时失败不删凭据，明确失效才删除。
- 前端只有根 auth store 订阅唯一事件；service/events 独占 transport；页面与组件不直接调用 Tauri。
- QR 原文只进入 canvas renderer，不进入 DOM 文本/log；头像仅允许 HTTPS `hdslb.com`/`biliimg.com` 域。
- 未扩大 Tauri secret capability；未引入明文、sample store、TTL auth cache、通用 Event Bus、Repository 或 DI container。
- 证据：`execution/changelog.md`、`verification/evidence/commands.md`、Rust/TS tests、7 张视觉截图。

## Granularity And Contracts

- spec-plan 粒度对齐：pass。AUTH-01..08 与 AC-AUTH-01..16 保持同一 function-complete 范围，没有实现 settings、多账号、下载执行器或发布能力。
- API contract：pass。Rust serde 是权威；TS 保留 camelCase 与 nullable 字段；四 command 无 args；事件严格为 `auth://state` + `{ snapshot }`。
- backend handling：pass。Bilibili generate/poll/nav raw code 被 adapter 隔离；固定 HTTPS allowlist、redirect disabled、8s connect/20s total、4MiB body guard。
- TypeScript context：pass。实现使用根 tsconfig 与直接相对导入，无猜测 alias/ambient/generated contract；相关 qrcode/Vite 声明已纳入现有编译闭包。

## TDD And Failures

- AUTH-01..07 的可测试产品行为均记录可信 red -> green；具体失败点见 execution changelog。
- verify 补测 9 项（八个 QR 公共状态与 direct-login fallback）用于增强已有行为证据，未制造虚假 red；作为验证补测明确记录。
- 视觉回流发现英文 adapter message，新增本地化错误 red 后修复并重跑 7-view suite。
- ignored 仅为需要公网的 anonymous smoke；真实扫码/keyring runtime 未获账号授权，不作为阻断，也未伪造成功。

## Review Re-entry Verification

- 结果：PASS；原 review 的 7 个 blocking issues 与 1 个 allowlist non-blocking issue均已修复并有直接回归证据。
- 外部 contract：匿名 nav 实际响应只读核对为 `code=-101` + `isLogin=false`，数字码仍只存在 raw adapter。
- Observer ownership：异步 listener 只有在 lifecycle 当前时才提交给 store；stale listener 立即 unlisten。
- UI/a11y：alertdialog、pending Escape、restoring spinner、session-invalid info、QR late promise 均按批准语义实现。
- credential transaction：补偿删除失败会推进 generation 并 abort 当前 poll，错误态不能被后台提交覆盖。
- TDD：回流新增 8 个可信失败并逐项转绿；全量门禁更新为 frontend 120、Rust 47+28。
- workflow handoff readiness：pass，可重新进入 review。

## Summary

登录与认证模块通过全部批准验收项、自动化门禁、安全边界与视觉矩阵。verification artifact 完整，可进入 review；后续模块只能消费公开 auth snapshot/parser context，不得取得 credential。
