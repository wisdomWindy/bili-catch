# 请求：Windows 仅打 NSIS，不打 MSI

## 标识

`bilicatch-windows-nsis-only-20261114`

## 来源

用户直接指令（2026-11-14）：“windows不打包mis，直达包NSIS”（MSI 笔误）；随后澄清“NSIS 不是打包一个 exe 文件”——即需要完整安装包。

## 目标

Windows 构建只产出 NSIS 安装包（setup.exe），不再生成 MSI；安装包必须封装完整应用（主程序、sidecar、DLL、许可证），并具备安装/卸载/快捷方式能力。

## 实现

1. 平台配置 `src-tauri/tauri.windows.conf.json`：`bundle.targets = ["nsis"]`，Windows 不再默认打 MSI。
2. 官方 Tauri NSIS 管道：从旧 Codex 会话缓存中恢复 `nsis_tauri_utils.dll`（SHA1 校验通过）并与 NSIS 3.13 合并，`pnpm tauri build` 已能直接产出官方 NSIS 安装包。
3. 产物：`src-tauri/target/release/bundle/nsis/BiliCatch_0.2.2_x64-setup.exe`。

## 验收

- Windows 构建目标只有 NSIS（日志无 MSI/WiX 步骤）。
- 安装包是可执行的 setup（36.5 MB），安装后含全部运行文件与快捷方式。
- 静默安装、启动、静默卸载均通过，用户数据保留。

详见 `verification/verification.md`。
