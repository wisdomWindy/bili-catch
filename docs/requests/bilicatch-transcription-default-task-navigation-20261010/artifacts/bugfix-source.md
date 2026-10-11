# 缺陷来源归一化

## 来源系统

当前 Codex 会话中的直接用户报告。

## 项目 key 或来源范围

BiliCatch 桌面应用，音频转文本参数与任务列表导航链路。

## 缺陷或工作项 ID

`bilicatch-transcription-default-task-navigation-20261010`

## 缺陷标题

转写语言默认值错误，任务列表首次切换被数据初始化延迟。

## 观察到的行为

1. 音频转文本页面的语言下拉框显示“自动检测”。
2. 点击侧边栏“任务列表”后，当前页面会停留一段时间，随后才切换到任务列表。

## 期望行为

1. 未显式配置时默认显示“简体中文”。
2. 点击任务列表后立即切换页面；页面内任务数据允许随后异步加载并显示加载状态。

## 复现线索

1. 使用当前本机设置文件启动应用并进入音频转文本页面，语言项显示“自动检测”。
2. 音频转文本页面已经加载任务数据后点击“任务列表”，观察路由内容切换延迟。

本机设置证据：`C:/Users/yangjianlin/AppData/Roaming/com.bilicatch.app/settings.json` 为 `schemaVersion: 1`，没有 `defaultTranscriptionLanguage` 字段，因此迁移逻辑会使用后端默认值。

## 受影响服务、模块、接口、任务或消费者

- `src-tauri/src/services/settings/mod.rs`
- `src/pages/TranscribePage.vue`
- `src/features/settings/demo.ts`
- `src/features/download-center/store.ts`
- `src/pages/TasksPage.vue`
- 下载任务与转写任务初始化调用链

## 可用截图、日志或评论

- 截图：`C:/Users/YANGJI~1/AppData/Local/Temp/codex-clipboard-7519b472-d7f5-4f8b-a227-18108a30c89c.png`
- 用户原文：
  - “红框中的语言选项预期应该是默认简体中文”
  - “任务列表加载变慢，点击任务列表菜单之后需要等一会才会切换到任务列表页面，预期是点击任务列表菜单就切换到对应页面，至于页面中的数据可以等切换到页面之后再加载”

## 开放问题与缺失上下文

无阻断项。用户要求明确限定为默认值和加载时序，不涉及移除“自动检测”选项或修改任务数据格式。

## 闭世界修改白名单

- 设置默认值与旧文档缺省字段迁移测试。
- 转写页及其直接默认值提供者。
- 任务页挂载后的初始化调度及其页面测试。
- 本请求工作区文档。

## 复测重开（2026-10-11）

- 用户复测反馈：任务列表切换仍然很慢。
- 运行进程证据：`D:\program\bilicatch\bilicatch.exe` 的 SHA-256 为 `430cf72ccccef2b27ca323970bf055c29cb5f5eba0efa11080448c05ebd05bd0`；当前 release 的 SHA-256 为 `37a12495b8d2f7784d3d2e9f98de0edab0d57e123a4e671e906a3dd2cdecefed`，说明复测程序未包含上一轮修复。
- 测试缺口：现有测试只断言初始化未调用，没有断言 Pinia 中已有的缓存任务在首帧不得被投影和渲染。
- 修复边界：任务页首帧只呈现结构与 skeleton；首帧完成后再开放缓存投影并初始化 store；版本升级为 `0.1.19` 以保证安装器升级识别。
