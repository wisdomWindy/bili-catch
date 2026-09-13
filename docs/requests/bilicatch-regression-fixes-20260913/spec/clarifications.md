# 规格澄清

## Q1：手动重复解析是否复用上次成功结果？

- 回答：不复用。
- 最终决定：点击解析或 Enter 时强制获取新 metadata/playurl；WBI key 缓存可保留。
- 影响区域：手动解析、前端 result cache、Rust parser result cache。

## Q2：哪种登录成功需要自动返回？

- 回答：只有用户在当前 LoginPage 完成的新扫码流程。
- 最终决定：初始进入时已登录，或 restoring 恢复 authenticated，都留在账户信息页。
- 影响区域：`LoginPage` watcher 和路由测试。

## Q3：普通匿名音频源的判定基准？

- 回答：不用严格 `bandwidth <= 64_000` 代替名义档位判定。
- 最终决定：匿名时允许 Bilibili 返回的普通 lossy DASH audio 候选，选最高可用普通源；排除 FLAC/HiRes。
- 影响区域：`select_audio_source`、video+audio 和 audio-only 执行测试。
