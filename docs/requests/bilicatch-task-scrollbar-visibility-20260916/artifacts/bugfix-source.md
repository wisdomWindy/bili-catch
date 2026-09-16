# 缺陷来源

- evidence id：`USER-DEFECT-01`
- verbatim：任务列表中当任务数量超过软件高度时，没有出现纵向滚动条。
- reproduction：打开任务列表，使任务行总高度超过窗口主内容区高度。
- expected：主内容区显示纵向滚动条并可滚动到底部。
- affected：`src/styles/layout.css` 中的 `.content-scroll`。

## 范围

- 允许：增强现有滚动容器的滚动条可见性。
- 不允许：创建嵌套滚动、修改任务状态或下载行为。
