# Bugfix source

## Source system

Direct user report in the active Codex thread.

## Project key or source scope

BiliCatch desktop application.

## Defect or work item id

`bilicatch-regression-fixes-20260913`

## Defect title

Authentication refresh, navigation, reparse, task layout, and multi-mode download regressions.

## Observed behavior

1. Qualities marked as login-required remain stale after login.
2. Task action buttons overflow their action-column cell.
3. Opening account information through the logged-in username automatically returns to the previous page.
4. Clicking Parse again does not refresh the displayed video information.
5. Only video-only downloads work; video+audio and audio-only report that video is unavailable.

## Expected behavior

1. A successful login triggers an immediate fresh capability parse and updates quality labels and disabled states.
2. Every task-table value and control stays within its own grid cell at supported viewport widths.
3. An already-authenticated user who intentionally opens account information stays on that page until navigating away.
4. Every explicit Parse action fetches and reapplies fresh page information, even for the same canonical input.
5. Video-only, video+audio, and audio-only use the appropriate source pipeline and complete when Bilibili exposes the required streams.

## Reproduction clues

- Parse a video while anonymous, choose an available quality, then log in and return.
- Inspect task rows containing multiple operation icons at desktop and compact widths.
- Click the authenticated username in the top bar and wait longer than 800 ms.
- Click Parse repeatedly with the same complete Bilibili URL.
- Queue each download mode for the same parsed video.

## Affected module or page

`/download`, `/tasks`, `/login`, authentication events/store, parser cache, media-source selection, audio and video executors.

## Available screenshots, logs, or comments

No screenshots supplied. Existing task records showed E004/source-unavailable failures. Live API inspection showed anonymous audio stream bandwidth values around 66-85 kbps, while the source selector rejected all anonymous streams above a strict 64 kbps threshold.

## Open questions and missing context

None blocking. Explicit manual Parse is defined here as cache-bypassing; background auth refresh also requests fresh capabilities.
