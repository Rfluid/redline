# Context

**redline** lets a user comment inline on an artifact a coding agent built (an HTML page, a dashboard, an app). The comments reach that agent's session, and the agent changes the artifact and replies inline. It is installed for any coding agent the same way as the sibling `handoff` skill: "install <repo> for all my agents" → `INSTALL.md`.

## One-screen architecture

```
page + web/client.js (+ web/overlay.js) ──HTTP/WS──► redline serve (axum, 127.0.0.1:7465, SQLite/Diesel)
                                                         ▲ HTTP                    ▲ HTTP long-poll
                                              redline mcp (rmcp, stdio)    redline wait|reply (CLI)
                                                         ▲ MCP
                                                    agent session
```

There is one Rust binary, built as a library (`src/lib.rs`) with a thin `main.rs`. Modules:

| Module | Role |
|---|---|
| `cli` | clap commands |
| `daemon/` | axum: routes, auth, long-poll, WS |
| `store/` | Diesel: rows, schema, event log |
| `mcp/` | rmcp server and tools |
| `api` | blocking HTTP client |
| `lifecycle` | starts, replaces (after updates) and stops the daemon |
| `install/` | agents table, symlinks, MCP config editing |
| `strip` | removes embed snippets |
| `model` | wire types |
| `paths` | state dir |
| `ids` | ids, tokens, time |

## Invariants (break one only with a DECISIONS.md entry)

1. **Communication only.** redline never renders or serves the artifact. `overlay.js` never mutates the artifact's DOM: it uses a shadow root and CSS Custom Highlights.
2. **Loopback only.** The daemon binds `127.0.0.1`.
3. **Two token kinds.** The admin token speaks as `agent` everywhere. An artifact token speaks as `user` on that artifact only.
4. **Every write appends exactly one event**, in the same transaction. Consumers follow `seq` and never miss events.
5. **Drafts live only in the browser.** The daemon sees only submitted comments.
6. **Every MCP tool has a CLI twin.** Agents without MCP must be able to do everything.
7. **Install never clobbers user files:** it backs up once, skips foreign targets without `--force`, and refuses JSONC.
8. **Any agent:** nothing agent-specific in the core. Agent differences live in `install/agents.rs` and the optional Claude channel push.

## Status

See `docs/ROADMAP.md` (phase checklist) and the newest entry in `.agent/log/`.
