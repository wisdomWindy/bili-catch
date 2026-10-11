# 实施计划：转写默认语言与任务页切换时序

## 1. 执行范围

依据已批准 spec 的 3.1、3.2 和 AC-01 至 AC-08，只修改默认语言提供链、任务页初始化调度及直接测试。product-content allowlist 仅包含：默认值 `zh`、切页后加载、保留显式 `auto` 和既有 UI/IPC 合同。

不修改路由、语言选项集合、任务数据结构、CSS、组件库、状态管理方式或错误文案。

## 2. 业务功能点

### BF-01 默认简体中文

- 角色/入口：进入音频转文本页面的用户。
- 对象：`defaultTranscriptionLanguage` 与 `form.language`。
- 前置：全新设置，或 schema v1 设置缺少语言字段。
- 规则：缺省值为 `zh`；schema v2 已保存合法值原样保留。
- 成功：页面 select 显示“简体中文”，创建任务继续提交现有语言枚举。
- 失败/异常：非法旧值沿用既有迁移回退；不在模板中吞掉错误。
- 来源：`[SRC-U1][DATA-D1][CODE-C1][CODE-C2]`，spec 3.1。

### BF-02 页面先呈现、任务后加载

- 角色/入口：从侧边栏点击“任务列表”的用户。
- 对象：`TasksPage` 挂载和两个 store 初始化副作用。
- 前置：路由已创建目标页面。
- 规则：mount 后当前浏览器任务只完成页面渲染；下一零延迟任务才调用两个 `initialize`。卸载时取消未触发的调度；已触发后由既有 `dispose` 清理。
- 成功：慢订阅/慢快照时页面标题与 skeleton 已可见。
- 失败/异常：订阅和快照错误继续由 store 归一并显示现有失败态。
- 来源：`[SRC-U2][CODE-C3][CODE-C4]`，spec 3.2。

## 3. 顺序任务

### T-01 默认语言 RED

目标文件：`src-tauri/tests/settings_manager.rs`、`src/pages/TranscribePage.test.ts`。

1. 将现有 schema v1 迁移断言从 `Auto` 改为手工期望 `Zh`。
2. 增加/调整全新设置默认值断言为 `Zh`。
3. 增加 schema v2 显式 `Auto` 保留测试，防止把“默认中文”误实现成“禁止自动检测”。
4. 在转写页测试中不初始化 settings store，读取真实 language select 的 DOM value，期望 `zh`。
5. 运行定向测试并记录预期失败：生产代码仍返回 `auto`。

product-content delta：仅测试已批准默认值，无新增产品内容。

### T-02 默认语言 GREEN

目标文件与符号：

- `src-tauri/src/services/settings/mod.rs::SettingsDefaults::values`
- `src/pages/TranscribePage.vue::form`
- `src/features/settings/demo.ts::values`
- `src/features/download-center/store.ts::transcriptionDefaults`

步骤：

1. 把权威后端默认值改为 `TranscriptionLanguage::Zh`，使新文档和 schema v1 缺字段迁移共享同一结果。
2. 把页面、demo 和下载 store 的启动安全缺省改为 `zh`。
3. 不修改 schema v2 直接反序列化路径和 `SettingsPatch`，确保显式 `auto` 保留。
4. 运行 T-01 定向测试直到通过。

product-content delta：默认语言由自动检测变为简体中文，来源 spec 3.1。

### T-03 任务加载 RED

目标文件：`src/pages/TasksPage.test.ts`。

1. 使用 Vitest fake timers 挂载真实 `TasksPage`，只替换外部 task service/event source。
2. mount 返回后断言真实页面标题存在，且两个事件订阅均未调用。
3. 推进 pending timers 后断言下载和转写事件订阅开始。
4. 增加卸载分支：推进 timer 前卸载，之后订阅保持未调用。
5. 运行该测试，确认当前同步 `onMounted` 实现失败。

product-content delta：none；测试切入点对应 spec AC-05/AC-06。

### T-04 任务加载 GREEN

目标文件与符号：`src/pages/TasksPage.vue` 的挂载/卸载生命周期。

1. 在页面同文件定义可空 timer handle 和局部启动函数。
2. `onMounted` 只调度零延迟任务；回调先清空 handle，再调用现有两个 store `initialize`。
3. `onUnmounted` 在调用两个 store `dispose` 前清除仍待执行的 timer。
4. 保留 store 内部先订阅后快照、错误归一化和 loading owner，不创建第二套状态。
5. 运行 T-03 及既有 TasksPage 状态测试直到通过。

product-content delta：任务页在数据加载前可见，来源 spec 3.2。

### T-05 回归、手工验证与交付检查

1. 运行前端定向测试、全量 Vitest、`npm run typecheck`、`npm run build`。
2. 运行 Rust settings 定向测试与 `cargo test`。
3. 在 demo 浏览器检查 `/transcribe?demo=1` 默认中文；从转写页点击任务列表，确认页面先切换并显示加载态。
4. 检查最终 diff，仅保留 T-01 至 T-04 对应 hunks；不覆盖工作区既有无关修改。
5. 更新 changelog、verification、review 和 state。

product-content delta：none。

## 4. TDD 与验证命令

- RED/GREEN 前端：`npm test -- src/pages/TranscribePage.test.ts src/pages/TasksPage.test.ts`
- RED/GREEN Rust：`cargo test --test settings_manager`（工作目录 `src-tauri`）
- 相关 store：`npm test -- src/features/download-center/store.test.ts src/features/settings/store.test.ts`
- 前端全量：`npm test`
- 类型：`npm run typecheck`
- 构建：`npm run build`
- Rust 全量：`cargo test`（工作目录 `src-tauri`）
- 仓库无 lint script，记录为不适用。

## 5. TypeScript 与 API 合同

- governing config：根 `tsconfig.json`，重点约束 `strict`、no-unused、DOM timer 类型和 bundler resolution。
- 类型来源：现有 `TranscriptionLanguage`、`TaskService`、`TaskEventSource`、`TranscriptionEventSource`；不新增并行类型。
- API 合同：Rust `SettingsValues` 的 camelCase 序列化字段与前端合同形状不变，仅缺省枚举值改变；直接消费，无 adapter。
- `null`：timer handle 无待执行任务；`undefined` 不用于该状态。

## 6. 副作用、健壮性与清理合同

- 定时器 owner 为 `TasksPage`，唯一创建点在 `onMounted`，唯一清理点在 timer 回调和 `onUnmounted`。
- Promise 错误仍由 store action 捕获，不增加裸 `await` 或未处理拒绝。
- 页面卸载和 store dispose 顺序必须防止新订阅在离页后启动。
- 不引入 debounce/throttle/cache/virtualization；零延迟 timer 是渲染时序边界，不是频率优化。
- 不抽取共享常量或 composable；跨语言默认值由各层最窄 owner 持有，并由测试校准。

## 7. 代码评审预检合同

- 健壮性：覆盖缺字段、显式 `auto`、pending timer 卸载、服务 Promise pending。
- 可维护性：保持现有 SFC 生命周期结构；新增局部函数只负责启动初始化，不混合数据转换或渲染。
- 性能/内存：timer 必须 cleanup；既有 5 个订阅仍由 store dispose。
- 技术栈：继续使用 Vue 3、Pinia、Vue Router、Vitest、Rust 现有 settings service；禁止新增库。
- UI fidelity：语言 select 只有默认值变化；任务页视觉、文案和交互态不变。

## 8. 回滚与阻断

- 任一测试显示显式 `auto` 被覆盖，回退到 spec 检查默认与强制迁移边界。
- 如果 timer 调度导致测试无法确定或页面卸载后仍订阅，回退 T-04 重新设计局部生命周期，不修改 store 合同。
- 如果发现路由守卫/组件异步加载才是真实阻塞，回退 code-context 和 plan；当前代码证据未发现该路径。

## 9. 复测重开计划（iteration 7）

1. 先补充缓存任务首帧不可见的失败测试，保留现有初始化与卸载断言。
2. 在 `TasksPage` 增加本地内容开放状态，以 frame 后 timer 的顺序提供一次真实绘制机会，并在卸载时清理两类句柄。
3. 将缓存投影、批量能力和容量/错误提示绑定到内容开放状态，首帧只渲染稳定 skeleton。
4. 升级应用版本到 `0.1.19`，执行定向测试、全量门禁、前端构建和 Tauri 安装器打包。
5. 比对新 release/安装器产物并记录验证证据。

## 10. Iteration 8 执行计划

1. 新增 RED store 测试，证明同一运行时在 hydration 进行中及完成后均不会重复初始化。
2. 新增路由回归测试，证明 `/transcribe` 到 `/tasks` 不销毁、不重建共享任务订阅。
3. 两个 store 保存并返回同一运行时的初始化 Promise；dispose 或失败重试时失效。
4. 删除页面级共享 store dispose，并由 `App.vue` 在根节点卸载时统一清理。
5. 执行定向测试、全量前端测试、typecheck、build；全部通过后重新生成仅 NSIS 安装包。
