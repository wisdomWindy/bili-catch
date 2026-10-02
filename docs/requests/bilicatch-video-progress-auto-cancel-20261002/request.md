# 请求：修复视频任务自动取消

## 请求标识

`bilicatch-video-progress-auto-cancel-20261002`

## 来源链接

2026-10-02 当前 Codex 会话中的用户缺陷报告，复现 BV 号为 `BV1P2HQzbE61`。

## 业务摘要

修复视频与音频双轨下载过程中，用户没有执行取消操作但任务被标记为“已取消”的问题。

## 目标陈述

双轨下载在第二条媒体轨首次进入进度聚合、聚合总量发生变化时，任务进度保持单调且任务继续执行；只有用户取消或执行尝试确实失效时才中止下载。

## 初始完成信号

- 回归测试覆盖视频进度已到 60% 后音频首次上报导致原始比例回退的场景。
- 回归测试证明合法的双轨进度更新不会触发 `DownloadControlState::CancelRequested`。
- Rust 定向测试与完整测试通过。

## 触发条件

用户输入 `BV1P2HQzbE61` 并将“视频 + 音频”任务加入队列；任务下载至约 60% 后自动显示“已取消”。

## 初始上下文来源

- 用户缺陷报告
- 本机 `tasks.json.bak` 中该 BV 的持久化任务记录
- Bilibili 官方 view/playurl API 对该 BV 返回的媒体元数据
- `VideoProgressState`、`TaskManager::apply_execution_update_at` 与下载控制器实现

## 人工交接点

按仓库默认隐式审批完成 spec 与 plan；实现、验证和评审结果在本请求工作区内交付。

## 影响范围

仅调整视频双轨下载的进度聚合与上报规则，不改变前端状态契约、任务持久化结构、下载接口或取消按钮行为。

## 参与服务或模块

`services::video::progress`、`services::tasks::mutations`、视频进度回归测试。
