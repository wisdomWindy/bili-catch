# GitHub Releases 发布

仓库：`https://github.com/wisdomWindy/bili-catch`

## Desktop distribution policy

- GitHub Releases publishes Windows x64 NSIS, macOS Intel DMG and macOS Apple Silicon DMG installers.
- The installer is intentionally not Authenticode-signed and does not require a Windows certificate, PFX, certificate thumbprint, hardware token, or Trusted Signing account.
- Windows SmartScreen can show `Unknown publisher`. Users must verify that the installer came from this repository, then use `More info -> Run anyway` if they choose to continue.
- The macOS DMGs are not Apple Developer signed or notarized. Gatekeeper can block first launch until the user explicitly allows the app in System Settings.
- Managed enterprise devices may block unsigned installers. This channel does not promise installation on devices whose policy requires trusted publishers.
- `TAURI_SIGNING_PRIVATE_KEY` remains mandatory. It signs Tauri updater metadata and artifacts; it is not a Windows certificate and does not remove SmartScreen warnings.

## FFmpeg release inputs

- Windows x64 uses the Git LFS tracked FFmpeg 9.0.1 sidecar and verifies its size and SHA-256 before bundling.
- macOS builds compile FFmpeg 9.0.1 and LAME 3.100 from pinned source archives on the matching native runner.
- macOS builds target macOS 11.0 or later and verify source SHA-256 values, Mach-O architecture, deployment target, dynamic dependencies and `libmp3lame`.
- Each generated DMG is mounted before publication, and its packaged FFmpeg executable must pass real stream-copy mux and AAC-to-MP3 smoke tests.
- The FFmpeg and LAME source archives are attached to each Release alongside the installers.

## Required repository secrets

- `TAURI_SIGNING_PRIVATE_KEY`: contents of the Tauri updater private key.
- `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`: password for that key. Leave empty only for the current local development key; use a password-protected key for production.

The workflow does not store either secret in the repository. The built updater artifacts and `latest.json` are uploaded to the GitHub Release by `tauri-apps/tauri-action`.

No Windows code-signing secret is used by this workflow.

## Release procedure

1. Update `src-tauri/tauri.conf.json` version to the next SemVer value.
2. Commit and push the change.
3. Push a matching tag, for example `v0.2.0`.
4. GitHub Actions builds into a draft Release, then validates Windows x64, macOS Intel and macOS Apple Silicon bundles, updater signatures, source archives and `latest.json`.
5. The workflow publishes the draft only after every validation passes; the configured endpoint then becomes available at `/releases/latest/download/latest.json`.
