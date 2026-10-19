# Workflow: add an MCP tool

Invariant: every MCP tool has a CLI twin.

1. **Daemon route** (if new data or behaviour):
   - Add a handler in `src/daemon/handlers.rs` (or `live.rs` for streaming).
   - Register it in `router()` in `src/daemon/mod.rs`.
   - Auth: `require_admin` for agent-only routes, `author_for` / `thread_author` when artifacts may call it.
2. **Store** (if needed): a method on `Store`. Any write runs in one transaction and appends one event (see CODE_STYLE).
3. **Client:** a typed method on `Api` in `src/api.rs`.
4. **Tool:** in `src/mcp/tools.rs`, add:
   - an args struct (`Deserialize + schemars::JsonSchema`; field doc comments become the schema descriptions)
   - a `#[tool(description = "...")]` method that returns `ToolResult` and goes through `blocking(|api| …)`.
   Descriptions are imperative and say when to use the tool.
5. **CLI twin:** a `Cmd` variant plus a dispatch arm in `src/cli.rs`.
6. **Tests:**
   - a unit test for any new store logic
   - an e2e step calling the tool over stdio (copy the MCP block in `tests/e2e.sh`)
   - the CLI twin.
7. **Docs:** add the tool to `docs/PROTOCOL.md` (MCP tools + CLI tables) and to the `SKILL.md` tool table. Also update the review loop if agents should use the tool there.
