# 验证命令证据：音频独立下载

日期：2026-09-11

## 自动化门禁

- `rtk npm run test -- --run`：45个test files、166个tests，全部通过。
- `rtk npm run build`：`vue-tsc --noEmit`与Vite production build通过；4665 modules；JS 508.89 kB（gzip 160.29 kB），CSS 27.39 kB（gzip 5.66 kB）。构建仅有大于500 kB的非阻断chunk提示。
- `rtk C:/Users/yangjianlin/.cargo/bin/cargo.exe fmt --all -- --check`：通过。
- `rtk C:/Users/yangjianlin/.cargo/bin/cargo.exe check --all-targets -j 1`：通过。
- `rtk C:/Users/yangjianlin/.cargo/bin/cargo.exe test --all-targets -j 1`：66 unit与69 integration通过，共135 passed；1个public Bilibili opt-in smoke ignored。
- 初次并行Rust全量测试出现Windows OS 1455页面文件不足；单作业重跑成功，未掩盖任何测试失败。

## 视觉与交互

- `rtk node scripts/verify-audio-visuals.cjs`：3组场景全部通过。
- 每组 `documentWidth == viewportWidth`，`clippedRegions=0`，`undersizedControls=0`。
- 匿名场景FLAC disabled且标签为“需登录”；authenticated场景FLAC可选，lossless profile只读但可见。
- 截图经人工检查，未发现重叠、截断、空白主内容或不可达操作。

## 静态边界

- `src-tauri/capabilities/default.json` 仅含 `core:default`、`opener:default`、`dialog:allow-open`；未给WebView增加shell/filesystem/http capability。
- Vue扫描未发现对backend `error.message` / `error.details` 的直接渲染；页面只消费本地化稳定code。
- source URL只存在于Rust进程内adapter/execution结构；checkpoint、task contract、event与前端不携带URL或credential。
- FFmpeg使用离散argv与Tokio `Command`，stderr设为null；没有shell字符串或PATH扫描。
- 生产职责尺寸：`executor.rs` 341行；`http_downloader.rs` 第1-317行为生产代码，第318行后为loopback tests；cover/support/runtime/progress均为独立小模块。

## 明确跳过

- 可信FFmpeg inventory输出：`SKIP: no trusted ffmpeg executable is present in the repository resources`。
- 因此未运行real FFmpeg 7三容器/封面smoke；缺sidecar由production adapter映射E008/`FFMPEG_UNAVAILABLE`，最终打包证据归属 `system-release`。
- public Bilibili smoke保持ignored，默认验证不访问公网或使用真实credential。
