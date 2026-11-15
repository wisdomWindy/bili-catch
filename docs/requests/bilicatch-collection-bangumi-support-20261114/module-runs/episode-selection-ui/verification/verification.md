# 验证：统一选集界面

## 命令证据

| 命令 | 结果 |
| --- | --- |
| `cargo fmt -- --check` | 通过 |
| `cargo clippy --all-targets -j 1` | 通过（仅既有 warning） |
| `cargo test --lib --tests -j 1` | 通过（lib 111 passed；集成套件全绿） |
| `pnpm run typecheck` | 通过 |
| `pnpm run test` | 通过（58 files / 242 tests） |
| `pnpm run build` | 通过 |

## 验收标准映射

| AC | 结论 | 证据 |
| --- | --- | --- |
| AC-1 标签 | 通过 | `PartSelector.test.ts`：番剧 `["EP1","EP2"]`；普通视频 `["P1","P2"]` |
| AC-2 集头 | 通过 | 多 P 合集成员渲染 `["EP1","P1","P2"]` 且 2 个 `.part-child`；普通多 P 无 `.part-group` |
| AC-3 计数与全选 | 通过 | store 用例：`selectAll` 只剩可下载 1001；`invert` 清空；`toggleGroup([1001,1002])` 只选 1001 |
| AC-4 不可下载 | 通过 | 组件用例：行 `.is-disabled`、原因“需大会员”、`disabled` 属性存在；store 用例：`togglePart(不可下载)` 不改变选中 |
| AC-5 集头联动/半选 | 通过 | 组件用例：半选时点击 emit `[[30]]`（排除不可下载子单元）；全选场景 emit `[[10,11]]` |
| AC-6 摘要 | 通过 | `VideoSummary` 条件渲染 `v-if="result.bvid"` 与 `durationSeconds > 0` |
| AC-7 空能力 | 通过 | 既有 `DownloadOptions` 的“暂无可用选项”路径未改动；入队条件未改 |
| AC-8 不回归 | 通过 | 全量 57 既有前端测试 + 全部 Rust 测试通过 |
| AC-9 三命令 | 通过 | 见命令证据 |

## 交互要点

- 集头 `indeterminate` 通过 DOM property 设置（Vue `:indeterminate`），并显式维护 `checked`。
- 不可下载行 `aria-describedby` 指向原因文本 id。
- 全选/反选不再能选中不可下载单元（行为变化仅影响原本不该被选中的单元）。

## 未覆盖与原因

- 真实浏览器中的半选视觉与窄宽度布局：仅组件级渲染断言，未做截图回归。
- 大列表滚动与性能：沿用既有固定高度滚动，未做虚拟列表。

## 残留风险

- 若某集所有子 P 都不可下载，集头禁用但子行仍显示（置灰），用户可见但不可操作。
