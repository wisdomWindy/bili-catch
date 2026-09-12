# Video Download Review

## Delivery Unit Identifier

- request: `bilicatch-v1-desktop`
- module: `video-download`
- reviewed artifacts: approved architecture/design, spec, plan, verification evidence

## Blocking Issues

- None.
- All implementation blockers are cleared. The only unavailable external capability, a trusted bundled FFmpeg binary, is explicitly outside this module and remains deferred to `system-release`.

## Non-Blocking Issues

- The production build retains the existing Vite warning for a JavaScript chunk above 500 kB. It does not change runtime behavior or acceptance criteria for this desktop module.
- The video executor now retries one stale-source failure after a fresh source resolve. A broader pause/cancel matrix for every interleaving remains useful, but current shared-control and cleanup tests cover the required contract paths.

## Accepted Risks

- Real FFmpeg stream-copy smoke cannot run in the current workspace because no trusted sidecar is present. Fake-runner argument, cancellation, empty-output, and failure tests pass; packaging and a real MP4 smoke test are owned by `system-release`.
- `StrategyRegistry` retains an active-attempt key until the strategy reports completion through its own lifecycle. The current executor removes keys from its spawned task; registry cleanup remains a small follow-up if future strategies become long-lived or externally managed.

## Follow-Up Items

- In `system-release`, inventory, package, and validate the trusted `ffmpeg.exe` sidecar, then run a real MP4/AAC `-c copy` smoke test and record the evidence.
- Consider adding explicit registry lifecycle cleanup and a dedicated pause/resume/cancel interleaving test matrix when another executor strategy is introduced.

## Spec-Plan Alignment

- **PASS**. The implementation preserves the approved function-complete granularity: exact quality+codec selection, MP4-only video transfer, fresh source resolution, per-track checkpoints, one-connection sequential scheduling, split parallel scheduling, aggregated progress, video-only direct finalize, video-audio Processing plus stream-copy mux, typed E008/E009 failures, and registry routing.
- No new command, event, WebView capability, route, or unsupported output container was introduced.
- The stale-source retry added during review is an implementation completion of the existing `fresh source` requirement and does not expand scope.

## API Integration Findings

- **PASS**. Rust serde contracts remain the source of truth; TypeScript mirrors the public camelCase fields and nullable audio/video draft shape.
- Raw Bilibili response details stay behind infrastructure adapters. URL, cookie, stderr, and credential material are not persisted in tasks, checkpoints, events, or UI errors.
- Shared `MediaSourceCandidate`, `MediaDownloadRequest`, `DownloadControl`, and `ProgressSink` contracts are reused by audio and video. The video executor depends on ports and does not own HTTP, filesystem, or FFmpeg argument construction.
- `npm run typecheck` and Rust contract tests confirm the integration boundary.

## Clean-Code Assessment

- result: **pass**
- The previous oversized executor was split into focused `executor.rs` (312 lines), `track.rs`, `progress.rs`, and `runtime.rs`. Each module has one clear responsibility, and the core production file is below the plan review threshold.
- Ownership is explicit: orchestration owns attempt state, track owns byte-transfer/checkpoint setup, progress owns aggregation, runtime owns `TaskExecutorPort`, and infrastructure owns HTTP/filesystem/process effects.
- No required follow-up for clean-code compliance.

## Design-Pattern Assessment

- result: **pass**
- Ports/adapters are used only at real external boundaries (`VideoSourcePort`, `ByteDownloaderPort`, `VideoWorkspace`, `VideoMuxerPort`, `ExecutionReporterPort`).
- `StrategyRegistry` solves the actual audio/video routing problem, and the progress sink is a small composite for the fixed two-track workflow. No plugin registry, codec-class hierarchy, shell command builder, or pipeline DSL was introduced.
- Pattern use is justified and remains local to the module.

## Code-Context Structural Assessment

- **PASS**. The changed graph stays within the approved boundaries: DownloadPage/store/options and stable task DTOs on the frontend; parser/source/download/workspace/mux/executor/runtime on Rust; no new IPC surface.
- Full Rust tests (73 passed, 1 explicit public-endpoint ignore), frontend tests (176 passed), typecheck, build, `cargo fmt --check`, and `cargo check --all-targets` are green.
- Security scans embodied by the contract tests cover URL allowlists, MIME-kind matching, secret/URL non-persistence, safe error mapping, contained workspace paths, and no shell-based FFmpeg execution.

## Merge Readiness Summary

- decision: **ready for module handoff**
- blockers: **0**
- module status: **completed after review**
- next module: `system-release`
- handoff condition: retain the explicit FFmpeg sidecar defer; do not claim real-binary compatibility until `system-release` supplies trusted inventory and smoke evidence.
