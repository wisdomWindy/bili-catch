# 代码上下文

## 上下文需求

需要还原双轨进度从下载器到任务状态机再到取消控制器的完整调用链。

## graph 可用性检查

当前环境未提供仓库 code graph 工具。

## 安装或 bootstrap 记录

未发现可执行的仓库内 code graph bootstrap 入口；该缺陷可通过现有测试和文本检索完成可靠定位，因此未引入新工具依赖。

## fallback 记录

使用 `rg`、Git 历史、本机持久化任务记录和 Bilibili 官方 API 响应重建调用链并交叉验证根因。

## 相关入口点

- 内部执行入口：`VideoExecutor::execute`
- 双轨并发入口：`tokio::join!(video_future, audio_future)`
- 进度入口：`VideoProgressSink::report`
- 状态写入入口：`TaskManager::apply_execution_update_at`
- 取消入口：`DownloadControl::request_cancel`

## 关键符号与模块

- `VideoProgressState`
- `aggregate_video_progress`
- `download_percent`
- `ExecutionReporterPort`
- `ExecutionUpdate::Progress`
- `DownloadControlState::CancelRequested`

## 依赖与副作用边界

进度聚合器同步调用任务管理器；任务管理器拒绝非单调快照时返回 `None`，适配器将其映射为 `Ok(false)`；进度回调把任何非 `Ok(true)` 结果映射为下载取消。修复必须只消除由总量扩展产生的合法百分比回退，不能吞掉真正的过期 attempt 信号。

## 影响范围

集中在视频双轨进度计算和对应测试；音频单轨进度、任务数据结构、前端展示和持久化格式不变。

## 开放后续检查项

- 验证总量扩展时百分比保持当前值。
- 验证后续字节增长可继续推进百分比。
- 验证 reporter 明确拒绝过期 attempt 时仍请求取消。
