# BiliCatch

BiliCatch 是一个基于 Tauri 2、Vue 3、TypeScript 和 Rust 的哔哩哔哩桌面下载工具，面向 Windows 桌面使用场景，支持视频、音频解析、队列管理和 GitHub Releases 自动更新。

## 功能概览

- 支持视频链接、b23.tv 短链、BV 号和 AV 号解析。
- 支持视频 + 音频、仅视频、仅音频三种下载模式。
- 视频下载支持清晰度与编码选择；音频支持 MP3、M4A 和 FLAC 输出（能力由源视频和登录状态决定）。
- 支持 B 站二维码登录，会话凭据存储在系统凭据库中，不写入任务记录或日志。
- 下载任务支持排队、暂停、恢复、取消、重试、删除和完成记录清理。
- 任务状态和设置可持久化恢复，断电或重启后可继续处理可恢复任务。
- 设置页支持下载目录、临时目录、并发数、主题、语言、通知和关闭行为配置。
- 默认目录为安装目录下的 download 和 temp 子目录；已有有效的自定义目录不会被覆盖。
- Windows 构建内置 FFmpeg sidecar，用于音频转码和视频音画合并。
- 更新器使用 GitHub Releases 的 latest.json，更新包安装前会校验 Tauri updater 签名。

## 当前发布状态

- 当前正式发布目标为 Windows x64 NSIS 安装包。
- GitHub Actions 通过推送 v* 版本标签触发发布，产物上传到 GitHub Releases。
- Windows 安装包目前未配置 Authenticode 证书，首次安装可能出现 SmartScreen“未知发布者”提示。请确认下载地址来自本仓库后，再选择“更多信息 → 仍要运行”。
- macOS 和 Linux 发布任务尚未启用；对应平台的 FFmpeg sidecar 和打包验证完成后再扩展流水线。

## 安装使用

从 [GitHub Releases](https://github.com/wisdomWindy/bili-catch/releases) 下载最新 Windows NSIS 安装包并运行。启动后：

1. 在“下载中心”粘贴 B 站视频链接或 BV/AV 编号。
2. 等待解析完成，选择分 P、视频清晰度、编码或音频格式。
3. 点击“添加到下载队列”，在“任务列表”查看进度并控制任务。
4. 需要访问登录内容时，从右上角进入登录页，用 B 站 App 扫描二维码。
5. 在“设置”中修改下载目录、临时目录和并发策略。

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

Vite 开发服务默认地址为 http://localhost:1420。完整桌面应用应优先使用 npm run tauri dev，这样可以验证 Rust 命令、文件系统和 FFmpeg sidecar 集成。

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

## FFmpeg sidecar

Windows 构建使用以下资源：

~~~text
src-tauri/binaries/ffmpeg-x86_64-pc-windows-msvc.exe
src-tauri/binaries/ffmpeg-9.0.1-GPLv3-LICENSE.txt
~~~

src-tauri/tauri.conf.json 通过 bundle.externalBin 配置 binaries/ffmpeg。Tauri 会根据目标平台自动匹配带 target triple 的可执行文件。替换 FFmpeg 时必须保留 target triple 命名，并同步检查许可证、版本和实际媒体 smoke test。

## 发布与自动更新

发布流水线位于 .github/workflows/publish.yml，仅响应版本标签：

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

Tauri updater 的签名密钥只用于更新元数据和更新包校验，不能替代 Windows 安装包的 Authenticode 证书。发布新版本前应同步更新 package.json、src-tauri/tauri.conf.json 和对应版本标签，并确认 GitHub Release 中包含安装包、签名文件和 latest.json。

## 项目结构

~~~text
src/                         Vue 页面、组件、Pinia store 和 IPC 客户端
src-tauri/src/               Rust commands、服务层和平台适配器
src-tauri/binaries/          FFmpeg sidecar 与许可证文件
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

本项目代码使用 [MIT License](LICENSE)。FFmpeg sidecar 使用其随附许可证，详见 src-tauri/binaries/ffmpeg-9.0.1-GPLv3-LICENSE.txt 和 docs/third-party/ffmpeg-9.0.1-essentials.md。