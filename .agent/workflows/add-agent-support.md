# Workflow: support another coding agent

1. **Find the agent's current docs for:**
   - the user-level skills dir
   - how to detect it (binary name, config dir)
   - its MCP config: file, format, and the key the servers live under.
   Don't guess. Record the source URL in your log entry.
2. **Add an `Agent`** to `AGENTS` in `src/install/agents.rs`.
   - Reuse an existing `McpConfig` variant if the format matches.
   - Otherwise add a variant, and handle it in `register` / `unregister` / `present` in `src/install/mcp_config.rs`. It must back up once, never clobber, and refuse unparseable configs.
3. **Unit-test** any new config editor against a sample file in a `tempfile::tempdir()`, including "other servers survive" and "no-op when already present".
4. **Update** the known-agents lists in `INSTALL.md`, `README.md`, `.agent/context.md` and `src/cli.rs` (the `--agents` help).
5. **Optional:** with the user's OK, run `redline install --agents <new>` and then `redline doctor`.
