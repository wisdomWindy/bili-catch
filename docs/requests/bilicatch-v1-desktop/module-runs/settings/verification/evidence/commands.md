# Settings 验证命令证据

执行日期：2026-09-11（Asia/Shanghai）

| 命令/检查 | 结果 |
| --- | --- |
| `rtk npm test -- --run` | PASS；43 个测试文件，156 个测试通过 |
| `rtk npm run build` | PASS；`vue-tsc --noEmit` 与 Vite production build 通过 |
| `rtk C:/Users/yangjianlin/.cargo/bin/cargo.exe fmt --manifest-path src-tauri/Cargo.toml -- --check` | PASS |
| `rtk C:/Users/yangjianlin/.cargo/bin/cargo.exe check --manifest-path src-tauri/Cargo.toml` | PASS |
| `rtk C:/Users/yangjianlin/.cargo/bin/cargo.exe test --manifest-path src-tauri/Cargo.toml --all-targets` | PASS；49 unit + 36 integration 通过，1 个需显式联网的 smoke ignored |
| Tauri import 扫描 | PASS；生产代码仅 `features/settings/dialog.ts` 在 adapter 内动态导入 dialog |
| `scheduler_config|set_scheduler_limits` 扫描 | PASS；无命中 |
| `console.|println!|dbg!` 扫描 | PASS；Settings 生产范围无命中 |
| capability 读取 | PASS；仅 `core:default`、`opener:default`、`dialog:allow-open` |
| release URL/权限扫描 | PASS；无 updater/store/shell WebView 权限，无 endpoint/license URL |

## 浏览器与界面证据

- 本地演示地址：`http://127.0.0.1:1422/#/settings?demo=1`。
- 真实应用内浏览器可访问性树确认 4 个 section、11 个持久化控件、3 个 About 行、2 个 switch、2 个 range，以及全部 label/id/status 关系。
- 约 533px 窄屏面板中，表单按 `<760px` 单列呈现；路径、滑块、选择框和按钮没有横向溢出或相互遮挡。
- 真实交互确认 `system -> light` 立即切换主题并出现“已保存”；`zh-CN -> en-US` 立即切换全部可见文案并出现 `Saved`。
- 首屏与中段截图已在本次 Codex 浏览器验收中观察；系统输入法悬浮条属于桌面覆盖层，不计入应用布局。
- 外部宽屏浏览器当时检测到用户输入，自动化按安全规则停止。900/800/1280 证据采用已批准断点 CSS 静态核查、production build 和页面/组件测试；未伪造持久化截图文件。

## 已知非阻断输出

- Vite 报告主 JS chunk 505.63 kB（gzip 159.28 kB），超过 500 kB 提示阈值；构建成功。该提示来自现有整体依赖体积，不影响 Settings 正确性，记录为后续性能优化项。
- Windows linker 输出 DLL import library 创建信息；测试成功，无 linker failure。
