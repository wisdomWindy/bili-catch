# BiliCatch

BiliCatch 是一个基于 Tauri 2、Vue 3、TypeScript 和 Rust 的哔哩哔哩桌面工具，面向 Windows 和 macOS 桌面使用场景，支持视频/音频下载、本地音视频转写、任务管理和 GitHub Releases 自动更新。

## 功能概览

- 支持视频链接、b23.tv 短链、BV 号和 AV 号解析。
- 支持视频 + 音频、仅视频、仅音频三种下载模式。
- 视频下载支持清晰度与编码选择；音频支持 MP3、M4A 和 FLAC 输出（能力由源视频和登录状态决定）。
- 支持 B 站二维码登录，登录会话仅保存在当前进程内，应用退出后自动清除，不写入任务记录或日志。
- 下载任务支持排队、暂停、恢复、取消、重试、删除和完成记录清理。
- 使用 whisper.cpp 在本机完成音频和视频转写，媒体内容与转写文本不会上传到转写服务。
- 支持 Tiny、Base、Small、Medium 和 Large v3 模型按需下载，并在模型落盘前校验 SHA-256。
- 转写结果支持分段预览和编辑，并可导出 TXT 或 SRT；简体中文模式会将输出统一为简体中文。
- 下载完成后可按设置自动转写，下载与转写任务可在同一任务列表中查看和控制。
- 任务状态和设置可持久化恢复，断电或重启后可继续处理可恢复任务。
- 设置页支持下载目录、临时目录、并发数、主题、语言、通知和关闭行为配置。
- 默认下载目录、临时目录和语音模型目录分别为安装目录下的 `download`、`temp` 和 `models/whisper`；已有有效的自定义目录不会被覆盖。
- Windows 与 macOS 构建内置 FFmpeg 和 whisper-cli sidecar，分别用于媒体处理和本地语音识别。
- 更新器使用 GitHub Releases 的 latest.json，更新包安装前会校验 Tauri updater 签名。

## 下载

当前稳定版本为 [v0.2.0](https://github.com/wisdomWindy/bili-catch/releases/tag/v0.2.0)：

| 平台 | 系统要求 | 安装包 |
| --- | --- | --- |
| Windows x64 | Windows 10/11 | [NSIS 安装包](https://github.com/wisdomWindy/bili-catch/releases/download/v0.2.0/BiliCatch_0.2.0_Windows_x64-setup.exe) |
| macOS Intel | macOS 11 或更高版本 | [Intel DMG](https://github.com/wisdomWindy/bili-catch/releases/download/v0.2.0/BiliCatch_0.2.0_macOS_Intel.dmg) |
| macOS Apple Silicon | macOS 11 或更高版本 | [Apple Silicon DMG](https://github.com/wisdomWindy/bili-catch/releases/download/v0.2.0/BiliCatch_0.2.0_macOS_AppleSilicon.dmg) |

Windows 仅发布 NSIS 安装包，不发布 MSI。所有历史版本和更新说明可在 [GitHub Releases](https://github.com/wisdomWindy/bili-catch/releases) 查看。

## 安装说明

- Windows 安装包目前未配置 Authenticode 证书，首次安装可能出现 SmartScreen“未知发布者”提示。请确认下载地址来自本仓库后，再选择“更多信息 → 仍要运行”。
- macOS 安装包目前未配置 Apple Developer 签名与公证。若首次启动被系统阻止，请确认安装包来自本仓库，然后在“系统设置 → 隐私与安全性”中允许打开。
- 应用启动后会通过签名的更新清单检查新版本；更新包下载完成后仍会校验 Tauri updater 签名。

## 安装使用

下载对应平台和架构的安装包并运行。启动后：

1. 在“下载中心”粘贴 B 站视频链接或 BV/AV 编号。
2. 等待解析完成，选择分 P、视频清晰度、编码或音频格式。
3. 点击“添加到下载队列”，在“任务列表”查看进度并控制任务。
4. 需要访问登录内容时，从右上角进入登录页，用 B 站 App 扫描二维码。
5. 在“设置”中修改下载目录、临时目录和并发策略。

本地转写流程：

1. 打开“音频转文本”，在“模型管理”中下载所需模型；默认模型为 Base。
2. 选择一个音频或视频文件，设置语言、输出格式、时间戳和 GPU 模式。
3. 点击“开始转写”，在当前页面或“任务列表”查看进度。
4. 转写完成后可编辑分段内容、复制全文，或导出 TXT/SRT 文件。

模型文件不会打包进安装程序，而是在用户明确操作后下载到模型目录。可在“设置”中更换模型目录或配置 HTTPS 模型下载源；更换目录不会自动搬迁已有模型。

请遵守 Bilibili 服务条款、版权要求和当地法律法规。BiliCatch 不绕过付费、DRM 或平台访问控制。

## 开发环境

### 前置依赖

- Node.js LTS（建议使用当前 LTS 版本）。
- npm。
- Rust stable 和 Cargo。
- Windows 开发需要 Visual Studio C++ Build Tools、Windows SDK 和 WebView2 Runtime。
- Git LFS（用于拉取仓库中的 FFmpeg sidecar）。

克隆仓库后，如果本机启用了 Git LFS，执行：

~~~bash
git lfs install
git lfs pull
~~~

### 安装依赖

~~~bash
npm ci
~~~

### 启动开发环境

只启动前端 Vite：

~~~bash
npm run dev
~~~

启动完整 Tauri 桌面应用：

~~~bash
npm run tauri dev
~~~

Vite 开发服务默认地址为 http://localhost:1420。完整桌面应用应优先使用 `npm run tauri dev`，这样可以验证 Rust 命令、文件系统、FFmpeg 和 whisper-cli sidecar 集成。

## 验证与构建

提交代码前建议运行以下门禁：

~~~bash
npm run typecheck
npm test
npm run build
cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check
cargo test --manifest-path src-tauri/Cargo.toml --all-targets -j 1
~~~

构建 Windows NSIS 安装包：

~~~bash
npm run tauri build -- --bundles nsis
~~~

前端静态产物位于 dist/，Tauri 构建产物位于 src-tauri/target/release/bundle/。src-tauri/target/、dist/ 和临时目录均已加入 .gitignore，不要提交到仓库。

## 本地 Sidecar

Windows 构建使用仓库内的 FFmpeg 和 whisper-cli sidecar；macOS 发布任务从固定源码构建对应架构的 sidecar：

~~~text
src-tauri/binaries/ffmpeg-x86_64-pc-windows-msvc.exe
src-tauri/binaries/ffmpeg-x86_64-apple-darwin
src-tauri/binaries/ffmpeg-aarch64-apple-darwin
src-tauri/binaries/whisper-cli-x86_64-pc-windows-msvc.exe
src-tauri/binaries/whisper-cli-x86_64-apple-darwin
src-tauri/binaries/whisper-cli-aarch64-apple-darwin
src-tauri/binaries/ffmpeg-9.0.1-GPLv3-LICENSE.txt
src-tauri/binaries/whisper.cpp-v1.8.6-MIT-LICENSE.txt
~~~

`src-tauri/tauri.conf.json` 通过 `bundle.externalBin` 配置 `binaries/ffmpeg` 和 `binaries/whisper-cli`。Tauri 会根据目标平台自动匹配带 target triple 的可执行文件。替换 sidecar 时必须保留 target triple 命名，并同步检查许可证、固定版本和实际媒体 smoke test。

## 发布与自动更新

发布流水线位于 `.github/workflows/publish.yml`，推送 `v*` 版本标签时自动运行，也可通过 GitHub Actions 手动选择已有标签重新发布：

~~~bash
git tag v0.1.0
git push origin v0.1.0
~~~

GitHub Actions 需要配置以下 Repository secrets：

- TAURI_SIGNING_PRIVATE_KEY：Tauri updater 私钥，不要提交到仓库。
- TAURI_SIGNING_PRIVATE_KEY_PASSWORD：私钥密码；无密码密钥可为空，但生产环境建议使用密码保护的密钥。

更新检查地址配置为：

~~~text
https://github.com/wisdomWindy/bili-catch/releases/latest/download/latest.json
~~~

Tauri updater 的签名密钥只用于更新元数据和更新包校验，不能替代 Windows Authenticode 或 Apple Developer 签名。发布新版本前应同步更新 `package.json`、`package-lock.json`、`src-tauri/Cargo.toml`、`src-tauri/Cargo.lock`、`src-tauri/tauri.conf.json` 和对应版本标签，并确认 GitHub Release 中包含 Windows NSIS、两种 macOS DMG、三个目标平台的更新包与签名文件，以及 `latest.json`。

## 项目结构

~~~text
src/                         Vue 页面、组件、Pinia store 和 IPC 客户端
src/features/transcription/  转写页面状态、组件、合同和 IPC 服务
src-tauri/src/               Rust commands、服务层和平台适配器
src-tauri/binaries/          FFmpeg、whisper-cli sidecar 与许可证文件
src-tauri/tauri.conf.json    Tauri 窗口、打包和 updater 配置
.github/workflows/            GitHub Actions 发布流水线
docs/requests/                PRD、规格、计划、验证和执行记录
~~~

前端通过 src/services/ipc/ 访问 Tauri 命令和事件；业务规则、下载执行、任务状态机、设置持久化和外部平台请求由 Rust 服务层负责。修改跨端数据结构时，请同时更新 TypeScript contract、Rust DTO 和对应测试。

## 贡献约定

1. 先阅读 biliCatch_PRD.md 和 docs/requests/bilicatch-v1-desktop/ 中对应模块的规格与验收记录。
2. 保持 UI 只消费稳定的 IPC contract，不在组件中直接拼接 Rust 命令或平台路径。
3. 新增行为先补测试，再实现代码；提交前运行上面的前端和 Rust 门禁。
4. 不提交私钥、GitHub token、用户 Cookie、下载产物、src-tauri/target 或未审查的第三方二进制。

## 许可证

本项目代码使用 [MIT License](LICENSE)。FFmpeg 和 whisper.cpp 使用各自随附许可证，详见 `src-tauri/binaries/ffmpeg-9.0.1-GPLv3-LICENSE.txt`、`src-tauri/binaries/whisper.cpp-v1.8.6-MIT-LICENSE.txt`、`docs/third-party/ffmpeg-9.0.1-essentials.md`、`docs/third-party/ffmpeg-9.0.1-macos.md` 和 `docs/third-party/whisper.cpp-v1.8.6.md`。
