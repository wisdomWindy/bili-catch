# Execution Changelog

## 2026-09-14 - T1 恢复根 Provider 高度链

### Implemented Changes

- `src/app/App.vue`：为真实 `NConfigProvider` 添加 `app-provider` 类。
- `src/styles/base.css`：为 `.app-provider` 添加 `height: 100%` 与 `min-height: 0`。
- `src/app/App.test.ts`：增加真实 Provider DOM 的根类回归断言。

### TDD Evidence

- RED：`npm test -- src/app/App.test.ts` 退出码 1；测试明确报告无法在真实 `.n-config-provider` 内找到 `.app-provider`。
- GREEN：相同命令退出码 0；1 个测试文件、1 个测试通过。
- Refactor：无需重构；生产实现保持一个模板类和一个两属性 CSS 规则。

### Browser Layout Evidence

- 环境：本地 Vite demo，Edge headless，视口 `900x500`。
- `.app-provider`：computed height `500px`，computed min-height `0px`，clientHeight `500`。
- `.content-scroll`：clientHeight `444`，scrollHeight `1252`，overflowY `auto`。
- 将滚动位置设为末尾后：scrollTop `808`，等于 maxScrollTop `808`。

### Deviations

- 无规格或计划偏差。
- Playwright 包未附带 Chromium 二进制，因此使用机器现有 Edge 可执行文件；未下载或安装新浏览器。

### Clean-code And Pattern Notes

- 布局类使用应用语义 `app-provider`，未耦合第三方内部类名。
- 未新增抽象、依赖、运行时分支或副作用。
