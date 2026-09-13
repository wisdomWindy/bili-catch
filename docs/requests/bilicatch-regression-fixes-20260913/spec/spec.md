# 工程规格：BiliCatch 回归缺陷修复

## 交付单元标识

`bilicatch-regression-fixes-20260913`

## 背景与目标

修复打包应用中的 5 项回归：登录后媒体能力不刷新、任务操作列溢出、已登录账户页自动返回、重复解析不刷新、仅视频以外的模式无法下载。

## 范围

- 登录状态变化后的下载能力重新解析与选择校正。
- 已登录用户主动进入账户信息页的路由停留规则。
- 手动解析的强制刷新语义。
- 任务表格操作列的响应式边界。
- Bilibili 普通音频源选择和三种下载模式的执行回归。
- 自动化、真实 URL smoke 和 Windows NSIS 打包。

## 不在范围

- 新登录方式、新下载模式或视频转码。
- 改变 Bilibili 账户权限判定。
- 推送代码或发布 GitHub Release。

## 触发与开始条件

用户在打包应用中执行登录、打开账户页、解析、操作任务或入队三种模式时触发。

## 需求拆分摘要

非 PRD 拆分的缺陷修复；所有改动属于一个交付单元，因为共享登录、解析和媒体能力状态。

## 用户流程

1. 匿名解析 -> 登录成功 -> 当前输入自动重新解析 -> 需登录标识和 disabled 状态实时更新。
2. 已登录用户点击顶栏用户名 -> 进入账户信息页 -> 持续停留，仅主动导航时离开。
3. 每次点击“解析” -> 立即进入 parsing -> 绕过上次成功结果缓存 -> 重置并应用新的视频信息与选择。
4. 任务行操作控件始终留在操作列内。
5. 三种下载模式只解析所需源，需要 FFmpeg 的模式处理后原子落盘。

## 页面与模块设计

- `DownloadPage` 编排 auth watcher 和 store command，不直接调 IPC。
- `download-center/store` 拥有 parse token、选择重置和认证协调。
- `LoginPage` 只在当前页内从非 authenticated 转为 authenticated 时安排 800ms 返回。
- `TaskRow` 保持 DOM 结构，CSS 负责列内宽度与 overflow 边界。
- Rust adapter 隔离 raw API；音频选择规则由 `select_audio_source` 唯一拥有。
- 执行器按 `DownloadMode` 分流，不跨模式猜测或静默降级。

## 功能完整行为分解

### 认证后能力刷新

- auth status 转为 authenticated 后，对已有 normalized input 发起 fresh parse。
- 旧响应不得覆盖新响应。
- 刷新成功保留模式；原 quality+codec 仍有效则保留，否则 fallback 并通知一次。
- `requiresLogin`、disabled 和文案来自新结果。

### 登录页返回

- 本页扫码从等待态变为 authenticated 后保留 800ms 成功反馈，再返回有效 `from` 或 `/download`。
- 进入页面时已 authenticated，或 restoring 恢复 authenticated，不创建返回 timer。
- 离开页面清理 timer 和活动扫码。

### 手动重新解析

- 输入保持原值展示，按现有 normalizer 做 trim/链接校验。
- 每次点击或 Enter 都发起 fresh request，不使用前端成功结果缓存。
- Rust `parse_video` 也获取新 metadata/playurl；WBI key 缓存可保留。
- parsing 及时显示，成功完整 `applyResult`，失败清除旧 result。

### 任务表格操作列

- 操作列宽度与行内控件宽度一致，子项 `min-width: 0`。
- 按钮组在单元格内紧凑排列或换行，不覆盖相邻列。
- `<=1050px` 表头和行内容保持同一列 mapping；`<=850px` 使用窄屏行布局。
- 1280/900/800px 无页面横向溢出、按钮裁切和内容串列。

### 三种下载模式

- 仅视频：匹配 quality+codec 的 MP4 视频源，不要求音频源，不 mux。
- 视频+音频：匹配视频源和普通 AAC/M4A 音频源，两轨下载后 mux。
- 仅音频：不要求 quality/codec 或视频源，通过音频 executor 处理。
- 匿名“64K”普通源不得仅因为报告 bandwidth 略高于 64,000 bps 被拒绝；仍排除 FLAC/HiRes。
- completed 前校验最终文件存在且非空。

## 设计约束

- 页面编排，store 拥有前端状态规则，Rust service 拥有用例，adapter 隔离 raw API，executor 拥有 I/O。
- fresh parse 规则在 download store/parser 边界各一处；音频选择只在 `select_audio_source`。
- 网络调用不进入 Vue component；路由 timer 只在 LoginPage；文件/进程操作只在 Rust executor/infrastructure。
- 保留现有 Pinia command、Observer auth event 和 executor Strategy registry；不增加 manager/factory。

## 项目脚手架决策

已有 Vue 3 + Pinia + Tauri 2 工程，不新建脚手架或更换依赖。

## 变化轴与模式决策

变化轴为 auth transition、parse trigger 和 download mode。保留现有模式，不引入新设计模式。

## 代码上下文与影响假设

`tsconfig.json` 管理 `src/**/*.ts|vue`；关键选项为 ES2020、DOM lib、strict、bundler module resolution、noEmit。类型使用现有 download/auth/task contracts，无新 ambient/generated 声明。Rust serde DTO 是 IPC 权威契约。

## API 与数据契约

- `parse_video({ input: string }) -> ParseVideoResult`：字段不变，交互调用改为 fresh result。
- `auth://state -> AuthStateEvent`：按 revision 单调应用，status/revision 触发能力刷新。
- `CreateDownloadTasksRequest/DownloadTaskDraft`：字段不变，mode 决定 video/audio 字段必填性。
- Bilibili JSON 继续经 Rust raw + adapter 转换；前端不消费 raw 字段。
- E003/E004/E005/E006/E009 语义不变。

## 上下文与依赖源

用户缺陷报告、既有获批规格、`tsconfig.json`、Vue/Pinia contracts、Rust models/services/infrastructure。

## 边界条件

- 登录成功但没有已解析输入：只更新 authenticated，不请求。
- auth refresh 失败：显示 parse 错误，不保留伪新能力。
- 快速重复解析：只最后 token 提交结果。
- 音频候选真为空才返回 E004。
- 最终文件不存在/为空：不上报 completed。

## 验收标准

- **REG-AC-01** 匿名解析后登录，至少再调用一次 parse service；新结果中可用清晰度不再显示“需登录”且可选。
- **REG-AC-02** 已登录快照下点击用户名进入 `/login`，超过 800ms 仍留在 `/login`；本页扫码成功仍按 800ms 返回。
- **REG-AC-03** 同一输入连续两次手动解析，service 调用两次，第二次 result 覆盖所有结果与默认选择。
- **REG-AC-04** 1280/900/800px 任务行的各子元素不超出所属单元格，行无横向滚动，按钮保持 36x36px。
- **REG-AC-05** 匿名普通音频候选 bandwidth 为 65,971/85,411 bps 时，M4A/MP3 与 video+audio 可选源，不返回 E004。
- **REG-AC-06** audio-only 不调用视频 resolver；video+audio 下载两轨并 mux；video-only 不 mux。
- **REG-AC-07** 三模式完成后 outputPath 对应非空文件；输出失败不标记 completed。
- **REG-AC-08** 前端/Rust 专项与全量测试、typecheck 全部通过。
- **REG-AC-09** 用用户提供的完整 URL 做解析 smoke，并成功构建 NSIS 安装包及 updater 签名。

## 人工审核与交接

用户批准本规格后生成实施计划；实施后交付变更、测试结果、安装包路径和剩余风险。

## 风险

- Bilibili 外部接口和 bandwidth 可变；用 fixture 固定规则，公网只做 smoke。
- 高清能力依赖当前账户，自动化不记录 Cookie。
- CSS 修复可能影响菜单定位，需多宽度边界检查。
