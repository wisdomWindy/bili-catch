# Normalized defect snapshot

## Source summary

Five packaged-application regressions reported by the user on 2026-09-13.

## Key business goals

- Keep authentication-dependent download capabilities accurate without restarting or manually clearing state.
- Preserve intentional navigation and predictable page refresh behavior.
- Keep the task work surface usable at supported widths.
- Make every advertised download mode operational.

## Explicit behavior constraints

- Login transition refreshes media capabilities using the current authentication revision.
- Visiting account details while already authenticated must not schedule an automatic return.
- Explicit Parse bypasses successful-result caches and resets/reapplies all result-derived selections.
- Task action controls are clipped/wrapped/reflowed within their assigned cell, never over adjacent columns.
- Audio source selection accounts for encoded-stream overhead and does not reject valid nominal 64 kbps Bilibili streams solely because reported bandwidth is slightly above 64,000 bps.
- Video+audio requires one matching video source plus one ordinary audio source; audio-only must use the audio executor and never require a video source.

## Forms, tables, displays, and interactions extracted from source

- Download form: URL/ID input and Parse button; each click refreshes current result.
- Download options: login-required labels and disabled states update after auth changes.
- Task table: operation column owns all row action buttons and menus.
- Login page: account summary remains visible after a top-bar username click.

## Workflow and state rules extracted from source

- Anonymous parse -> login succeeds -> auth event -> fresh parse -> capability UI reconciled.
- Login-page auto-return occurs only for a login completed during the current login flow, not for an authenticated snapshot present on entry.
- Manual parse starts a new request token and ignores stale responses.
- Each mode selects only its required media sources and reports accurate E004/E005/E006 failures.

## Relevant modules or pages

Authentication, download center, task management, audio download, video download, and Windows packaging.

## Notable open questions from the upstream report

None.
