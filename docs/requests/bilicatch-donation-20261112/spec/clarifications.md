# Clarifications

## Resolved Upstream Decisions

- 收款渠道：附件明确为支付宝与微信支付。
- 资产来源：仅使用用户上传的两张原图。
- 展示位置：现有设置页“关于”区域，依据代码所有权收敛影响范围。
- 支付范围：仅展示二维码，不接入交易流程。

## Open Questions

无。

## Assumptions

- `code-fact-backed`：Vite 的图片 ESM import 和现有 `NModal`/Naive UI 组件能力可满足本地展示；这是可逆且不改变产品语义的技术选择。

## Missing Source / Out Of Scope

- 金额、订单、支付状态、感谢文案、渠道推荐、分析埋点均无来源，保持 out-of-scope。
