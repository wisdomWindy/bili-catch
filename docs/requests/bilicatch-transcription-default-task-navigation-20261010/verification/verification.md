# 验证报告

## Iteration 8 verification

- AC-10 pass：真实 RouterView 从 `/transcribe` 导航到 `/tasks`，等待目标页延迟初始化后，下载/转写 event source 均只订阅一次，两个 unsubscribe 均为 0。
- AC-11 pass：两个 store 在 hydration pending 与 ready 状态均复用同一运行时会话；快照与监听调用次数保持 1。
- Cleanup pass：根 App 卸载统一 dispose；subscribe 在 dispose 后返回时会被立即释放，且不会继续请求快照。
- Handoff pass：复用已就绪下载 store 时仍消费新的 drafts handoff，合并创建结果并清理匹配 handoff id。
- Packaging pass：`0.1.20` NSIS 构建退出 0；bundle 文件总数 1，MSI 文件数 0；SHA-256 `3d7e272252c9b4e8ab650a377a48b5d264282454472a0a5af5a36d3ca60b4f91`。
- Evidence：`npm test` 56 files / 218 tests；typecheck、Tauri build、cargo fmt、diff check 均 exit 0。
- self-healing loop compliance: pass.
- spec constraint compliance: pass.
- source grounding compliance: pass.
- design-pattern compliance: pass.
- user intent compliance: pass.
- change-chain integrity: pass.
- removal cleanup compliance: pass.
- expert frontend engineering compliance: pass.
- frontend architecture quality compliance: pass.
- production code quality compliance: pass.
- human review readiness compliance: pass.
- functional-programming compliance: pass.
- architecture reuse compliance: pass.
- frontend styling compliance: not applicable.
- API contract conformance: pass.
- TypeScript context compliance: pass.
- remaining blockers: none.

## v0.2.0 release evidence

- Version sources：`package.json`、`package-lock.json`、`Cargo.toml`、`Cargo.lock`、`tauri.conf.json` 均为 `0.2.0`。
- Frontend：56 files / 218 tests passed；typecheck 与 production build exit 0。
- Rust：227 passed / 1 ignored；format check exit 0。
- Windows artifact：仅 `BiliCatch_0.2.0_x64-setup.exe`，31,303,252 bytes，SHA-256 `28856127cd763d23f961892e1e93c4e656f8baa4007c12db1c6e9e4f17b34274`；MSI 文件数 0。

## 验证范围

请求 `bilicatch-transcription-default-task-navigation-20261010`，self-healing iteration 4。覆盖默认语言、设置迁移、任务页首次呈现/延迟初始化、生命周期清理、静态门禁、全量测试、浏览器行为和安装器产物。

## 验收矩阵

| AC | 证据 | 结果 |
| --- | --- | --- |
| AC-01 新设置默认 `Zh` | `missing_store_persists_complete_runtime_defaults` | pass |
| AC-02 schema v1 缺字段迁移 `Zh` 并保存 | `schema_one_is_migrated_to_schema_two_with_transcription_defaults` | pass |
| AC-03 schema v2 显式 `Auto` 保留 | `schema_two_preserves_an_explicit_auto_transcription_language` | pass |
| AC-04 转写页表单初值 `zh` | `defaults the language to simplified Chinese before settings load`；浏览器 option selected | pass |
| AC-05 页面先渲染、随后初始化两个 store | `renders before starting task initialization` | pass |
| AC-06 离页取消 pending 初始化 | `cancels pending task initialization when leaving the page` | pass |
| AC-07 测试/type/build 门禁 | 见命令证据 | pass |
| AC-08 浏览器走查 | demo 转写页“简体中文” selected；任务列表链接激活并显示目标页面 | pass |

## 命令证据

- `npm test -- src/pages/TranscribePage.test.ts src/pages/TasksPage.test.ts src/features/download-center/store.test.ts src/features/settings/store.test.ts`：4 files / 38 tests passed。
- `cargo test --test settings_manager`：14 passed。
- `npm test`：56 files / 211 tests passed。
- `npm run typecheck`：passed。
- `npm run build`：passed。
- `cargo test`：227 passed / 1 ignored。
- `cargo fmt -- --check`：passed。
- `git diff --check`：passed。
- lint：不适用，`package.json` 无 lint script。

已知非阻断警告：Vite chunk >500 kB、dialog plugin 同时静态/动态导入、两个 auth dead-code 警告、macOS `.app` identifier 建议；均为本次变更前已存在且不属于批准范围。

## 浏览器证据

- URL：`http://127.0.0.1:1422/#/transcribe?demo=1`。
- DOM：语言 combobox 中“简体中文”带 `[selected]`。
- 点击“任务列表”后：导航链接变为 active，TopBar 和页面 `h1` 均为“任务列表”。
- 时序量化采用 Vitest fake timers：mount 后两个 initialize 调用均为 0，推进 timer 后各为 1；自动化点击 API 自带等待，因此未将其墙钟时长作为性能证据。

## 安装器证据

- MSI：`BiliCatch_0.1.18_x64_en-US.msi`，42,864,640 bytes，SHA256 `92a85e22aa256592f2748354c26377721ba84befaf8bad5922817506019056a3`。
- NSIS：`BiliCatch_0.1.18_x64-setup.exe`，31,301,452 bytes，SHA256 `1eb130d60cb37e155b8bc0cdc02aa471e9a9aec633723841d26555c3cb75792f`。
- 成功命令：`npm run tauri -- build --config '{"bundle":{"createUpdaterArtifacts":false}}'`。
- updater `.sig` 未生成：当前环境没有 `TAURI_SIGNING_PRIVATE_KEY`；仅影响发布更新签名，不影响两个本地安装器。

## Change-Chain Verification

- 设置链：默认值 -> schema v1 fallback -> settings snapshot -> page watcher 完整；schema v2 direct deserialize 保留显式值。
- 导航链：RouterLink -> TasksPage render -> local timer -> existing store actions 完整。
- cleanup：pending timer clear；已启动订阅仍由 store dispose。
- 无新增/移除 API 字段、路由、UI 控件、文案或样式。

## Self-Healing Verification

- 当前迭代产生新证据：完整门禁、浏览器 DOM、安装器产物与 SHA256。
- defect inventory：none。
- 本轮修复：初始 execute 已完成；verify 无回流修复。
- remaining blockers：none。
- completion allowed：待 review 通过后 yes。

## 合规结论

- `spec constraint compliance: pass`
- `source grounding compliance: pass`
- `design-pattern compliance: pass`
- `self-healing loop compliance: pass`
- `user intent compliance: pass`
- `change-chain integrity: pass`
- `production code quality compliance: pass`
- `human review readiness compliance: pass`
- `expert frontend engineering compliance: pass`
- `field display semantics compliance: pass`
- `functional-programming compliance: pass`
- `architecture reuse compliance: pass`
- `API contract conformance: pass`
- `TypeScript context compliance: pass`
- frontend architecture quality：不适用，无架构边界变化。
- removal cleanup：不适用，无删除行为。
- frontend styling：不适用，无样式改动。

## 风险结论

默认语言和初始化时序均有确定性回归保护。剩余风险仅为发布 updater 缺少外部签名私钥；不阻断本地安装器交付，也不属于本请求代码范围。

## Iteration 7 复测证据（2026-10-11）

- RED：`does not project cached tasks before the first page frame` 在修改前失败，DOM 中首帧已出现 `P1.mp4`。
- GREEN：`src/pages/TasksPage.test.ts` 7/7 通过；缓存任务在首帧不可见，推进 frame 与 timer 后才出现。
- 全量前端：56 files / 212 tests passed。
- Rust：227 passed / 1 ignored；仅有既有 dead-code/linker warnings。
- `npm run build`：TypeScript 与 Vite production build 通过；仅有既有 dynamic-import/chunk-size warnings。
- `git diff --check`：通过。
- release 二进制 `ProductVersion` / `FileVersion`：`0.1.19`。
- MSI：`BiliCatch_0.1.19_x64_en-US.msi`，42,864,640 bytes，SHA-256 `c4a0aa524f6a2b31ae0ab539ca2341da014227c579a24a42511b8c01a4d238ad`。
- NSIS：`BiliCatch_0.1.19_x64-setup.exe`，31,284,916 bytes，SHA-256 `35c598eb83d85009a31a2c40a3349afb43a5c4b9f6330070936ee8c13fcc6cfa`。
- bundle 清理：旧 `0.1.18` MSI/NSIS 已删除，两个文件均为可重新生成的构建产物。
- 签名说明：本机构建没有 updater 私钥，安装器以 `--no-sign` 生成；不含 updater `.sig`，不影响本地安装测试。
- 浏览器导航回归：从“音频转文本”点击“任务列表”后 URL 为 `#/tasks`，任务页标题、筛选工具栏和内容区均正常呈现；开发服务器没有 Tauri IPC，因此其中的任务读取错误不作为安装版业务结果证据。
