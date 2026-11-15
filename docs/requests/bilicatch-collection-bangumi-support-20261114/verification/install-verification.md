# 本地打包 / 安装验证记录

日期：2026-11-14　范围：`bilicatch-collection-bangumi-support-20261114` 完成后的本地构建、安装与运行验证。

> 范围约定（2026-11-14 用户指示）：只做**本地**验证；远程/在线行为（B 站真实解析、下载、更新服务、GitHub 资产）不作为本次验收项。此前已执行的在线解析仅作为构建产物的附带证据，不参与本地验收结论。

## 结论

| 项 | 结果 |
| --- | --- |
| 前端生产构建 | 通过 |
| Rust release 编译 | 通过（`bilicatch.exe` 9.2 MB + sidecar） |
| MSI 本地打包 | **通过**：`BiliCatch_0.2.2_x64_en-US.msi`（47.92 MiB） |
| 静默安装（默认 per-machine） | 失败：Error 1925（当前会话无管理员权限） |
| 按用户安装（无管理员） | **通过**：`ALLUSERS=2 MSIINSTALLPERUSER=1` |
| 安装产物校验 | 通过：安装目录、开始菜单快捷方式、ARP 卸载项 |
| 启动已安装应用 | 通过（`D:\program\bilicatch\bilicatch.exe`） |
| 卸载 | 通过：应用文件移除，用户数据保留 |
| NSIS 安装包 | 未产出（GitHub 资产不可达；不在本地验收范围） |
| 远程/在线行为 | 不在本次范围（按用户指示） |

## 1. 打包：绕过 WiX 下载

Tauri 首次打包在下载 WiX 时失败：

```
Downloading https://github.com/wixtoolset/wix3/releases/download/wix3141rtm/wix314-binaries.zip
failed to bundle project: `io: Connection refused`
```

网络诊断：`github.com` 首页可达，但 release 资产地址不可达；本机无代理环境变量（区分：PowerShell 走系统代理可用，Tauri 的 reqwest 直连失败）。

绕过方式（未改仓库配置）：

1. 从 NuGet 获取 WiX 3.14.1（`api.nuget.org` 可达）：
   `https://api.nuget.org/v3-flatcontainer/wix/3.14.1/wix.3.14.1.nupkg`
2. 解包后把 `tools/` 复制到 Tauri 缓存：`%LOCALAPPDATA%\tauri\WixTools314`（`candle.exe`、`light.exe`、扩展 DLL 齐全）。
3. 打包命令：

```powershell
$cfg = "$env:TEMP\bilicatch-build-config.json"   # {"bundle":{"createUpdaterArtifacts":false}}
pnpm tauri build --bundles msi --ignore-version-mismatches --config $cfg
```

结果：

```
Running candle ... main.wxs
Running light ... BiliCatch_0.2.2_x64_en-US.msi
Finished 1 bundle ... (47.92 MiB)
```

说明：
- `--ignore-version-mismatches`：仓库既有 npm/crate 次版本漂移（`tauri 2.11.5` vs `@tauri-apps/api 2.12.4` 等），正式发布前应对齐。
- 临时配置把 `createUpdaterArtifacts` 关掉（缺 `TAURI_SIGNING_PRIVATE_KEY`），未修改仓库配置。
- MSI 未做代码签名。

## 2. 安装

默认安装（per-machine）在非管理员会话失败：

```
Error 1925. You do not have sufficient privileges ... for all users of the machine.
```

按用户安装成功（同一 MSI，无需管理员）：

```powershell
msiexec /i BiliCatch_0.2.2_x64_en-US.msi /qn /norestart ALLUSERS=2 MSIINSTALLPERUSER=1
```

安装后校验：

- 安装目录 `D:\program\bilicatch\`：`bilicatch.exe`、`ffmpeg.exe`、`whisper-cli.exe`、`sherpa-onnx-offline.exe`、DLL、`licenses/`、`models/`。
- 开始菜单：`%APPDATA%\Microsoft\Windows\Start Menu\Programs\BiliCatch.lnk` 与 `Programs\BiliCatch\BiliCatch.lnk`。
- 卸载项：`HKCU/HKLM ...\Uninstall\{DB947529-5CDD-4254-B103-AE26F2BE8C0C}` → `MsiExec.exe /X{DB947529-5CDD-4254-B103-AE26F2BE8C0C}`。
- 启动已安装应用正常（版本 0.2.2、下载中心可用）：[installed-launch](docs/requests/bilicatch-collection-bangumi-support-20261114/verification/evidence/bilicatch-installed-launch.png)。

## 3. 卸载

```powershell
msiexec /x BiliCatch_0.2.2_x64_en-US.msi /qn /norestart MSIINSTALLPERUSER=1 ALLUSERS=2
```

结果：exit 0，应用文件与 DLL 移除，`download/`、`models/`、`temp/` 用户数据保留（符合预期）。日志中 `Setting File Security failed for D:\Config.Msi\...` 是回滚脚本权限告警，不影响卸载完成。

## 4. 重要发现：机器上原有安装

`D:\program\bilicatch` 创建于 2026-09-12，内含 `uninstall.exe`（2026-09-16）与 `Uninstall BiliCatch.lnk`，并在 `HKCU\...\Uninstall\BiliCatch` 有一条指向 `uninstall.exe` 的记录。这说明**本机此前已用 NSIS 安装过 BiliCatch**，本次 MSI 安装沿用了其安装目录（MSI 通过注册表 `InstallDir` 决定路径）。

由此产生两点：

1. 本次安装实际是“覆盖/升级”既有安装，而非完全干净环境；干净环境下的安装路径与快捷方式行为需另机确认。
2. 卸载后残留了旧 NSIS 的 `uninstall.exe` 与 HKCU 卸载项；当前已用 MSI 重新安装，应用可用，但这条旧记录会误导（点击它会用旧卸载器删文件）。

建议清理（需你确认，我不擅自删注册表）：删除 `HKCU\Software\Microsoft\Windows\CurrentVersion\Uninstall\BiliCatch` 与 `D:\program\bilicatch\uninstall.exe`，或等 NSIS 安装包可用后重新安装覆盖。

## 5. 运行验证（release 包内，非安装目录）

安装前已用 release 可执行文件做过真实解析：

- 合集 `BV1kw8b6bEeL`：7 个 `EP`、标题/时长、默认选中 EP1、480P。
- 番剧 `ep733316`：200 个 `EP`、480P 能力。

截图：[collection](docs/requests/bilicatch-collection-bangumi-support-20261114/verification/evidence/bilicatch-release-collection-parse.png)、[bangumi](docs/requests/bilicatch-collection-bangumi-support-20261114/verification/evidence/bilicatch-release-bangumi-parse.png)、[launch](docs/requests/bilicatch-collection-bangumi-support-20261114/verification/evidence/bilicatch-release-launch.png)。构建日志：[tauri-build.log](docs/requests/bilicatch-collection-bangumi-support-20261114/verification/evidence/tauri-build.log)。

## 6. 仍未验证

- NSIS 安装包（GitHub `tauri-apps/binary-releases` 资产不可达）。
- 管理员权限下的默认 per-machine 安装路径与升级行为。
- 真实下载落盘（避免验证消耗数百 MB 与带宽）。
- 更新器签名与自动更新。

## 7. 复现命令

```powershell
# 1) 准备 WiX 缓存（仅一次；网络可用时可省略）
#    nuget: wix 3.14.1 -> tools/ -> %LOCALAPPDATA%\tauri\WixTools314

# 2) 打包
pnpm tauri build --bundles msi --ignore-version-mismatches --config <临时配置>

# 3) 无管理员安装
msiexec /i src-tauri\target\release\bundle\msi\BiliCatch_0.2.2_x64_en-US.msi /qn `
  ALLUSERS=2 MSIINSTALLPERUSER=1

# 4) 卸载
msiexec /x {DB947529-5CDD-4254-B103-AE26F2BE8C0C} /qn MSIINSTALLPERUSER=1 ALLUSERS=2
```

正式发布仍建议：对齐 npm/crate 版本、配置签名私钥、在能访问 GitHub 资产的环境构建 MSI/NSIS 并做干净机安装。
