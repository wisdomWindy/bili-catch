# 澄清记录

## CL-01：是否移除“自动检测”

- question：默认简体中文是否意味着不再允许自动检测？
- answer：否。用户只要求默认值改变。
- final decision：保留 `auto` 选项和显式持久化能力，仅把缺省值改为 `zh`。
- affected spec area：3.1、范围外、AC-03。
- grounding status and required source：`grounded`，来源 `[SRC-U1]` 的“默认”措辞及现有选项合同。

## CL-02：任务数据何时开始加载

- question：页面 DOM 更新后是否可以立即启动加载？
- answer：可以，但必须先让浏览器获得一次渲染机会，避免事件监听注册和 IPC 读取阻塞可见切页。
- final decision：在任务页挂载后通过可取消的零延迟浏览器任务启动既有初始化。
- affected spec area：3.2、5、AC-05、AC-06。
- grounding status and required source：用户可见目标由 `[SRC-U2]` 授权；具体调度机制是可逆、非产品语义的技术选择。

## CL-03：是否需要架构设计阶段

- question：两个 store 的初始化时序是否需要新共享抽象？
- answer：不需要。变化仅属于 `TasksPage` 单一生命周期边界，没有第二个同语义生产调用方或公共 API 变化。
- final decision：`architecture-design_required=false`，进入 plan。
- affected spec area：7。
- grounding status and required source：`code-grounded`，来源 `[CODE-C3]` 与 locality / Anti-DRY 评估。

## 开放问题

无。当前信息足以实施和验收。
