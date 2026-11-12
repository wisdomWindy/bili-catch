# Task Board

- [x] T1 RED：新增 DonationDialog / AboutSettings / SettingsPage 行为断言并记录预期失败
- [x] T2 资产与 i18n：逐字节复制两张图片、校验 SHA-256、增加双语键
- [x] T3 GREEN：实现 DonationDialog、AboutSettings 入口和 settings dialog 样式复用
- [x] T4 REFACTOR：scoped tests 通过、引用链与 diff 自检通过
- [x] T5 VERIFY：全量测试、构建、三组浏览器场景和焦点/关闭交互通过
- [x] T6 REVIEW：完成适用 verdict、无 blocker、状态推进 complete

## Source And Product Delta

- T1：code-fact-backed test adaptation；product-content delta none。
- T2：source-backed assets/labels；delta 仅限 spec allowlist。
- T3：source-backed interaction + code-fact-backed implementation；delta 仅限 spec allowlist。
- T4/T5/T6：verification/review；product-content delta none。
