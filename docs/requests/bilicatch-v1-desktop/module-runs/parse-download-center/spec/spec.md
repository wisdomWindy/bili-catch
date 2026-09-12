# 工程规格：解析与下载中心

## 交付单元标识

`parse-download-center`

## 目标

交付可真实解析五类 B 站输入的 `/download` 工作台，展示稳定视频信息和媒体能力，支持分 P 与模式化参数选择，并生成可供下一任务模块消费的下载草稿。

## 范围内

- 标准/移动 URL、b23 短链、BV/AV ID、指定 P 的校验与规范化。
- Rust `parse_video` command、匿名 HTTP/WBI/缓存/adapter、稳定 DTO 与错误映射。
- 下载中心完整空/校验/解析/成功/失败/入队状态。
- 视频身份、多 P 默认/全选/反选、三种模式与条件参数。
- 会话解析缓存、WBI 12h cache 与失败后刷新重试一次。
- 生成 `DownloadTaskDraft`、成功通知并导航 `/tasks`。

## 范围外

- 下载执行、任务进度/持久化/重试、任务页实现。
- 二维码登录与 Cookie 保存；本模块只保留认证端口并默认匿名。
- 原生目录选择器；使用默认目录展示与明确禁用态。
- 音频转换、DASH 下载、FFmpeg 合并。

## 功能行为

1. 空白输入解析按钮禁用；非法输入本地失败，不发 IPC。
2. Enter、解析按钮、合法粘贴和文本拖拽进入同一 parse action；并发触发只接受最新结果。
3. 解析成功展示封面、标题、UP 主、时长、bvid 与 parts；指定 P 默认选中，否则第一 P。
4. 多 P 支持逐项、全选、反选；0 项时入队禁用。
5. video-audio/video-only 显示 quality+codec；audio-only 只显示 format+bitrate；切换时清理非法字段。
6. 可选项严格来自 ParseVideoResult；requiresLogin 项不可匿名提交，并提供登录引导。
7. 相同 canonical 输入复用会话成功缓存；清空恢复 UI 初始态但不要求清除 cache。
8. 入队生成每个选中 part 所需的单个稳定 draft，写入 task draft store，通知并跳转 `/tasks`。
9. E001-E006/E_INTERNAL 映射本地化 inline alert；E005 提供 `/login` 按钮；失败可重试和清空。
10. 健康横幅与 AppShell 始终保留，解析失败不得破坏全局导航。

## Rust 行为与安全

- `parse_video({ input }) -> Result<ParseVideoResult, AppError>`。
- URL allowlist 限制到 bilibili HTTPS 域与 b23.tv；短链最终目标再次校验，限制重定向、超时和响应大小。
- 规范化提取 aid/bvid 与正整数 page；无效/不存在映射 E003/E004。
- 通过 adapter 把 raw metadata/playurl response 转稳定 DTO；raw 字段不进入前端。
- 匿名认证端口不携带 Cookie；媒体能力以服务端实际返回为准，匿名不得伪造 1080P+ 可用。
- WBI key 缓存 12h；签名失败仅失效并重试一次。
- 技术日志不得输出 Cookie、WBI key 或敏感响应体。

## UI 与可访问性

- 遵守已批准 page-design 的五段布局和 1280/900/800 响应式行为。
- label、fieldset/legend、aria-describedby、live status、role=alert 完整；拖拽不是唯一输入。
- 解析/入队中防重复提交；loading 与错误区保持稳定，封面失败有占位。
- 仅使用现有语义 token 与 Lucide，不在 feature 散落颜色或手写 SVG。

## 架构约束

- DownloadPage 只组合 feature；URL/WBI/raw JSON 全在 Rust；Tauri invoke 仍只在现有 IPC adapter。
- 下载表单规则只在 download store；DTO 只在 feature contracts/Rust models；不重复 AppError。
- task-drafts store 只持有草稿交接，不引入任务状态机或持久化。
- 不建立 manager、event bus、repository、DI container 或外部 API 类型泄漏。

## 验收标准

- AC-PARSE-01：五类输入规范化单测通过，非法/非 allowlist 输入不发网络请求。
- AC-PARSE-02：空输入禁用；按钮/Enter/合法粘贴/拖拽共享 parse action，重复提交受控。
- AC-PARSE-03：成功结果完整展示 title/owner/cover/duration/id/parts，封面失败不破坏布局。
- AC-PARSE-04：指定 P/默认第一 P、逐项/全选/反选和 0 选择禁用均通过测试。
- AC-PARSE-05：三模式条件字段与非法组合清理正确，媒体选项只来自后端能力。
- AC-PARSE-06：匿名能力不允许提交 requiresLogin 选项；E005 可跳登录。
- AC-PARSE-07：相同 canonical 输入会话缓存命中；并发旧响应不覆盖新结果。
- AC-PARSE-08：WBI 12h TTL、失败失效、最多重试一次有 Rust 测试。
- AC-PARSE-09：raw fixture adapter 与稳定 TS/Rust DTO 字段一致，外部字段不泄漏页面。
- AC-PARSE-10：入队产生正确 DownloadTaskDraft，通知并导航 `/tasks`；不伪造任务进度。
- AC-PARSE-11：E001-E006/E_INTERNAL 的 inline error、retry/clear/login 分支可观察。
- AC-PARSE-12：1280/900/800 下输入、结果、分 P 与操作区无重叠/横向溢出，键盘与 ARIA 检查通过。
- AC-PARSE-13：前端 test/typecheck/build 与 Rust fmt/check/test 通过；opt-in 公网 smoke 无凭据时可单独报告。
- AC-PARSE-14：invoke/raw DTO/cache ownership 和跨模块 task draft 边界符合架构，无范围外业务。

## TypeScript 与 API 来源

继续使用 `artifacts/code-context.md` 的 strict/bundler/ES2020 配置和相对导入；不新增 alias。API 权威源为本模块 Rust DTO serialization tests + TS feature contracts；B 站 raw fixture 仅是 infrastructure adapter 输入，不是页面契约。

## 交接

用户批准本规格后进入 plan；计划批准后才能实现。review 通过后提升 task-management，并由其消费 DownloadTaskDraft。
