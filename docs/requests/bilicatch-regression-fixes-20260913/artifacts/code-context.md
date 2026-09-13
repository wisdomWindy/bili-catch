# Code context

## Context requirement

Cross-module behavior spans Vue route/store watchers, Tauri parser caches, task CSS, and Rust media executors. Structural analysis was required during bugfix intake; code graph was preferred.

## Graph availability check

- Repository graph status: missing
- Detection method: available tool/skill inventory and repository inspection
- Tool or runtime found: none
- Check context: intake iteration 1, 2026-09-13

## Installation or bootstrap record

- Attempted: no
- Method: no repository-preferred graph bootstrap was present
- Result: not_needed
- Output summary: continued with bounded repository-local symbol search
- Next step: verify affected call paths with focused tests

## Fallback record

- Fallback used: yes
- Method: `rg` symbol/call-site search plus focused file reads
- Reason: no usable code graph capability was available
- Residual confidence: high for the scoped entrypoints; runtime behavior still requires packaged smoke tests

## Relevant entrypoints

- `DownloadPage.vue` auth watcher and submit action
- `LoginPage.vue` authentication watcher
- `TaskRow.vue` and `task-management.css`
- Rust `ParserService::parse` and source resolver implementations
- `AudioExecutor` and `VideoExecutor`

## Key symbols and modules

`refreshForAuthChange`, `parse`, `resultCache`, `select_audio_source`, `VideoSourcePort::resolve`, `TaskRuntimeRegistry`, `task-row__actions`.

## Dependency and side-effect boundaries

Vue pages orchestrate Pinia stores and router state. IPC services own Tauri calls. Rust parser owns Bilibili HTTP/cache behavior. Executors own disk/process side effects. Task CSS owns grid containment.

## Impact scope

Focused changes are expected in download-center store/page tests, login-page watcher/tests, task-management CSS/component tests, Rust audio-source selection/parser tests, and executor integration tests.

## Open follow-up checks

Completed in execution iteration 5-6: live parsing passed for `BV1FNb366EH2`, and the Windows NSIS installer plus updater signature were generated.

## Execution discoveries

- The frontend and Rust parser business-result caches had the same stale-result behavior and were both removed; WBI key caching remains isolated at protocol level.
- `ValidatedAuthContext.revision` was only consumed by the removed parser cache. The internal field was removed without changing public auth snapshot/event revisions.
- Ordinary audio selection is owned solely by `select_audio_source`; the authentication bandwidth cutoff was the direct cause of E004 for 65,971/85,411 bps Bilibili candidates.
- Video and audio workspaces already enforce non-empty finalization; executors now also verify the final path immediately before reporting completed.
