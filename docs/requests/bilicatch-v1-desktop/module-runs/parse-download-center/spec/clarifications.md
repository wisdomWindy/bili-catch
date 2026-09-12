# 规格澄清：解析与下载中心

- “加入队列”在本模块定义为生成稳定 `DownloadTaskDraft`、写入会话交接队列并导航 `/tasks`；任务持久化、进度与下载状态属于下一模块。
- 目录更改的原生对话框属于 system-release；本模块展示设置默认目录并明确标注暂不可更改，不伪造目录选择。
- authentication 尚未实现时 parser 使用匿名上下文；E005 仍提供登录跳转，后续认证模块只替换认证端口。
- 外部 B 站精确字段不是前端契约。实现以 Rust fixture 固定 raw adapter，并保留 opt-in 公网 smoke test。
