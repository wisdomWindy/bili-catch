# 代码评审：应用基础、壳层与共享契约

## 交付单元标识

`foundation-shell-contracts`

## Blocking issues

无。verify 的 12 条验收项均有证据，评审未发现会破坏正确性、边界或后续模块接入的问题。

## Non-blocking issues

- `styles/layout.css` 为 265 行，略高于约 250 行的观察阈值，但内容仍是单一壳层布局职责，当前拆分会降低导航与响应式规则的可读连续性，因此不要求修改。
- Naive UI 使首包约 402KB（未压缩）；本模块仅安装全局 provider，当前 gzip 约 126KB，不构成功能或桌面分发 blocker。后续业务模块可按真实测量再决定按需优化。

## Accepted risks

- 品牌标记与 Tauri 图标仍为可替换占位资源，符合批准规格的范围外声明。
- 浏览器预览无法提供真实 Tauri runtime，因此会显示错误横幅；真实 Rust DTO/service/command 核心契约由 cargo check 与 6 个 Rust 测试覆盖。
- 首模块不提供主题/语言设置 UI 与持久化；store、运行时同步和双语消息基础已完成，UI/持久化归属后续 settings 模块。

## Follow-up items

- `parse-download-center` 只替换 DownloadPage 内部，不改写 AppShell、router meta、IPC transport 或错误契约。
- settings 模块复用 `ThemePreference`、`AppLocale` 与 store action，并负责持久化，不新增重复枚举。
- system-release 模块替换最终品牌/安装图标并执行原生打包验证。

## Clean-code assessment

- result: pass
- key findings：页面、布局、store、IPC adapter 与 Rust command/service 职责清晰；路由、导航、错误码和颜色均为单一来源；production 无 `any`、localStorage、TODO 或散落 invoke；没有三类职责混入同一文件。
- required follow-up if failed：不适用。

## Design-pattern assessment

- result: pass
- key findings：IpcTransport Adapter 对应 Tauri/测试双环境这一真实变化轴；main.ts 与 lib.rs 的轻量 Composition Root 有明确组装职责；未引入 Repository、DI container、Event Bus、Factory 或继承层次。
- required follow-up if failed：不适用。

## Code-context structural assessment

result: pass。该模块从绿地 scaffold 建立，依赖方向可由直接 import/模块扫描完整判断，无需构造代码图。TypeScript 实现前已恢复实际 tsconfig/reference/声明来源，没有猜测 alias 或 ambient global。Rust models 为公共契约，commands -> services -> models 方向稳定；前端 pages/components -> store/contracts -> services/ipc 边界符合架构设计。

## Spec-plan alignment

result: pass。FND-01..06 与批准规格保持 function-complete 粒度：bootstrap、双端契约、router/store/i18n、壳层、健康/占位集成、门禁逐项完成，未提前实现七个后续业务模块。

## API integration findings

result: pass。command 名、请求无参数、响应字段与错误码均保持批准契约；Rust serde 与 TS contracts 有双端测试；`@tauri-apps/api/core` 只有 transport adapter 导入，页面/组件/store 不接触 Tauri reject 原始形状。

## Merge readiness summary

结论：ready。blocking issues 为 0；clean-code assessment: pass；design-pattern assessment: pass。当前模块可标记 completed，并提升 `parse-download-center` 到其 page-design 阶段。
