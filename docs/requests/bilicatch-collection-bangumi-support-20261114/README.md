# BiliCatch 合集与番剧支持请求

本目录记录 `biliCatch_PRD.md` §1.3“合集/番剧批量下载”与用户直接指令共同驱动的交付生命周期。状态源为 `state.json`，原始需求源为仓库根目录 PRD 与本目录 `artifacts/prd-snapshot.md`。

当前生命周期：`intake -> requirement-analysis -> requirement-splitting -> page-design -> spec -> architecture-design -> plan -> execute -> verify -> review -> complete`

已确认口径（2026-11-14）：番剧仅支持匿名可见的免费/限免集；合集与番剧共用统一选集列表；批量沿用多选；选择粒度展开到分 P。

模块队列：`episode-identity-and-contract` → `episode-metadata` → `episode-download` → `episode-selection-ui`（全部已完成，请求 `complete`）。

模块 1：`ep`/`ss` 识别、单元契约、任务字段与旧数据兼容。
模块 2：合集（`ugc_season`）与番剧（`pgc season`）解析、正片过滤、毫秒归一、可用性初判、`requestedCid`。
模块 3：PGC 取流、权限矩阵、任务归属校验、`{序号}_{标题}` 命名与 `containerKind`。
模块 4：统一选集列表（EP/P、集头/半选、置灰原因）、摘要空值处理、双语原因文案。
