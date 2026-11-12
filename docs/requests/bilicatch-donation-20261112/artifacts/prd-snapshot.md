# 归一化直接变更快照

## Verbatim Source Evidence Index

- `USER-DIRECT-01`：`添加打赏功能，上传的图片为收款码`
- `USER-ASSET-ALIPAY-01`：附件 `codex-clipboard-3116a9f3-0ca9-4f5c-8e1d-579c0669e092.jpg`，图片明确标识“支付宝”“推荐使用支付宝”“打开支付宝[扫一扫]”。
- `USER-ASSET-WECHAT-01`：附件 `codex-clipboard-c8609589-797e-4b96-b104-e7300616505b.png`，图片明确标识“推荐使用微信支付”“微信支付”。

## Source Summary

用户要求在现有应用中增加打赏功能，并指定两张上传图片作为收款码。附件分别属于支付宝和微信支付。

## Key Business Goals

- `source-backed`：应用提供打赏功能。
- `source-backed`：打赏功能使用用户上传的两张收款码图片。
- `source-derived`：用户必须能从现有应用界面发现入口并查看收款码，才构成可用的打赏功能。

## Greenfield Scaffold Sensitivity

不适用：not a project/app/package/frontend-surface creation request。

## Explicit Behavior Constraints

- `source-backed`：展示两张用户提供的收款码，不生成替代二维码。
- `source-backed`：支付宝与微信收款码的支付渠道语义由附件中的文字和品牌标识确定。
- `code-fact-backed`：功能应接入现有设置页“关于”区域；该区域已承载版本、更新和开源许可等产品信息，是最小影响的可发现入口。
- `code-fact-backed`：交互复用仓库现有 `naive-ui` `NModal` 模式与主题令牌，避免引入新的 UI 技术栈。
- `code-fact-backed`：功能完全本地展示，不需要后端、IPC、网络请求或持久化状态。
- `missing-source`：用户未要求金额选择、支付状态、交易确认、统计或感谢文案；这些内容不得实现。

## Closed-World Product Scope Allowlist

- 在设置页“关于”区域新增一个名为“打赏”的可操作入口。
- 操作入口后展示用户提供的支付宝收款码和微信收款码。
- 收款码界面提供渠道识别、完整查看和标准关闭能力。
- 为现有中英文界面提供等义的打赏相关标签，英文仅作为既有国际化能力的必要兼容项。

除此之外的支付语义、字段、控件、状态、默认值、验证、权限、导航、接口、分析事件或成功反馈均不在范围内。

## Forms, Tables, Displays, And Interactions Extracted From Source

- 页面/容器：现有设置页的“关于”区域（仓库约束）。
- 控件：一个“打赏”入口（`USER-DIRECT-01` 的最小可操作形态）。
- 展示：支付宝收款码 `USER-ASSET-ALIPAY-01`；微信收款码 `USER-ASSET-WECHAT-01`。
- 交互：打开收款码界面、查看两张图片、关闭界面。
- 表单/表格：不适用。

## Workflow And State Rules Extracted From Source

- 初始状态：打赏界面关闭。
- 打开：用户触发打赏入口后显示收款码界面。
- 关闭：通过标准对话框关闭手段回到设置页。
- 支付中、成功、失败、重试、确认状态：不适用，用户未要求且本功能不接入支付服务。

## Relevant Modules Or Pages

- `src/features/settings/components/AboutSettings.vue`
- `src/features/settings/components/AboutSettings.test.ts`
- `src/features/settings/settings.css`
- `src/locales/zh-CN.ts`
- `src/locales/en-US.ts`
- `public/` 本地静态图片资源

## Notable Open Questions

无阻塞问题。收款码渠道和资产均由附件明确；展示位置与交互实现受现有设置页结构和对话框能力约束。

## Out Of Scope

- 任何后端支付能力、金额选择、订单、支付回调、交易确认、历史记录或统计。
- 修改、裁切、重绘或重新编码二维码内容。
- 新增独立路由、全局导航项或首页推广位。
- 与本功能无关的设置页重构或视觉调整。
