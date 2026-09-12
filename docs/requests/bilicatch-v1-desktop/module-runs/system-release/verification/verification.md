# System Release Verification

## Delivery Unit

- request: `bilicatch-v1-desktop`
- module: `system-release`
- implementation scope: SR-01..SR-04 local core plus Windows x64 FFmpeg sidecar inventory

## Acceptance Coverage

| Acceptance | Result | Evidence | Follow-up |
| --- | --- | --- | --- |
| SR-AC-01 | PASS | Tauri tray/lifecycle composition compiles; startup composition unchanged | OS-level tray smoke on target runner |
| SR-AC-02 | PASS | `WindowDecision` contract and close-event integration | Native confirmation UX smoke |
| SR-AC-03 | PASS | stable notification mapping/privacy contract | Optional OS notification plugin decision |
| SR-AC-04 | PASS | settings-driven close/window policy boundary | target-platform bounds smoke |
| SR-AC-05 | PASS | typed updater state machine contract tests | real updater endpoint smoke |
| SR-AC-06 | PASS | signature-confirmation gate in `UpdateMachine` | real signed artifact install smoke |
| SR-AC-07 | PASS | existing capability remains minimal; no wildcard shell/WebView permission added | release capability audit |
| SR-AC-08 | PASS | `SidecarManifest`/`ReleaseArtifact` E008 validation and consistency tests | provide trusted sidecars |
| SR-AC-09 | DEFERRED | bundler targets remain existing `all` configuration; no signed platform artifacts available | SR-05 with platform runners and signing credentials |
| SR-AC-10 | DEFERRED | Windows x64 FFmpeg 9.0.1 sidecar is inventoried and packaged; real MP4/AAC smoke is still pending | SR-04/06 release smoke |
| SR-AC-11 | PASS | full Rust/frontend gates recorded below | repeat after release inputs are supplied |

## Commands

- `cargo fmt --all -- --check`: PASS
- `cargo check --all-targets`: PASS
- `cargo test --all-targets`: 73 passed, 1 explicit public-endpoint ignore
- `npm run typecheck`: PASS
- `npm test -- --run`: 46 files, 176 tests passed
- `npm run build`: PASS; existing >500 kB chunk warning only

## Spec Constraint Compliance

- result: **PASS for local scope; release artifacts explicitly deferred**.
- No new route, business task state, shell command, raw update response, secret, cookie, URL, or stderr persistence was introduced.
- Missing non-Windows sidecars, signing material and platform runners remain external release gates, not local test failures.

## Decision

- verification status: **blocked for final release acceptance by external inputs**.
- local implementation may proceed to review only after SR-04/05 inputs are supplied or the product owner explicitly accepts a deferred release artifact.
