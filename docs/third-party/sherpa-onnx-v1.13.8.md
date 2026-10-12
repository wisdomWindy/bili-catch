# sherpa-onnx v1.13.8 and FireRedASR2

BiliCatch distributes the official `sherpa-onnx-offline` CPU runtime from the
[`sherpa-onnx` v1.13.8 release](https://github.com/k2-fsa/sherpa-onnx/releases/tag/v1.13.8).
The runtime is used only for the optional FireRedASR2 CTC INT8 model.

- Runtime project: `k2-fsa/sherpa-onnx`
- Runtime version: `v1.13.8`
- Runtime license: Apache-2.0
- Embedded inference runtime: ONNX Runtime v1.28.2, MIT
- Windows archive SHA-256: `d2ce464a9e28d9128ab1fd15720df6f0e5e19053f725035d630af20a09c17e36`
- Windows executable SHA-256: `de366889e8cf3cde93121f2d11296749769c8624995030a44b86316f8b0c6779`

The model is converted from
[`FireRedTeam/FireRedASR2`](https://github.com/FireRedTeam/FireRedASR2S), which
is licensed under Apache-2.0. Model weights are not distributed in the
installer. BiliCatch downloads the selected model on demand and verifies the
following fixed artifacts before use:

- Model repository: `csukuangfj2/sherpa-onnx-fire-red-asr2-ctc-zh_en-int8-2026-02-25`
- Model file: `model.int8.onnx`
- Model size: `775861420` bytes
- Model SHA-256: `ca3dbabd82170110cc0b343c2890866d449984bc9cd92b9a18371ff80a81bb99`
- Token table SHA-256: `1bc613de2112d257e61a349c3e72d1b1a9cf19c33d3ca954197ad2171e5ea07b`

Bundled license texts:

- `src-tauri/binaries/sherpa-onnx-v1.13.8-Apache-2.0-LICENSE.txt`
- `src-tauri/binaries/onnxruntime-v1.28.2-MIT-LICENSE.txt`
