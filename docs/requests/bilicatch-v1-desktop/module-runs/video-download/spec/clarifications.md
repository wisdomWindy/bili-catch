# 规格澄清：视频下载与音视频合并

## Q1：仅视频是否调用FFmpeg remux？

- Question：DASH fragmented MP4直接保存可能存在播放器兼容差异，是否为兼容性调用FFmpeg？
- Answer：PRD明确“仅视频只下载视频流，不调用合并”，V1不包含转码；保持直接输出最符合范围。
- Final decision：`video-only` 不调用FFmpeg，非空video/mp4轨直接原子保存为 `.mp4`。若真实验证不满足最低可用性，必须回spec重新批准。
- Affected spec area：user flow、MP4 mux and output、VID-AC-07/13。

## Q2：遇到非MP4 DASH源如何处理？

- Question：是否为WebM等容器增加转换或自动换容器？
- Answer：PRD只要求AVC/HEVC/AV1下载与无损合并，没有多容器/转码目标；静默转换会扩大范围并改变性能。
- Final decision：V1只接受允许的MP4 video与普通AAC/M4A audio；其他source以E004拒绝，不转码、不静默换quality/codec。
- Affected spec area：source permission、edge cases、VID-AC-06/07。

## Q3：`connectionsPerTask=1` 与双轨并行如何协调？

- Question：单任务总连接预算为1时，不可能同时维持两个HTTP轨道请求。
- Answer：设置合同优先保证连接预算上限，默认值8仍走并行；极端值1允许顺序完成两轨。
- Final decision：预算>=2按ceil/floor分配并行；预算1先video后audio。任何同轨Range并发仍需服务端能力证明。
- Affected spec area：connection budget、VID-AC-08。

## Q4：HDR/8K是否新增独立控件或稳定字段？

- Question：HDR/杜比视界可能包含额外profile语义，V1是否增加独立选择？
- Answer：PRD现有下载中心只有清晰度与编码；新增控制会改变已批准页面设计。
- Final decision：不新增控件；HDR/8K由B站quality label展示，精确source仍由qualityId+codec匹配。未知codec/profile不伪装为支持。
- Affected spec area：page and module design、VideoVariant contract、VID-AC-01/02。

## Q5：视频合并选择哪条音频轨？

- Question：视频模式没有audio format/profile字段，应使用普通音频还是HiRes/FLAC？
- Answer：MP4 stream-copy与现有视频draft需要稳定且无需新增UI的选择；独立FLAC属于audio-only合同。
- Final decision：选择当前账号有权访问的最高普通AAC/M4A轨；不选择FLAC/HiRes，不写独立音频封面/标签。
- Affected spec area：fresh source、mux、VID-AC-05/13。

## Q6：登录状态变化如何处理前端解析缓存？

- Question：仅按输入键控的前端cache可能在登录后继续展示匿名能力，或退出后保留高质量选择。
- Answer：执行期fresh验证仍会保护安全，但UI能力必须及时一致。
- Final decision：auth状态变化清除parse result cache；当前成功输入自动fresh reparse一次。有效选择保留，否则按默认quality和codec顺序回退并提示一次。
- Affected spec area：page behavior、selection、VID-AC-02/18。

## Approval status

- 上述决定是本次视频模块规格的一部分。
- 当前状态：用户已于2026-09-11批准规格；Q1..Q6均已确认并进入plan。
