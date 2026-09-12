# System Release Evidence Commands

## Rust

```text
rtk C:/Users/yangjianlin/.cargo/bin/cargo.exe fmt --all -- --check
rtk C:/Users/yangjianlin/.cargo/bin/cargo.exe check --all-targets
rtk C:/Users/yangjianlin/.cargo/bin/cargo.exe test --all-targets
```

Result: format and check pass; 73 non-ignored tests pass and one public endpoint smoke is explicitly ignored.

## Frontend

```text
rtk npm run typecheck
rtk npm test -- --run
rtk npm run build
```

Result: 46 test files and 176 tests pass; typecheck/build pass. Vite reports the existing single chunk-size warning.

## External release gate

- Windows x64 FFmpeg sidecar: `src-tauri/binaries/ffmpeg-x86_64-pc-windows-msvc.exe`.
- Archive SHA-256: `49a73bdf0850092a252ac4641d922f3048d63ed113e196cc65ce1e4f7fb33e85`.
- Executable SHA-256: `72a489eccd008c2ec2c0a5856c5c75bc3d8bbfa90166c4566865c246445e6aa3`.
- `ffmpeg -version`: `9.0.1-essentials_build-www.gyan.dev`.
- License: GPLv3; see `src-tauri/binaries/ffmpeg-9.0.1-GPLv3-LICENSE.txt` and `docs/third-party/ffmpeg-9.0.1-essentials.md`.
- Tauri updater keypair: generated locally; private key remains at `C:\Users\yangjianlin\.tauri\bilicatch-updater.key` and is not tracked.
- Tauri updater public key: configured in `src-tauri/tauri.conf.json`; byte-for-byte match verified against `.key.pub`.
- GitHub Releases endpoint: `https://github.com/wisdomWindy/bili-catch/releases/latest/download/latest.json` configured; current HTTP status is `404` because no compatible Release asset has been published yet.
- OS installer signing certificates (Windows/macOS): not provided.
- Target-platform runners: not provided.
- Therefore no real signed installer or MP4/AAC sidecar smoke is claimed yet.
