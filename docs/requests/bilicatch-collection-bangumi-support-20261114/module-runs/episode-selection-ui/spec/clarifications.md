# 澄清：统一选集界面

## Q1：为什么用 `EP{n}` 表示合集成员？

- 需求 H2 要求用 P/EP 区分来源；合集成员在 B 站语境就是“第 N 集”，但没有稳定的成员级标题可供容器行使用。
- 决定：合集成员统一 `EP{groupIndex}`，多 P 成员的子行用 `P{page}`。
- 锚定：confirmed-decision（H2）。

## Q2：集头哪些行为是默认口径？

- 集头点击 = 全选/取消组内可下载子单元；半选视作未全选。
- 锚定：analysis Q9 默认口径。

## Q3：不可下载行是否计入 m？

- 不计入；全选/反选也不包含。
- 锚定：spec AC-3。

## Q4：为什么保留 `PartSelector.vue` 不拆组件？

- 只有一个真实调用方（DownloadPage），行结构是简单重复；拆分会引入 props 透传且无可复用语义。
- 锚定：`vue-component-extraction.md` 的 keep-inline 条件。

## Q5：为什么不用 Tailwind？

- 项目未引入 Tailwind，样式体系为 `download-center.css` 语义类；按技术栈锚定沿用既有方案，不新增依赖。
- 锚定：既有代码事实（`src/styles/pages.css` 引入、组件 `class` 使用语义类）。
