# 任务板：合集/番剧元数据解析

| ID | 任务 | 状态 | 依赖 | 主要文件 | 验证 |
| --- | --- | --- | --- | --- | --- |
| T1 | raw 模型与 PGC envelope | 完成 | — | `raw.rs` | 反序列化单测 |
| T2 | PGC 端口方法 | 完成 | T1 | `client.rs`、`mod.rs` | URL/错误单测 |
| T3 | 两个适配函数 | 完成 | T1 | `adapter.rs` | 三条单测 |
| T4 | 解析分派与 requestedCid | 完成 | T2、T3 | `parser.rs`、`models/parse.rs` | 三路径测试 |
| T5 | 前端契约与默认选中 | 完成 | T4 | `contracts.ts`、`store.ts` | store 测试 |
| T6 | 测试夹具与回归 | 完成 | T1–T5 | 测试/夹具 | 全量命令 |

## 关键检查点

- 合集成员 `pages` 为空时必须合成单单元，不能丢成员。
- 番剧 `section_type=1` 必须被丢弃，且不得导致 `E004`（除非全部为预告）。
- `duration` 1203160ms → 1203s。
- `ep` 输入 `requestedCid` 必须命中该集首单元。
- 删除占位分支后，`ep` 不再返回 `E004`。

## 清理清单

- 模块 3 合入时补 PGC 能力字段与取流权限复核。
