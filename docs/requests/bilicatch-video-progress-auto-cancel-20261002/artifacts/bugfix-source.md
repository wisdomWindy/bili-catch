# 缺陷来源归一化

## 来源系统

当前 Codex 会话中的直接用户报告。

## 项目 key 或来源范围

BiliCatch 桌面应用，视频下载任务执行链路。

## 缺陷或工作项 ID

`bilicatch-video-progress-auto-cancel-20261002`

## 缺陷标题

双轨视频下载执行到一半时在未操作取消的情况下变为“已取消”。

## 观察到的行为

`BV1P2HQzbE61` 的“视频 + 音频”任务在约 60% 时变为 `cancelled`。用户未执行取消操作，任务没有错误信息。

## 期望行为

第二条媒体轨首次上报导致聚合总量增加时，已展示进度不得回退，任务应继续下载、合并并完成。只有显式用户取消或过期执行尝试才应触发内部取消控制。

## 复现线索

1. 输入 `BV1P2HQzbE61`。
2. 选择视频与音频模式、480P AVC，并加入任务队列。
3. 视频流先产生进度，音频流稍后首次产生进度。
4. 观察任务在约 60% 时进入 `cancelled`。

本机持久化记录显示：`status=cancelled`、`progressPercent=60`、`bytesDownloaded=1014439`、`totalBytes=1514175`、`error=null`。官方 playurl 响应验证 AVC 视频流总长正好为 `1514175` 字节，音频流总长为 `1610204` 字节。因此持久化快照来自仅视频轨的进度；音频首次加入聚合时总量扩展并使计算百分比回退，任务管理器拒绝该更新，进度回调随后主动请求取消。

## 受影响服务、模块、接口、任务或消费者

- `src-tauri/src/services/video/progress.rs`
- `src-tauri/src/services/download/transfer.rs::download_percent`
- `src-tauri/src/services/tasks/mutations.rs::apply_execution_update_at`
- 视频与音频双轨下载任务

## 可用截图、日志或评论

无截图。可用证据为用户描述、本机任务持久化记录、官方 API 媒体大小以及代码调用链。

## 开放问题与缺失上下文

无阻断问题。缺陷由真实任务记录与流大小共同确认，可用确定性的单元测试复现。
