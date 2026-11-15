# 验证：Windows 仅 NSIS

## 1. 平台配置

新增 `src-tauri/tauri.windows.conf.json`：

```json
{ "bundle": { "targets": ["nsis"] } }
```

构建日志只出现 NSIS 步骤（`Patching ... bundle type information: nsis` → `Running makensis ...`），不再生成 MSI。

## 2. 工具链恢复（为什么之前能打、这次一开始不能）

之前能打包，是因为 NSIS 工具链曾缓存在 Codex 会话的沙箱目录：

```
C:\Users\yangjianlin\AppData\Local\Packages\OpenAI.Codex_2p2nqsd0c76g0\LocalCache\Local\tauri\NSIS
```

本次一开始 `%LOCALAPPDATA%\tauri` 为空，Tauri 试图重新下载 `nsis_tauri_utils.dll`，GitHub Release 不可达 → 失败。

恢复方式：

1. 从上述旧缓存取 `nsis_tauri_utils.dll`，SHA1 校验 = `75197FEE3C6A814FE035788D1C34EAD39349B860`（与 Tauri 期望一致）。
2. NSIS 3.13 主程序用 NuGet `NSIS-Tool 3.13.0`（旧缓存的 `NSIS.exe` 版本不符，Tauri 会判 outdated）。
3. 合并到 `%LOCALAPPDATA%\tauri\NSIS`：NSIS 3.13 全量文件 + 官方插件 DLL。

之后 `pnpm tauri build` 直接通过校验，无需再联网、无需 `--ignore` 之外的绕过（版本漂移仍用 `--ignore-version-mismatches`）。

## 3. 官方 NSIS 产物

```
Running makensis to produce ...\bundle\nsis\BiliCatch_0.2.2_x64-setup.exe
Finished 1 bundle ... (34.91 MiB)
```

产物：`src-tauri/target/release/bundle/nsis/BiliCatch_0.2.2_x64-setup.exe`

## 4. 本地安装/卸载实测（官方安装包）

| 步骤 | 结果 |
| --- | --- |
| `setup.exe /S /D=%LOCALAPPDATA%\BiliCatchNsisOfficial` | exit 0；安装目录含 exe、ffmpeg/whisper/sherpa、DLL、licenses、uninstall.exe |
| 开始菜单快捷方式 | 已创建 |
| HKCU 卸载项 | `BiliCatch 0.2.2`，UninstallString 指向安装目录 `uninstall.exe` |
| 启动安装后的 `bilicatch.exe` | 进程正常启动 |
| `uninstall.exe /S` | exit 0；程序文件、快捷方式、卸载项移除 |
| 用户数据 | 仅保留 `download/ models/ temp/` |

## 5. 兜底链路（保留）

若换机器又没有工具链缓存，可继续用离线脚本生成等价 setup：

- `src-tauri/nsis/installer.nsi`
- `scripts/build-nsis.ps1`（NSIS 3.13 + 自组 payload）
- 产物同名 `BiliCatch_0.2.2_x64-setup.exe`

官方管道可用时优先用官方产物（含 WebView2 引导、标准升级/安装范围逻辑）。

## 6. 未验证 / 限制

- 安装包未做代码签名，SmartScreen 可能提示。
- 未在全新 Windows 用户/干净虚拟机验证“无 WebView2 运行时”场景。
- 真实下载落盘未纳入本次安装验证。
