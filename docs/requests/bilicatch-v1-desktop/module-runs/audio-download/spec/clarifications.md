# 规格澄清：音频独立下载

## Q1：MP3 128/192/320K 与匿名源最高64K、登录源最高192K如何同时成立？

- 回答：MP3档位是输出编码码率，B站限制是输入源质量，两者不是同一概念。
- 最终决定：三个MP3输出档位都可选择；executor始终选择当前权限允许的最佳真实源。UI、任务信息和测试不得把320K输出称为320K源音质，也不得宣称转码提升音质。
- 影响规格区域：能力矩阵、source选择、任务副信息、AUD-AC-01/05。

## Q2：PRD要求FLAC源不支持时自动降级M4A，何时降级？

- 回答：只有在入队前，当前选择或settings默认FLAC因认证/解析能力不可用时，前端自动切换M4A并提示。
- 最终决定：任务一旦以FLAC入队，执行期source能力变化必须E005/E006失败，不能改扩展名或静默输出M4A。这样保持队列承诺与实际文件一致。
- 影响规格区域：用户流程、表单归一化、fresh source、边界情况、AUD-AC-02/03。

## Q3：M4A和FLAC的码率控件是隐藏还是禁用？

- 回答：原始PRD明确为码率下拉禁用。
- 最终决定：保留同一程序化label与稳定控件位置；M4A显示disabled“原始码率”，FLAC显示disabled“无损”。只有MP3可以展开选择。
- 影响规格区域：页面模块设计、可访问性、AUD-AC-01/20。

## Q4：M4A“直接保存”与metadata如何兼容？

- 回答：裸源直接rename无法可靠写入标题、artist和cover。
- 最终决定：M4A音频使用FFmpeg `-c:a copy` 重封装并写metadata，不重新编码音频，因此仍保持原始音频数据/码率。
- 影响规格区域：FFmpeg处理、M4A合同、AUD-AC-11/12。

## Q5：FLAC封面嵌入在不同FFmpeg build失败时是否允许忽略封面？

- 回答：模块需求把封面列为完成流程的一部分，静默忽略会产出不完整文件。
- 最终决定：使用FFmpeg 7.x参数fixture和可选real smoke验证；封面写入或输出probe失败映射E008，保留source供重试。system-release必须用实际随包binary跑最终E2E。
- 影响规格区域：processor、错误、验证、AUD-AC-12/23。

## Q6：失败后哪些临时文件保留？

- 回答：完全清除会破坏断点恢复，全部保留又会留下无效processed输出。
- 最终决定：E007/E008/E009保留通过identity/长度校验的source partial和checkpoint，删除不完整processed；success/cancel清理整个task workspace。cleanup失败不得宣称cancelled。
- 影响规格区域：workspace、finalize、retry、AUD-AC-14/16/17。

## Q7：文件名如何解释“单视频标题、合集序号_标题”？

- 回答：单P使用解析结果总标题；多P使用该分P的page和part title。
- 最终决定：单P `{videoTitle}.{ext}`，多P `{page}_{partTitle}.{ext}`，页码不补零；统一净化、200字符限制、fallback和 `(n)` 冲突去重。
- 影响规格区域：draft合同、文件命名、AUD-AC-06。

## Q8：本模块没有随仓FFmpeg二进制，能否声称真实音频下载完成？

- 回答：本模块可完成生产process adapter、可信locator合同、fake集成与可选本地FFmpeg smoke，但安装包中的sidecar只能由system-release配置和验收。
- 最终决定：缺少可信binary时任务稳定E008/`FFMPEG_UNAVAILABLE`，已下载source可供后续Retry；不得扫描任意PATH或把系统FFmpeg偶然存在当作发行证据。
- 影响规格区域：范围、运行依赖、FFmpeg合同、AUD-AC-23。

## Q9：网络错误自动重试时是否重新使用旧媒体URL？

- 回答：媒体URL有时效，直接复用会扩大失败窗口。
- 最终决定：每个新attempt都重新验证auth并获取fresh playurl；checkpoint只按不含凭据的source identity验证，绝不保存/复用旧签名URL。
- 影响规格区域：fresh source、checkpoint、自动重试、AUD-AC-08/17。
