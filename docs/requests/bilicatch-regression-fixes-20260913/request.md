# Request: BiliCatch regression fixes 2026-09-13

## Request identifier

`bilicatch-regression-fixes-20260913`

## Source link

Local user defect report in the active Codex thread on 2026-09-13.

## Business summary

Fix five regressions across authentication, download-center refresh, task-table layout, and audio/video download execution.

## Goal statement

Login-dependent capabilities update immediately, account details remain open when intentionally visited, every manual parse refreshes displayed data, task actions stay inside their grid cell, and all three download modes can complete when their required sources exist.

## Initial done signal

Focused regression tests pass for all five cases, the full frontend and Rust suites pass, and the Windows package builds successfully.

## Trigger condition

The user reported the five observable regressions after running the packaged desktop application.

## Initial context sources

- User defect report
- Existing approved module specifications under `docs/requests/bilicatch-v1-desktop/module-runs/`
- Current Vue/Tauri implementation and tests

## Human handoff point

Specification and plan approval before product-code changes, followed by packaged-build verification results.

## Affected area

Authentication page routing, download-center store and parsing cache, task-row layout, Bilibili audio source selection, and video/audio executors.

## Participating modules

`authentication`, `parse-download-center`, `task-management`, `audio-download`, `video-download`, `system-release`.
