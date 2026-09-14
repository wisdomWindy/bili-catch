# FFmpeg 9.0.1 macOS Sidecars

- FFmpeg source: `https://ffmpeg.org/releases/ffmpeg-9.0.1.tar.xz`.
- FFmpeg source SHA-256: `cf38e0e28c7e5605942c4a77755349b0145804a397af37eb1fb4c77cb237f635`.
- LAME source: `https://downloads.sourceforge.net/project/lame/lame/3.100/lame-3.100.tar.gz`.
- LAME source SHA-256: `ddfe36cab873794038ae2c1210557ad34857a4b6bdc515785d1da9e175b1da1e`.
- Targets: `x86_64-apple-darwin` and `aarch64-apple-darwin`.
- Sidecar names: `ffmpeg-x86_64-apple-darwin` and `ffmpeg-aarch64-apple-darwin`.
- Minimum supported system: macOS 11.0.
- FFmpeg configuration enables GPLv3 (`--enable-gpl --enable-version3`) and statically links LAME for `libmp3lame` output. It disables autodetected third-party libraries and rejects non-system dynamic dependencies.
- License: GPLv3. The distributed license text is `src-tauri/binaries/ffmpeg-9.0.1-GPLv3-LICENSE.txt`.
- LAME is licensed under LGPL-2.0-or-later. Its exact source archive, including the upstream `COPYING` and `LICENSE` notices, is attached to every Release.

The release workflow compiles each sidecar on its matching native GitHub-hosted macOS runner, verifies its architecture, deployment target, dependencies and encoder inventory, then mounts the generated DMG and uses its packaged sidecar for real stream-copy mux and AAC-to-MP3 smoke tests. The exact source archives are uploaded with every Release.
