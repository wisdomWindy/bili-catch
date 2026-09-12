# GitHub Releases 发布

仓库：`https://github.com/wisdomWindy/bili-catch`

## Windows distribution policy

- GitHub Releases only publishes the Windows NSIS `.exe` installer.
- The installer is intentionally not Authenticode-signed and does not require a Windows certificate, PFX, certificate thumbprint, hardware token, or Trusted Signing account.
- Windows SmartScreen can show `Unknown publisher`. Users must verify that the installer came from this repository, then use `More info -> Run anyway` if they choose to continue.
- Managed enterprise devices may block unsigned installers. This channel does not promise installation on devices whose policy requires trusted publishers.
- `TAURI_SIGNING_PRIVATE_KEY` remains mandatory. It signs Tauri updater metadata and artifacts; it is not a Windows certificate and does not remove SmartScreen warnings.

## Required repository secrets

- `TAURI_SIGNING_PRIVATE_KEY`: contents of the Tauri updater private key.
- `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`: password for that key. Leave empty only for the current local development key; use a password-protected key for production.

The Windows workflow does not store either secret in the repository. The built updater artifacts and `latest.json` are uploaded to the GitHub Release by `tauri-apps/tauri-action`.

No Windows code-signing secret is used by this workflow.

## Release procedure

1. Update `src-tauri/tauri.conf.json` version to the next SemVer value.
2. Commit and push the change.
3. Push a matching tag, for example `v0.2.0`.
4. GitHub Actions runs `.github/workflows/publish.yml` and publishes the Windows NSIS bundle plus updater signature artifacts.
5. The configured endpoint becomes available at `/releases/latest/download/latest.json` after the Release is published.

macOS and Linux jobs are intentionally not enabled until their FFmpeg sidecars and platform release inputs are available.
