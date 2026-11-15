# 任务板：取流与权限

| ID | 任务 | 状态 | 依赖 | 主要文件 | 验证 |
| --- | --- | --- | --- | --- | --- |
| T1 | 能力映射抽取 | 完成 | — | `adapter.rs` | adapter 断言 + 番剧能力 |
| T2 | PGC 取流端口 | 完成 | — | `client.rs`、`mod.rs` | 端口实现与解析用例 |
| T3 | 请求与执行器字段 | 完成 | — | `services/{video,audio}` | 编译 + executor 测试 |
| T4 | 解析取流与权限 | 完成 | T1–T3 | `parser.rs` | E003/E004/E006 + 成功用例 |
| T5 | 命名与任务字段 | 完成 | — | `filename.rs`、`manager.rs` | 命名/manager 单测 |
| T6 | 回归与证据 | 完成 | T1–T5 | 文档 | 六条命令 |

## 关键检查点

- 权限判定先于 PGC 取流，且解析与下载共用同一函数。
- `container_kind=season` 命名对首成员也带序号。
- UGC 请求未携带 `source` 时默认 `ugc`（旧数据兼容）。

## 清理清单

- 模块 4：空 `ownerName`、`durationSeconds=0`、可空 `bvid` 的展示；不可下载单元的置灰与原因。
