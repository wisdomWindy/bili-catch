# 代码审查：登录与认证

## 交付单元

- request：`bilicatch-v1-desktop`
- module：`authentication`
- review iteration：38
- 当前结论：PASS

## Blocking Issues

- none。

### Resolved Blocking Findings

1. nav `code=-101` 已在 raw adapter 映射为 invalid session，并由实际匿名响应形状的去敏回归测试保护。
2. stale `subscribe()` 现在自行 unlisten，只有当前 lifecycle 能取得根 store listener 所有权。
3. 退出确认已使用 `alertdialog`，pending 时遮罩/Escape 都不能关闭；安全初始焦点与归还仍通过。
4. DownloadPage 已对 `authenticated -> anonymous` 发布本地化非阻塞提示，不读取 raw code 或 credential。
5. TopBar restoring 已使用 spinner 与恢复标签，anonymous/error 仍保持 UserRound。
6. QR canvas 使用局部 render generation，旧 promise 不能覆盖当前错误态。
7. credential 补偿删除失败会推进 manager generation、abort 当前 poll 并发布 error，后台结果不能再次覆盖。

## Non-blocking Issues

- none。头像 allowlist 已在 Rust/前端共同收敛为批准的 HTTPS `hdslb.com`/`biliimg.com` 域集合。

## Accepted Risks

- 真实扫码与三平台系统凭据库运行时仍需账号/平台授权；保持 opt-in/发布阶段验证，不以 fixture 代替。
- B 站 QR/nav 是无版本承诺的外部协议；raw adapter 与去敏 fixture 继续作为漂移定位边界。

## Follow-up Items

- system-release 阶段执行 Windows/macOS/Linux 系统凭据库与启动恢复实机验证。
- 获得明确账号授权后可运行真实扫码 smoke；当前保持 ignored 是批准的非阻断边界。

## Clean-code Assessment

- `clean-code assessment: pass`
- command 只委派、manager/context/polling 职责分离、外部 I/O 与状态提交清晰；listener、QR timer/render 和 notification 副作用均有明确 owner 与生命周期。
- 没有重复 raw code、cookie、session-invalid 删除或 revision 规则；生产文件保持在批准的规模阈值内，无需额外拆分。

## Design-pattern Assessment

- `design-pattern assessment: pass`
- State enum/guard 对九状态合适；单一 Observer 解决跨页同步且 stale listener 已安全清理；Port/Adapter 只隔离 B 站、keyring、clock/sleeper/event 等真实变化边界；generation token 同时保护 Rust async commit 与局部 canvas promise。
- 没有通用 Event Bus、Repository、DI container、state class 或 speculative strategy；pattern choice justified，未过度构建。

## Spec-plan Alignment

- pass。AUTH-01..08 与 review 回流仍覆盖同一 AC-AUTH-01..16 function-complete 范围；没有提前实现 settings、下载执行器、多账号或系统发布。
- 规格中的 TopBar restoring、session-invalid 提示、alertdialog、listener cleanup 和外部 invalid 语义现均有对应实现与测试，验证工件不再存在证据漂移。

## API Integration

- pass。Rust serde DTO 继续作为跨端权威，TS 保留 camelCase/nullable 字段；四 commands 无 args，唯一事件严格为 `auth://state` + `{ snapshot }`。
- B 站数字码/字段只在 raw adapter；`-101` 的实际响应核对没有扩散到 manager 或 Vue。Cookie 只在 Rust allowlist HTTP boundary，request/service 层拥有错误归一化。
- governing `tsconfig.json` 与 `vite-env.d.ts` 已恢复；strict、bundler、无 alias/自定义 ambient 假设，类型检查通过。

## Code-context Structural Assessment

- code graph 仍不可用；沿用已记录的仓库探测，以 `rg` 和入口到副作用的人工调用链审查。
- 结构方向保持 `UI -> store/service -> Tauri -> AuthManager -> adapters`；Parser 只依赖 `AuthContextProvider`，components 只使用 props/emits，secret side effect 只在 Rust adapter。
- Review 回流扩展的 listener、跨页 notification、QR render 与 cleanup error 关系已写入 `artifacts/code-context.md`，人工调用链无剩余结构盲点。

## Merge Readiness

- merge-ready for `authentication`。blocking=0、non-blocking=0；spec constraints、API contracts、clean code、patterns、TypeScript context、security 与 visual gates 均 pass。
- 最新证据：前端 29 files/120 tests，Rust 47 unit + 28 integration，1 个明确 ignored；typecheck/build/cargo fmt/check/test、静态扫描与 7-view visual suite 全部通过。
