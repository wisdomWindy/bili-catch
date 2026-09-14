# Clarifications

## Question

是否采用此前建议的根 Provider 类和全高约束方案？

## Answer

用户回复“根据你的建议修改”。

## Final Decision

采用 `app-provider` 类以及 `height: 100%`、`min-height: 0`；不修改设置页自身或 `body` 的滚动规则。

## Affected Spec Area

根高度链、滚动所有权、验收标准 `AC-01` 至 `AC-05`。
