# 架构设计

## ADR-CLIP-01：窄端口隔离原生剪贴板

- 决策：在 download-center 域定义 `ClipboardReader` 注入端口；环境 adapter 调用 Tauri clipboard-manager；`main.ts` 组合实现。
- 原因：页面只依赖读文本语义，测试/demo 无需加载原生插件，符合既有 service/injection 结构。
- 拒绝：直接在组件 import 插件会耦合测试与原生环境；自定义 Rust command 重复官方插件；后台轮询扩大隐私和性能成本。

## Ownership 与依赖方向

| Owner | 职责 |
| --- | --- |
| `clipboard.ts` | `ClipboardReader` 类型与 Tauri adapter |
| `injection.ts` | key、无操作 fallback、消费 hook |
| `DownloadPage.vue` | mount/focus 触发、去重、竞态和卸载、调用既有校验与 submit |
| `main.ts` | demo/production 依赖选择和 provide |
| Tauri composition | 插件注册与只读 capability |

允许 `DownloadPage -> input/injection/store`、`main -> adapter/injection`；禁止 adapter 反向依赖页面/store，禁止 store 读取剪贴板。无循环依赖。

## 状态拓扑与数据生命周期

系统剪贴板 -> `ClipboardReader.readText` -> 页面局部 raw text/读取 revision -> `normalizeParseInput` -> `submit(normalized)` -> Pinia input/status/result。页面卸载使 revision 失效并移除 focus listener。

## 复用与复杂度

复用候选只有既有 `normalizeParseInput`，同一领域且语义完全相同，直接复用。paste/drop 是事件携带文本，不与主动读取生命周期合并。复杂度预算：1 个窄 adapter 文件、1 个 injection key、页面内局部编排、2 个依赖项、1 个 capability；不引入 composable/class/manager。

## 回滚与清理

回滚面为移除页面监听、provide、adapter、插件注册/权限和两端依赖。若未来多个页面需要相同生命周期，再评估抽取 composable；当前仅一个生产调用方，保持页面就近编排。
