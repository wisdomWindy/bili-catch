# 任务看板：BiliCatch 回归缺陷修复

## 执行顺序

全部任务已按 `REG-01 -> REG-02 -> REG-03 -> REG-04 -> REG-05 -> REG-06` 串行完成，等待 verify/review。

| 任务 ID | 任务名称 | 状态 | 执行模式 | 执行人或执行说明 |
| --- | --- | --- | --- | --- |
| REG-01 | 认证刷新与手动 fresh parse | completed | 串行 | 已移除前后端业务结果缓存并补齐 store/page/parser 回归测试 |
| REG-02 | 登录页导航生命周期 | completed | 串行 | 已将返回 timer 绑定到当前页面实际启动的扫码流程 |
| REG-03 | 任务表格操作列边界 | completed | 串行 | 已消除操作 cell 的强制最小宽度并修正 900px 临界布局预算 |
| REG-04 | 普通音频源选择规则 | completed | 串行 | 已允许 Bilibili 返回的普通有损音轨并保留 lossless 安全边界 |
| REG-05 | 三模式执行与完成态一致性 | completed | 串行 | 三模式调用边界、Referer 和最终非空文件规则已实现并通过专项测试 |
| REG-06 | 全量验证、真实 URL smoke 与 NSIS 打包 | completed | 串行 | 自动化、联网 smoke、多 viewport、启动与签名产物证据均已收集 |

## REG-01 任务卡

- 触发/前置条件：规格批准；读取 `tsconfig.json`、auth/download contracts、当前 diff。
- 关联规格区域：认证后能力刷新、手动重新解析；`REG-AC-01`、`REG-AC-03`、`REG-AC-08`。
- 覆盖功能单元：下载 URL 表单、auth watcher、download store、Rust parser business cache。
- 页面/模块/容器范围：`DownloadPage`、download-center store、parser service 及测试。
- 整洁性约束或注意点：页面只编排；store 拥有状态；parser 拥有 fresh 用例语义；保留 token 防竞态。
- 模式约束或抽象边界：沿用 Pinia command/Observer，不新增缓存管理器。
- 代码上下文或影响面备注：输入展示原值，校验按现有 trim/normalizer；不改 DTO。
- 关键交互与状态约束：每次按钮/Enter 立即 parsing；最后 token 才提交；成功重建能力，失败清旧结果。
- API 契约来源与类型策略：Rust `parse_video` serde DTO + 现有 TS contract；字段名和错误码不变。
- 测试切入点：重复输入调用次数、第二结果替换、auth upgrade fresh、过期响应、无输入不请求。
- 待确认项或已批准假设：已批准假设为每次显式解析绕过业务结果缓存，WBI key 缓存保留。

## REG-02 任务卡

- 触发/前置条件：REG-01 完成；登录页可观察当前挂载周期是否启动扫码。
- 关联规格区域：登录页返回；`REG-AC-02`、`REG-AC-08`。
- 覆盖功能单元：账户信息展示、扫码登录成功反馈、来源路由返回、timer/unmount。
- 页面/模块/容器范围：`LoginPage.vue` 与其测试。
- 整洁性约束或注意点：timer 仅在页面生命周期拥有；离开时清理。
- 模式约束或抽象边界：不新增全局导航服务；使用现有 auth store 和 router。
- 代码上下文或影响面备注：修订现有“authenticated 即返回”的测试语义。
- 关键交互与状态约束：初始/恢复 authenticated 稳定停留；仅本页扫码成功 800ms 后返回。
- API 契约来源与类型策略：现有 `AuthStateEvent`/auth snapshot；无 IPC 变更。
- 测试切入点：fake timer、direct visit、valid from、restoring、unmount 清理。
- 待确认项或已批准假设：已批准保留现有 800ms 成功反馈和默认 `/download` 回退。

## REG-03 任务卡

- 触发/前置条件：REG-02 完成；确认 actions 最大按钮组合和 More overlay。
- 关联规格区域：任务表格操作列；`REG-AC-04`、`REG-AC-08`。
- 覆盖功能单元：表头、任务行、操作单元格、按钮组、1050/850px 响应式规则。
- 页面/模块/容器范围：任务管理列表、`TaskRow`、task-management CSS 和测试。
- 整洁性约束或注意点：只修 grid/min-width/wrap，不改变业务操作或隐藏按钮。
- 模式约束或抽象边界：CSS 负责布局；组件 DOM 保持语义；overlay 不被裁切。
- 代码上下文或影响面备注：操作按钮稳定 36x36px；header/row 列模板必须一致。
- 关键交互与状态约束：active/completed/failed 的按钮均留在 cell；窄屏无横向滚动。
- API 契约来源与类型策略：现有 task view model；无 API/类型变更。
- 测试切入点：1280/900/800px bounding boxes、scrollWidth、不同按钮组合、More 菜单。
- 待确认项或已批准假设：已批准空间不足时可在所属单元格内换行。

## REG-04 任务卡

- 触发/前置条件：REG-03 完成；用 raw adapter fixture 固定 65,971/85,411 bps 候选。
- 关联规格区域：三种下载模式的音频源选择；`REG-AC-05`、`REG-AC-06`、`REG-AC-08`。
- 覆盖功能单元：audio selector、VideoAudio 音轨解析、AudioOnly 源解析。
- 页面/模块/容器范围：Rust audio/video services、parser adapter 边界及 tests。
- 整洁性约束或注意点：过滤与排序只在 `select_audio_source`；不在 executor 重复。
- 模式约束或抽象边界：沿用现有 source strategy；不增加 resolver/factory。
- 代码上下文或影响面备注：普通有损候选可选；FLAC/HiRes、空 URL 仍排除。
- 关键交互与状态约束：只有候选真为空才 E004；选择最高可用普通源且结果确定。
- API 契约来源与类型策略：Bilibili raw DTO -> adapter -> domain source；IPC 不变。
- 测试切入点：匿名 65,971/85,411、空候选、无效 URL、HiRes/FLAC、模式 resolver 次数。
- 待确认项或已批准假设：已批准以 Bilibili 返回普通候选为权限边界，不执行 64,000 bps 硬截止。

## REG-05 任务卡

- 触发/前置条件：REG-04 完成；FFmpeg sidecar 可用；先审阅工作区既有 downloader/executor diff。
- 关联规格区域：三种下载模式与最终文件校验；`REG-AC-06`、`REG-AC-07`、`REG-AC-08`。
- 覆盖功能单元：executor registry、video/audio executor、downloader header、FFmpeg、完成态。
- 页面/模块/容器范围：Rust services/infrastructure 与 integration tests。
- 整洁性约束或注意点：I/O 仅在 executor/infrastructure；完成态只在最终校验后写入。
- 模式约束或抽象边界：沿用 Strategy registry；禁止跨模式猜测或静默降级。
- 代码上下文或影响面备注：保留既有 Bilibili `Referer` 与最终 outputPath 修复，不覆盖用户改动。
- 关键交互与状态约束：VideoOnly 无 mux；VideoAudio 双轨 mux；AudioOnly 无视频 resolver；失败不 completed。
- API 契约来源与类型策略：现有 task draft、executor trait、registry；DTO/状态枚举不变。
- 测试切入点：调用次数、header、mux/提取失败、零字节、最终文件非空、取消路径。
- 待确认项或已批准假设：已批准最终非空文件是 completed 的必要条件。

## REG-06 任务卡

- 触发/前置条件：REG-01 至 REG-05 全部完成；本地网络、sidecar、签名环境可用。
- 关联规格区域：完整验收；`REG-AC-01` 至 `REG-AC-09`。
- 覆盖功能单元：专项/全量自动化、多 viewport、真实 URL、三模式 smoke、NSIS/updater 签名。
- 页面/模块/容器范围：整个前端/Rust 工程及 Tauri bundle 输出。
- 整洁性约束或注意点：验证失败返回对应任务修复；不为通过门禁降低断言。
- 模式约束或抽象边界：自动化确定性证据与公网 smoke 分开记录。
- 代码上下文或影响面备注：不提交 Cookie，不推送代码，不发布 GitHub Release。
- 关键交互与状态约束：检查 parsing、账户页停留、三模式 task state、退出后登录清除。
- API 契约来源与类型策略：使用实际 IPC/task event/file output；不新增契约。
- 测试切入点：Vitest/typecheck/build、Cargo tests、1280/900/800、完整 URL、NSIS、`.sig`、启动检查。
- 待确认项或已批准假设：真实登录高清能力和公网稳定性属于外部条件；失败需记录证据而非伪造通过。

## 验收映射

| 验收标准 | 主任务 | 验证任务 |
| --- | --- | --- |
| REG-AC-01 | REG-01 | REG-06 |
| REG-AC-02 | REG-02 | REG-06 |
| REG-AC-03 | REG-01 | REG-06 |
| REG-AC-04 | REG-03 | REG-06 |
| REG-AC-05 | REG-04 | REG-06 |
| REG-AC-06 | REG-04、REG-05 | REG-06 |
| REG-AC-07 | REG-05 | REG-06 |
| REG-AC-08 | REG-01 至 REG-05 | REG-06 |
| REG-AC-09 | REG-06 | REG-06 |
