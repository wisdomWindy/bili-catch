param(
  [string]$Version = "0.2.2",
  [string]$Configuration = "release"
)

$ErrorActionPreference = "Stop"

$root = Split-Path -Parent $PSScriptRoot
$release = Join-Path $root "src-tauri\target\$Configuration"
$payload = Join-Path $root "src-tauri\target\nsis-payload"
$output = Join-Path $root "src-tauri\target\$Configuration\bundle\nsis\BiliCatch_${Version}_x64-setup.exe"
$makensis = Join-Path $env:LOCALAPPDATA "tauri\NSIS\Bin\makensis.exe"
$script = Join-Path $root "src-tauri\nsis\installer.nsi"
$icon = Join-Path $root "src-tauri\icons\icon.ico"

if (-not (Test-Path $makensis)) {
  throw "makensis not found at $makensis. Seed %LOCALAPPDATA%\tauri\NSIS with NSIS 3.13 first."
}
if (-not (Test-Path (Join-Path $release "bilicatch.exe"))) {
  throw "Missing release build. Run: pnpm tauri build --no-bundle"
}

if (Test-Path $payload) { Remove-Item $payload -Recurse -Force }
New-Item -ItemType Directory -Force $payload | Out-Null
$appFiles = @(
  "bilicatch.exe",
  "ffmpeg.exe",
  "whisper-cli.exe",
  "sherpa-onnx-offline.exe",
  "ggml.dll",
  "ggml-base.dll",
  "ggml-cpu.dll",
  "whisper.dll",
  "fire-red-asr2-tokens.txt"
)
foreach ($name in $appFiles) {
  Copy-Item (Join-Path $release $name) $payload -Force
}
$licenses = Join-Path $payload "licenses"
New-Item -ItemType Directory -Force $licenses | Out-Null
Get-ChildItem (Join-Path $root "src-tauri\binaries") -Filter "*LICENSE*.txt" |
  Copy-Item -Destination $licenses -Force
foreach ($dataDir in @("download", "models", "temp")) {
  New-Item -ItemType Directory -Force (Join-Path $payload $dataDir) | Out-Null
}

New-Item -ItemType Directory -Force (Split-Path $output) | Out-Null
& $makensis "/DVERSION=$Version" "/DPAYLOAD=$payload" "/DOUTFILE=$output" "/DICONFILE=$icon" $script
if (-not (Test-Path $output)) { throw "NSIS build failed" }

Get-Item $output | Select-Object FullName, Length
