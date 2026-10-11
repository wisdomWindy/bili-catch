# whisper.cpp v1.8.6

BiliCatch distributes the official Windows x64 CPU build of `whisper-cli` from
the [`whisper.cpp` v1.8.6 release](https://github.com/ggml-org/whisper.cpp/releases/tag/v1.8.6).

- Upstream project: `ggml-org/whisper.cpp`
- Fixed source revision: `23ee035`
- Distributed asset: `whisper-bin-x64.zip`
- Runtime files: `whisper-cli.exe`, `whisper.dll`, `ggml.dll`, `ggml-base.dll`, `ggml-cpu.dll`
- License: MIT
- Bundled license text: `src-tauri/binaries/whisper.cpp-v1.8.6-MIT-LICENSE.txt`

Speech models are not included in the application bundle. They are downloaded
only after the user requests a model, verified against the versioned manifest,
and stored in the configured local model directory.
