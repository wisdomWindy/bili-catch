# 请求：合集与番剧支持

## 请求标识

`bilicatch-collection-bangumi-support-20261114`

## 来源

- 主来源：用户直接指令（2026-11-14 会话）
  - “「番剧/合集支持」做成一个正式 request”
  - 前置事实陈述：“这个地址明明是一个合集中的一个”（`BV1kw8b6bEeL`）
  - 前置事实陈述：“为什么不能获取 `https://www.bilibili.com/bangumi/play/ep733316/` 这个地址的视频”
- 上游产品来源：本地 PRD `biliCatch_PRD.md`（正式发布，2026-09-10）
  - §1.3 产品范围表
  - §1.4 登录必要性说明
  - 模块二业务规则（文件命名）
- 既有生命周期来源：`docs/requests/bilicatch-v1-desktop/analysis/requirement-analysis.md`

## 业务摘要

当前解析链路只把 B 站 UGC 投稿的 `pages`（分 P）当作可选单元。合集（`ugc_season`）中的每一集、番剧（PGC `ep_id` / `season_id`）中的每一话都无法进入解析与下载链路：合集内链接只能得到当前这一条视频，番剧链接在输入校验阶段即被拒绝。本请求补齐“一条链接 → 该合集/该季的可见集列表 → 选集 → 批量入队 → 下载”的能力，并把 PGC/合集特有的权限与不可下载状态显式暴露给用户。

## 目标陈述

使用户在下载中心粘贴合集内任一视频链接、或番剧 ep/ss 链接时，能够看到该合集或该季的完整可见集列表，选择其中若干集，生成与现有下载任务契约兼容的独立任务，并在权限不足或内容不可下载时得到准确、可理解的结果，而不是“解析成功但下载失败”或“输入无效”。

## 初始完成信号

- 粘贴合集内任一 BV 链接，解析结果列出该合集的全部可见集，每集可单独勾选并生成一个独立下载草稿。
- 粘贴 `/bangumi/play/ep…`、`/bangumi/play/ss…`、`b23.tv/ep…` 链接，解析结果列出该季的集列表（含已解锁与未解锁集的区分）。
- 对需登录、需大会员、付费、地区限制或禁止下载的集，返回明确错误码与用户可读文案，不出现静默失败或错误降级为普通视频。
- 现有 UGC 单视频与多分 P 的解析、选择、草稿与任务契约不回归。
- 合集内“某一集自身还有多个分 P”的情况有明确且一致的处理口径，不产生错误的任务标识。

## 触发条件

用户在下载中心粘贴合集内视频链接、番剧 ep/ss 链接或对应短链，并点击解析。

## 初始上下文来源

- `biliCatch_PRD.md`
- `docs/requests/bilicatch-v1-desktop/analysis/requirement-analysis.md`
- `docs/requests/bilicatch-v1-desktop/module-runs/parse-download-center/spec/spec.md`
- 现有代码：`src/features/download-center/input.ts`、`src/features/download-center/store.ts`、`src-tauri/src/infrastructure/bilibili/{input,raw,adapter,client}.rs`、`src-tauri/src/services/parser.rs`
- B 站公开接口实测响应（合集 `ugc_season`、PGC `pgc/view/web/season`、`pgc/player/web/playurl`），仅作为外部接口事实
- 框架：`frontend-agent-framework` 及其 `references/`

## 人工交接点

- 已确认（2026-11-14，见 `analysis/requirement-analysis.md` H1–H4）：番剧仅支持匿名可见的免费/限免集；合集与番剧共用统一选集列表；批量沿用多选手动勾选；选择粒度展开到分 P。
- 仍需人工关注：`外部接口待补` 项（PGC 错误码全集、合集失效成员状态、地区限制/DRM 实测表现）；Q5/Q6/Q8/Q9 的默认口径如不接受，可在 `spec` 评审时调整。
- 交付验证需真实链接与契约夹具；受限内容的账号权限无法在纯离线环境完整复现。

## 影响范围

- 前端：输入识别、解析 store、选集 UI、任务草稿构造。
- Rust 后端：输入归一化、B 站端口与适配、解析服务、下载源解析与任务校验。
- 契约：解析结果 DTO、选集模型、任务草稿标识。
- 不改动原始 PRD；不引入自建服务端；不实现 DRM 绕过或付费内容获取。

## 参与模块

以下为草案，最终模块边界以 `requirement-splitting` 为准：

1. 输入识别与视频/剧集标识模型
2. 合集与番剧元数据解析（端口 + 适配 + 契约）
3. 选集模型与任务草稿/任务校验
4. PGC 取流与权限/不可下载错误映射
5. 解析页选集 UI 与状态提示
