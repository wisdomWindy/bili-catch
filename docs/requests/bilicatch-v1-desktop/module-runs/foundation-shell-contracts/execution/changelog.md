# 执行记录：应用基础、壳层与共享契约

## 执行上下文

- 计划批准：2026-09-10，用户回复“继续”。
- 执行模式：严格串行，不启用并行 agent/workflow。
- 初始工具探测：Node `v24.19.0`、npm `11.17.0` 可用；`cargo`、`rustc` 不在当前 PATH。
- 当前已知限制：Rust/Tauri 编译验证需要后续恢复 Rust 工具链；先继续官方 scaffold 与前端实施，不把缺失工具链误报为通过。

## 任务记录

### FND-01

- 状态：completed
- TDD：不适用。该任务是官方脚手架与环境上下文恢复，不包含业务行为。
- 结果：官方 `create-tauri-app` 4.7.4 Vue + TypeScript/Tauri 2 模板生成成功，批准依赖安装成功，原始模板 `npm run build` 通过。
- 上下文：实际 TypeScript、Vite、Cargo、Tauri 与 capability 配置已写入 `artifacts/code-context.md`。
- 恢复：生成器 `--force` 意外覆盖 PRD/docs；已从本次会话日志原文与成功补丁完整恢复并校验，生命周期仍保持 execute。

### FND-02

- 状态：completed
- TDD：先建立前端错误归一、transport 命令，以及 Rust 序列化/服务测试，再实现契约。
- Red：`client.test.ts` 因 contracts 尚不存在而按预期失败。
- Green：`npm test -- --run src/services/ipc/client.test.ts` 通过，1 个文件、8 个测试；`npm run typecheck` 通过。
- 边界：生产 `@tauri-apps/api/core` 导入仅存在于 `src/services/ipc/client.ts`。
- Rust：DTO、AppError、service、两个薄 command 及序列化测试已实现；当前 PATH 无 cargo，测试执行留待 FND-06 环境门禁。

### FND-03

- 状态：completed
- TDD：先覆盖 root redirect、四路由/404、route meta、三态主题和 locale 消息闭包。
- Red：3 个 suite 均因 router/store/locales 尚不存在而按预期失败。
- Green：3 个文件、11 个测试通过；`npm run typecheck` 通过。
- 结果：Hash Router（生产默认）、路由标题 meta、三态主题与清理、中英双语消息树已建立。

### FND-04

- 状态：completed
- TDD：先锁定导航顺序、ARIA 当前项、品牌/登录/设置/返回交互与壳层结构。
- Red：suite 因 AppShell 尚不存在而按预期失败。
- Green：3 个壳层组件测试通过；首次 Green 检查发现测试未等待异步路由且使用了无效 `get().exists()`，修正测试基线后全绿；类型检查通过。
- 结果：Sidebar、TopBar、AppIconButton、语义 token 与 208/72/60px 响应式布局完成；视觉像素检查留在 FND-06。

### FND-05

- 状态：completed
- TDD：覆盖健康初始化/错误/重试、路由页内容与 404 返回入口。
- Red：7 个测试因临时空路由和缺失 initialize action 按预期失败。
- Green：2 个文件、7 个测试通过；类型检查通过。
- 边界：健康状态只经 AppIpcService；页面、布局与 store 均未导入 `@tauri-apps/api/core`。

### FND-06

- 状态：completed
- 前端：`npm test` 通过（7 文件、29 测试）；`npm run build` 通过（vue-tsc + Vite production build）。
- Rust：通过 Rust 官方 rustup 安装 stable 1.98.1 与 rustfmt；`cargo fmt --all -- --check`、`cargo check`、`cargo test` 通过，6 个 Rust 测试全绿。
- 静态边界：production `invoke` 仅存在于 IPC adapter；十六进制颜色仅存在于 token 文件；业务源码无 `any`/localStorage。
- 视觉：浏览器实测 1280x800、900x700、800x650；完整/72px/60px 侧栏切换正确，无文字重叠；800px 隐藏冗余设置入口。
- 交互：实测 download -> tasks -> back、未知路由 -> 404 -> download 均正确；后端不可达横幅不阻断导航。
- 修复：视觉交互发现返回按钮未订阅路由变化，增加 route dependency 后真实 Hash Router 复验通过；标题改为 BiliCatch。

## 偏差与决策

- scaffold 解析到 Vite 8.2.2、Vue Router 5、Pinia 4 等当前兼容版本；按已批准规格保留，不按 PRD 示例强制降级。
- 脚手架覆盖事故属于执行偏差，已恢复全部生命周期资料；后续不再对非空根目录运行强制生成。
- Rust 工具链缺失是环境门禁，不改变批准的工程结构。
