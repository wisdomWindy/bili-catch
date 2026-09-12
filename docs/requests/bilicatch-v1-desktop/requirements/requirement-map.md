# 需求拆分总览

## 请求摘要

将 BiliCatch V1.0 拆为 8 个顺序交付模块。每个模块独立完成设计、规格、计划、实现、验证和评审，后续模块只能消费已通过评审的前序契约。

## 需求分析承接

- 范围固定为 Tauri v2 桌面应用、四个路由页面、七类业务能力及打包更新。
- 非目标保持不变：直播、弹幕、字幕、编辑、播放列表导入和绕过内容保护均不进入实现。
- 外部 B 站接口、FFmpeg 二进制和发布凭据通过 adapter/config 边界承接，不阻塞模块拆分。
- 共享任务模型先于音视频执行；系统集成最后消费任务、设置和认证状态。
- 删除任务只删记录、更新安装需用户确认、核心 UI 提供中英文作为非阻塞默认口径。

## 拆分原则

- 以稳定职责、共享页面归属和依赖顺序拆分，而不是按文件或技术层机械拆分。
- `/download` 的解析与参数选择保持一个页面模块；音频与视频执行分别作为后端能力模块。
- 任务模型与任务页放在同一模块，成为后续下载执行和系统托盘的稳定状态源。
- 全局路由、主题基线、IPC 错误和共享类型先建立，避免模块间重复定义。

## 来源清单

- 主来源：`biliCatch_PRD.md`
- 需求细化：`analysis/requirement-analysis.md`
- 结构化索引：`artifacts/prd-snapshot.md`

## Markdown 归一化说明

原始 PRD 已是标准 Markdown，表格、代码块、流程和标题均可稳定表达，不需要格式转换。各模块文件仍同时保留“来源快照”和“Markdown 归一化快照”；后者只做跨章节合并与线性重排，不改变业务语义。原始内容始终保留在 `biliCatch_PRD.md`，来源行段在模块中列明。

## 模块清单

| 顺序 | ID | 名称 | 类型 | 来源章节 | 页面设计 | 架构设计 | 下游产物根 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 1 | `foundation-shell-contracts` | 应用基础、壳层与共享契约 | cross-module rule/page | 2、3、4.0、5.5、6 | 是 | 是 | `module-runs/foundation-shell-contracts/` |
| 2 | `parse-download-center` | 解析与下载中心 | page/form workflow | 3.2、4.0、模块一 | 是 | 是 | `module-runs/parse-download-center/` |
| 3 | `task-management` | 下载任务管理 | table/list workflow | 模块四、5.1 | 是 | 是 | `module-runs/task-management/` |
| 4 | `authentication` | 登录与认证 | page/interaction flow | 模块五、1.4、5.4 | 是 | 是 | `module-runs/authentication/` |
| 5 | `settings` | 设置管理 | page/form workflow | 模块六、6.3 | 是 | 是 | `module-runs/settings/` |
| 6 | `audio-download` | 音频独立下载 | interaction flow | 模块二 | 否 | 是 | `module-runs/audio-download/` |
| 7 | `video-download` | 视频下载与合并 | interaction flow | 模块三 | 否 | 是 | `module-runs/video-download/` |
| 8 | `system-release` | 系统集成与发布 | cross-module rule | 模块七、6.4、7、8 | 否 | 是 | `module-runs/system-release/` |

## 模块依赖与顺序说明

- `foundation-shell-contracts` 提供工程、导航、主题、错误和 IPC 边界，是全部模块前置。
- `parse-download-center` 稳定解析结果与任务创建输入。
- `task-management` 稳定任务状态、事件和持久化，音视频执行以此为宿主。
- `authentication` 与 `settings` 分别提供能力上下文和运行参数。
- `audio-download`、`video-download` 在任务与设置契约稳定后接入实际执行。
- `system-release` 最后连接托盘、通知、窗口、更新、sidecar 和构建产物。

## 顺序执行队列

严格顺序：`foundation-shell-contracts -> parse-download-center -> task-management -> authentication -> settings -> audio-download -> video-download -> system-release`。

第一模块从 `page-design` 开始。所有模块都需要架构设计；前五个包含可见页面或应用壳，需要页面设计。任何后续模块不得在前一模块通过 `review` 前开始。

## 页面设计候选

- 应用基础：全局壳层、侧栏、顶部栏、内容区、主题与响应式窗口行为。
- 下载中心：输入、结果、分 P、模式化参数和操作区。
- 任务管理：筛选、任务行、进度、密集操作和空/错状态。
- 登录：二维码状态机和已登录状态。
- 设置：分组表单、范围控件、即时保存状态。

## Spec 约束传递清单

| 模块 | 必须保留 |
| --- | --- |
| 基础 | 技术栈、Hash 路由、全局导航、Pinia/i18n、E001-E010、Tauri IPC Result、主题基线 |
| 解析 | 五类输入、空值禁用、自动解析、加载/错误、分 P 默认/全选/反选、模式条件显示、缓存、入队跳转 |
| 任务 | 100 项上限、状态字段、筛选、状态相关操作、事件合并、暂停全部、清除完成、恢复 |
| 认证 | 五个二维码状态、180 秒、Cookie 加密、启动恢复、退出清除、解析前校验、匿名可用 |
| 设置 | 四个分组、全部字段/范围/枚举/默认值、即时保存、目录选择、即时主题/语言 |
| 音频 | MP3/M4A/FLAC、码率规则、登录限制、转换阶段、元数据、失败重试 |
| 视频 | 视频+音频/仅视频、编码与清晰度、并行 DASH、FFmpeg 无损封装、E008/E009 |
| 系统发布 | 托盘菜单、通知场景、窗口与对话框、拖拽、更新签名、externalBin、目标包与发布清单 |

## 开放问题与未决歧义

- 真实 B 站接口字段、更新端点、公钥、FFmpeg 二进制和签名凭据继续作为外部接口/发布依赖。
- 删除记录不删除输出文件，自动更新安装需用户确认，除非后续用户明确修改。
- 性能目标需要在可复现环境记录，网络型目标不作为无条件完成保证。

