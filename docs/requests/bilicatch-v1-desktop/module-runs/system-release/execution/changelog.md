# 执行记录：系统集成与发布

## 执行约束

- 规格与计划已批准；当前按 SR-01 -> SR-06 串行收口。
- 不引入未锁定的 updater/notification 第三方依赖，不伪造签名或 sidecar 产物。
- 所有实现先以契约测试验证，再接入 Tauri 组合根。

## SR-01

- status: `completed`
- implemented: enabled Tauri built-in `tray-icon`; registered show/check-update/quit menu; close event now honors settings and active-task confirmation.
- verification: `cargo check --all-targets`, system contract tests.

## SR-02

- status: `completed`
- implemented: stable task notification mapping and `system:close-confirmation-required` event boundary; no raw URL/cookie/stderr fields enter the event.
- verification: notification/privacy contract test.
- limitation: OS-native notification plugin remains optional external integration; current workspace has no approved plugin dependency.

## SR-03

- status: `completed`
- implemented: typed updater state machine with duplicate-check guard, signature verification gate, confirmation gate, install success/failure transitions.
- verification: update state contract tests.

## SR-04

- status: `in_progress`
- implemented: typed `SidecarManifest` and `ReleaseArtifact` validation, Windows x64 `externalBin` packaging input, and sidecar path resolution fallback.
- evidence: FFmpeg 9.0.1 essentials sidecar is present, SHA-256 verified, and `ffmpeg -version` passes.
- remaining: organization approval for the GPLv3 build, non-Windows sidecars, and real MP4/AAC smoke.

## Updater signing input

- Generated a Tauri updater keypair outside the workspace at `C:\Users\yangjianlin\.tauri\bilicatch-updater.key`.
- Configured the public key and enabled `bundle.createUpdaterArtifacts`.
- The private key is unencrypted for local development only; production CI must replace it with a password-protected key stored as a secret.
- OS installer certificates, updater endpoint, and target-platform runners remain external release inputs.

## GitHub Releases endpoint

- Configured the static updater endpoint for `wisdomWindy/bili-catch`:
  `https://github.com/wisdomWindy/bili-catch/releases/latest/download/latest.json`.
- Added `tauri-plugin-updater` and the `updater:default` capability permission.
- Endpoint currently returns `404` until a Release uploads the generated `latest.json` and signed platform artifacts.

## Update action and CI

- Replaced the deferred release adapter with the real Tauri updater check/download/install flow.
- Added an “Install update” action in Settings; installation is followed by a relaunch where supported.
- Wired the tray “Check for updates” event to the same Settings update flow.
- Added `.github/workflows/publish.yml` for tag-driven Windows NSIS Releases with updater signing secrets.

## Windows unsigned NSIS decision

- The release owner selected GitHub Releases with an unsigned Windows NSIS `.exe`.
- The workflow has no Authenticode certificate, PFX, certificate thumbprint, hardware token, or Windows signing command dependency.
- Release notes now disclose the expected SmartScreen `Unknown publisher` warning and the possibility that managed enterprise devices block the installer.
- Tauri updater signing remains enabled and mandatory; it authenticates updater artifacts but does not establish a trusted Windows publisher.

## SR-05 / SR-06

- status: `pending`
- Windows Authenticode credentials are no longer a release input for the selected GitHub NSIS channel.
- remaining external inputs: the GitHub updater signing secret and first real Release smoke; macOS/Linux runners and sidecars remain deferred.
