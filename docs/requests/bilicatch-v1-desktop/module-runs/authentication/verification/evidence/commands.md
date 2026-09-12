# 登录与认证验证命令证据

日期：2026-09-11

## 自动化门禁

- `npm test -- --run`：29 个 test files，120 个 tests，全部通过。
- `npm run typecheck`：`vue-tsc --noEmit` 通过；严格模式、noUnused、ES2020/DOM、bundler resolution 下无错误。
- `npm run build`：TypeScript 检查与 Vite production build 通过；4633 modules，JS 481.38 kB（gzip 152.38 kB），CSS 23.44 kB（gzip 5.04 kB）。
- `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`：exit 0。
- `cargo check --manifest-path src-tauri/Cargo.toml`：exit 0，无 Rust code warning。
- `cargo test --manifest-path src-tauri/Cargo.toml`：47 lib passed、1 opt-in public smoke ignored；28 integration passed；0 failed。
- `node scripts/verify-auth-visuals.cjs`：7 个视图全部通过；截图写入 `../screenshots/`。

## 静态边界

- public TS、Rust model/command 扫描 Cookie/token/qrcode_key/Set-Cookie：无匹配。
- page/component/auth production code 扫描直接 `invoke(`/`listen(`：无匹配。
- auth scope 扫描 localStorage/sessionStorage/persist/console/Rust debug log：无匹配。
- Tauri capabilities 扫描 stronghold/store:keyring/credential/secret：无匹配。
- auth manager/infrastructure 扫描 `lock().await` 与 plaintext/file fallback：只命中两条明确否定 fallback 的测试名称，无生产命中。
- auth/bilibili production modules 扫描 dead-code/unused-import allow：无匹配。

## 视觉指标

- 视口：1280x800、900x700、800x650。
- 状态：requesting、waiting_scan、waiting_confirm、expired、error、authenticated。
- 主题：light 与 dark。
- 所有 case：documentWidth 等于 viewportWidth；clippedRegions=0；overlappingRegions=0；undersizedButtons=0；qrTextLeaked=false。
- waiting/confirm：canvas 224x224，21126 个非白像素，QR stage 为 `rgb(255, 255, 255)`。

## 明确例外

- `public_anonymous_parse_smoke` 需要显式公网 opt-in，本轮保持 ignored；没有真实账号授权，因此未执行真实扫码或 credential 写入。fixture/demo 未作为在线成功证据。

## Review 回流增量证据

- 2026-09-11 对 `https://api.bilibili.com/x/web-interface/nav` 的匿名只读响应确认：顶层 `code=-101`，同时 `data.isLogin=false`；新增 adapter regression 后明确映射 invalid session。
- 首轮 RED：前端 4 files/4 failures；Rust raw adapter 2 failures；QR late promise 1 failure；credential cleanup generation 1 failure。
- 定向 GREEN：前端 5 files/31 tests、Rust auth 12 tests、raw adapter 7 tests 全过；随后执行上述全量门禁。
- 静态复扫：public secret、direct transport、Web Storage/log、Tauri secret capability、`lock().await`、dead-code allow 均无命中。
- Cargo test 的唯一 warning 是 Windows MSVC linker 创建 import library 的工具链提示；`cargo check` 无代码 warning。
