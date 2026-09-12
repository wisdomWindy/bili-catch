# Video Download Verification

## Delivery Unit

- request: `bilicatch-v1-desktop`
- module: `video-download`
- spec: `spec/spec.md`
- plan: `plan/plan.md`
- implementation state: VID-01..VID-07 completed; VID-08 verification

## Acceptance Coverage

| Acceptance | Verification method | Result | Evidence | Follow-up / handoff |
| --- | --- | --- | --- | --- |
| VID-AC-01 | TS/Rust variant contracts, store/component tests | PASS | `src/features/download-center/video-options.test.ts`; `video_executor`/parse tests | none |
| VID-AC-02 | auth refresh store test and DownloadPage watcher | PASS | `src/features/download-center/store.test.ts`; `src/pages/DownloadPage.test.ts` | none |
| VID-AC-03 | task validation and filename tests | PASS | `tests/task_rules.rs`; frontend store tests | none |
| VID-AC-04 | parser fresh source tests and URL non-persistence review | PASS | `src/services/parser.rs` tests; video source adapter | none |
| VID-AC-05 | video source AAC/M4A selection tests | PASS | `src/services/parser.rs`; `tests/video_source_contract.rs` | none |
| VID-AC-06 | redirect/allowlist and MIME-kind guards | PASS | Bilibili client tests; `tests/media_download_contract.rs`; HTTP tests | none |
| VID-AC-07 | video-only fake E2E and workspace finalize | PASS | `tests/video_executor.rs`; `tests/video_workspace.rs` | none |
| VID-AC-08 | budget unit and dual fake E2E | PASS | `services::video::executor` unit; `tests/video_executor.rs` | none |
| VID-AC-09 | per-track workspace/checkpoint and shared byte tests | PASS | `tests/video_workspace.rs`; HTTP downloader tests | none |
| VID-AC-10 | aggregate progress tests and executor reporting | PASS | `tests/video_progress.rs`; `tests/video_executor.rs` | none |
| VID-AC-11 | control propagation implementation review; audio regression | PASS | shared `DownloadControl`/executor control paths; audio executor tests | broader pause matrix remains a follow-up test opportunity |
| VID-AC-12 | cancel cleanup implementation and mux cancel fake | PASS | mux fake tests; workspace cleanup; task manager cancel tests | real process smoke deferred with sidecar |
| VID-AC-13 | discrete argv and Processing state tests | PASS | `tests/video_mux.rs`; `tests/video_executor.rs` | none |
| VID-AC-14 | non-empty MP4 input/output and collision tests | PASS | `tests/video_mux.rs`; `tests/video_workspace.rs` | none |
| VID-AC-15 | typed E008/E009 paths and workspace preservation review | PASS | mux/HTTP tests; `VideoWorkspace::discard_processed` | startup sidecar inventory in system-release |
| VID-AC-16 | TaskManager retry/stale update regression | PASS | `tests/task_manager.rs`; executor stale-source paths | none |
| VID-AC-17 | StrategyRegistry mode routing unit and production composition | PASS | `services::download_runtime::registry` unit; `src-tauri/src/lib.rs` | none |
| VID-AC-18 | existing page/component/a11y tests and video option UI | PASS | frontend full suite; DownloadPage/DownloadOptions tests | none |
| VID-AC-19 | localized stable error mapping and no raw UI fields review | PASS | frontend error contract tests/locales; backend AppError boundaries | none |
| VID-AC-20 | serde/TS contract and nullability checks | PASS | `tests/parse_contract.rs`; frontend typecheck | none |
| VID-AC-21 | fake video-only/video-audio happy paths | PASS | `tests/video_executor.rs` | pause/fail matrix remains follow-up |
| VID-AC-22 | full frontend and Rust gates | PASS | evidence/commands.md | none |
| VID-AC-23 | trusted FFmpeg inventory | DEFERRED | no trusted bundled `ffmpeg.exe` in current workspace | system-release must run real MP4 smoke before packaging |
| VID-AC-24 | clean-code/ownership/pattern review | PASS | `review/review.md` | none |

## Spec Constraint Compliance

- result: **PASS with one declared external defer**.
- checked constraints: MP4-only video guard; exact quality+codec pairing; fresh auth/view/playurl; no URL/credential persistence; per-track workspace/checkpoint; budget 1 sequential and >=2 parallel; video-only no FFmpeg; video-audio Processing plus stream-copy mux; stable E008/E009; registry routing; no new command/event/capability.
- evidence: implementation files and command transcript in `verification/evidence/commands.md`.
- exception: VID-AC-23 is explicitly deferred because the approved environment has no trusted sidecar; this is not treated as a product capability pass.

## Spec-Plan Alignment

- result: **PASS**.
- VID-01..VID-08 remain at the approved function-complete granularity; VID-04/05/06/07 are closed only after focused tests and full gates. VID-08 remains the current verification/review unit.

## TDD Exceptions

- No implementation behavior was accepted without a red-to-green focused test or an existing regression test.
- VID-AC-23 has no fake substitute for real binary compatibility; it is recorded as an explicit external-resource skip and handed to system-release.

## Summary

- verification decision: **ready for review**.
- blockers: none in current workspace.
- external handoff: provide trusted bundled FFmpeg and run real stream-copy MP4 smoke in system-release.
