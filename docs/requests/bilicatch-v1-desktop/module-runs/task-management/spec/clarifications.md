# 规格澄清：下载任务管理

## Q1：下载中筛选包含哪些状态？

- question：PRD 只有“下载中”标签，但存在等待、暂停和处理状态。
- answer：采用活动集合 `queued | downloading | paused | processing`。
- final decision：四类未完成状态全部进入“下载中”，保证可发现和可控制；cancelled 只在“全部”。
- affected spec area：筛选、计数、空态。

## Q2：任务默认如何排序？

- question：PRD 未定义排序，实时状态变化可能导致行跳动。
- answer：活动组在前、终止组在后，各组按 createdAt 倒序；进度与速度不改变排序。
- final decision：使用稳定分组排序，不增加用户可配置排序。
- affected spec area：任务列表、焦点稳定性。

## Q3：100 项上限统计什么？

- question：PRD 写“队列最多 100”，未说明完成/取消记录是否计入。
- answer：统计全部未删除记录；用户通过删除或清除完成释放容量。
- final decision：批量创建超限时整批拒绝，避免部分入队和无限历史。
- affected spec area：容量、批量创建、清除完成。

## Q4：处理中任务能否取消？

- question：操作表未列 processing，但取消语义要求终止并清理临时文件。
- answer：协议允许 processing 取消，真实按钮只在 executor 声明该阶段可安全中断时启用。
- final decision：fake executor 验证终止/清理协议；后续 FFmpeg executor 必须先终止子进程再确认 cancelled。
- affected spec area：状态机、操作矩阵、TaskExecutorPort。

## Q5：没有音视频执行器时生产任务如何表现？

- question：task-management 先于 audio/video-download，不能实现范围外媒体执行。
- answer：任务可靠创建并持久为 queued；scheduler 仅在匹配 executor 注册且有槽位时启动。
- final decision：不提供伪进度或 no-op 完成；测试用 controllable fake 证明调度，视觉 demo 明确仅开发环境。
- affected spec area：调度、验收、范围边界。

## Q6：网络重试“最多 3 次”如何计数？

- question：PRD 给出 1s -> 2s -> 4s，但“最多 3 次”可能指总尝试或重试次数。
- answer：解释为初始尝试失败后最多 3 次额外自动重试，间隔依次 1/2/4 秒。
- final decision：第 3 次重试仍失败才进入 failed；手动重试开启新的自动重试周期。
- affected spec area：scheduler、状态机、进度展示。

## Q7：幂等与任务 ID 如何处理？

- question：草稿为 session 状态，IPC 成功但响应丢失可能重复创建。
- answer：handoff 增加 1-80 字符 requestId；Rust 在相关记录仍存在时复用该批任务。
- final decision：创建成功后前端才清空 handoff；任务 id 可由 requestId + 批内索引稳定派生，但内部格式不进入 UI 语义。
- affected spec area：草稿 store、create contract、persistence。

## Q8：删除任务是否删除输出文件？

- question：PRD 与 requirement-map 已给出默认口径。
- answer：否。
- final decision：删除/清除完成只移除记录；只有取消会删除临时文件，输出文件不受影响。
- affected spec area：删除、批量清理、确认文案。

## Q9：文件名归一化的未写细节是什么？

- question：PRD 只列非法字符与 200 字符上限。
- answer：额外移除控制字符，trim 首尾空白/点，内部连续空白保留；空名 fallback，Windows 保留名追加 `_`；限制包含扩展名。
- final decision：按 Unicode 字符计数至 200，不按 UTF-8 byte 截断。
- affected spec area：任务创建、跨平台文件安全。

## Q10：事件合同是否保留 PRD 名称？

- question：架构还需要状态变化与删除事件，但 PRD 明确 `download://progress`。
- answer：保留 `download://progress` 并将 payload 扩展为带 sequence 的完整 task；删除单独使用 `download://removed`。
- final decision：两个事件共享全局 sequence，前端统一缓冲/重放；不引入通用 Event Bus。
- affected spec area：API contract、Observer 生命周期、兼容性。

