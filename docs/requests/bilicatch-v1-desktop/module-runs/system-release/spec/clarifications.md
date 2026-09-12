# 系统集成与发布规格澄清

## Question 1

- question: 发布服务 URL、签名公钥和证书由谁提供？
- answer: 当前未提供，作为发布环境输入。
- final decision: 规格实现只校验配置存在与格式，不生成或硬编码密钥；缺失时停止发布验收。
- affected spec area: 更新检查、签名验证、发布流水线。

## Question 2

- question: Windows 是否同时交付 NSIS 与 MSI？
- answer: PRD 同时列出两类目标，默认都纳入配置。
- final decision: 计划同时配置 NSIS/MSI；若发布方只选一个，必须在发布清单中显式记录。
- affected spec area: 平台产物与版本一致性。

## Question 3

- question: macOS notarization 与 Linux 包签名是否已有凭据？
- answer: 当前未提供。
- final decision: 本模块实现配置和校验入口，不伪造签名/公证成功；真实凭据由发布环境注入。
- affected spec area: 外部发布依赖、验收 SR-AC-10。

## Question 4

- question: FFmpeg sidecar 的版本、平台架构、哈希和许可证材料是什么？
- answer: 当前工作区没有可信 sidecar。
- final decision: 仅实现 `externalBin` 注入与 E008 缺失保护；真实二进制兼容性转交执行阶段后的发布验收。
- affected spec area: sidecar manifest、真实 MP4/AAC smoke。

## Question 5

- question: 是否新增系统发布专用前端页面？
- answer: PRD 明确不新增页面。
- final decision: 沿用现有设置入口、托盘菜单、对话框和通知，不增加新路由。
- affected spec area: 页面与模块设计、IPC 范围。

## Question 6

- question: Windows GitHub Releases 是否要求 Authenticode 证书？
- answer: 发布方选择未签名 NSIS `.exe`，接受 SmartScreen 的“未知发布者”提示及部分企业设备可能阻止安装的限制。
- final decision: Windows GitHub Releases 仅构建未签名 NSIS；不配置 PFX、证书指纹、硬件令牌或 Trusted Signing。Tauri updater 私钥仍用于更新元数据和产物签名，不能替代 Windows 证书。
- affected spec area: Windows 发布目标、CI 密钥边界、发布说明与风险交接。
