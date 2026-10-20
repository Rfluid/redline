# 2026-10-20 · drawer-bulk-actions

**Agent:** claude  **Roadmap:** none (user request)

## Done
- Drawer gets a Select mode (header toggle; off by default after user feedback that always-on checkboxes were noisy). In it, rows get checkboxes (Shift+click for a range, select-all with an indeterminate state) and a selection bar: Resolve / Reopen / Send / Discard, each acting only on the rows it fits, with a count when it covers part of the selection. Resolve and Discard toast with Undo; Esc leaves the mode before it closes the drawer; a row click toggles it instead of opening the thread.
- New `web/bulk.js` (overlay.js was at the size limit); selection state `S.picked`, pruned to the rows on screen (`S.shown`) in `overlay-view.js`. `dock.renderDrawer` keeps focus and scroll across re-renders.
- Drawer header fits 360 px: "Threads" is screen-reader only, the drafts button reads "Send K".
- No protocol change: bulk resolve is one `POST /status` per thread (one event each).
- Docs: `.design/components.md` (Drawer), `.design/accessibility.md`, `docs/CODE_STYLE.md`, README, CHANGELOG.

## Verified
- New browser test step in `tests/browser/overlay.test.mjs`; manual pass in Chrome (light, 360 px drawer).

## Open / next
- The daemon serves `/client.js` and `/overlay.js` with no `Cache-Control`; after an update a browser can keep the old overlay until a hard reload. Fix: `Cache-Control: no-cache` on both.
- `tests/browser/react.test.mjs` timed out locally (esm.sh's `scheduler` module stalls, page stays `interactive`); same on a clean checkout, so not this change.
- Undo after a bulk (or single) resolve reopens as `open`, losing `answered`/`working`. Pre-existing for single resolve too.
- Gotcha: never `pkill -f "redline serve"`; it also kills the user's real daemon (and the shell running it). Kill the sandbox pid from `$REDLINE_HOME/daemon.json`.
