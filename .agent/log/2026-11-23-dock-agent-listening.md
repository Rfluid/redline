# 2026-11-23 · dock-agent-listening

**Agent:** claude  **Roadmap:** none (user request, from a real test: the dock said "Live" while no agent session was waiting)

## Done
- `src/daemon/presence.rs` (new): `Listeners` tracks admin long-polls on `/api/events` in flight by filter (moved from `App.waiting`, still feeds `redline status`) and when each filter's last one ended. `presence(artifact)` = a covering poll (artifact, its session, or no filter) in flight, or ended < `LISTEN_GRACE` (60 s) ago. A `watch` channel ticks on each start/end.
- `src/daemon/live.rs`: the artifact WebSocket sends `{"type":"presence","listening":bool}` on open and on change (rechecks on a tick, and when the grace window runs out). Never an event; no `seq`.
- `web/client.js`: routes the frame to `client.listening` + a `presence` event (bindings that only handle `event` are unaffected). `web/dock.js` / `overlay-view.js` / `overlay-style.js`: "No agent listening" (muted, hollow dot) when connected and the daemon says nobody listens, unless a thread is `working`. Unknown (no frame yet, older daemon) shows "Live" as before.
- Docs: PROTOCOL (WS frame, presence rule, client API), ARCHITECTURE (delivery), DECISIONS (new entry), `.design/components.md` (dock), README troubleshooting row, SKILL (keep a wait running), CHANGELOG Unreleased.

## Verified
- Unit tests: `daemon::presence::tests` (filters, grace, pruning) and `daemon::tests::a_session_long_poll_makes_its_artifacts_listened_to` (through the router; event log untouched).
- Browser test step: a fresh page shows "No agent listening", then "Live" once `redline wait` runs.
- `just check` passes.

## Open / next
- The React store (`bindings/react/redline-store.js`) and the Streamlit binding don't surface `listening` yet; one line each if wanted.
- A thread left `working` by an agent that died keeps the dock on "Live" (same as its spinner).
- `docs/screenshots/shoot.mjs` waits for `.conn.live`, which still matches the quiet state; the first screenshot may show "No agent listening" if regenerated before any wait.

## Gotchas
- Event frames have no `type` field; anything new on the WebSocket should carry one so clients can tell it apart.
