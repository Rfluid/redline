# Installing redline (instructions for an agent)

The user said something like "install redline from <repo> for all my agents", or "…for claude and codex". Follow these steps in order. Run the commands yourself; don't just print them.

**Assume nothing about this device.** Detect what you can and ask about the rest. Needs `git`. With a Rust toolchain (`cargo`), the first run builds from source. Without one, `bin/redline` downloads the release binary for Linux (x86_64/aarch64), macOS (arm64/x86_64) or Windows (x86_64/arm64); the repo is private for now, so that needs `gh` logged in (`gh auth status`). Offer to install Rust (https://rustup.rs) only if neither works.

**Windows:** everything below works the same, with the Windows twins of the scripts (Windows PowerShell runs them; it comes with Windows): `scripts\checkout-release.ps1` instead of `scripts/checkout-release.sh`, and `bin\redline.cmd` instead of `bin/redline`. Run them as shown in each step. From Git Bash, `bin/redline` works too (it hands over to `bin\redline.ps1`).

## 1. Get the repo

If you're not already inside a clone, ask the user where to keep it. It has to be a permanent location, because agents link to it and run its binary; never use a temp dir. Clone it there, then switch it to the latest release (main is where development happens; releases are what users run):

```sh
git clone https://github.com/Rfluid/redline <clone>
<clone>/scripts/checkout-release.sh    # checks out the highest vX.Y.Z tag; prereleases are skipped
```

On Windows: `powershell -NoProfile -ExecutionPolicy Bypass -File <clone>\scripts\checkout-release.ps1` (Bypass because Windows blocks local scripts by default; it applies to this one run).

Only if the user asks for a prerelease (e.g. `v0.3.0-beta`): `<clone>/scripts/checkout-release.sh v0.3.0-beta` (Windows: add `v0.3.0-beta` after the `.ps1`).

Below, `R=<clone>/bin/redline` (Windows: `R=<clone>\bin\redline.cmd`). The first run builds the release binary (about a minute) or downloads it.

## 2. Install for the agents

Run `$R install` (every agent detected on this device) or `$R install --agents claude,codex` (just those). Show the user the output: which agents were found, where the skill was linked, and where the MCP server was registered.

- Known agents: `claude`, `codex`, `opencode`, `cursor`, `agy`, `gemini`.
- For each agent, install does two things:
  - It symlinks this repo as the `redline` skill into that agent's skills dir (same as handoff). On Windows the link is a directory junction, which needs no admin rights.
  - It registers the MCP server `redline` → `<clone>/bin/redline mcp` (Windows: `<clone>\target\bin\redline.exe mcp`, the binary `bin\redline.ps1` keeps up to date).
    - Claude: `claude mcp add --scope user`.
    - Codex: `[mcp_servers.redline]` in `~/.codex/config.toml` (`$CODEX_HOME/config.toml` if set).
    - Cursor, Gemini, Antigravity: `mcpServers.redline` in their JSON config (Cursor: `~/.cursor/mcp.json`, with `"type": "stdio"`; Antigravity: `~/.gemini/config/mcp_config.json`, shared by the app, IDE and `agy` CLI).
  - Skills: Codex, opencode, Cursor and Gemini CLI share `~/.agents/skills`. Cursor also reads `~/.claude/skills`, so with Claude Code installed it lists redline twice; that's harmless.
  - `gemini` (Gemini CLI) and `agy` (Antigravity CLI) are separate agents with separate configs. Antigravity replaced Gemini CLI for unpaid users, but paid users still have it.
    - opencode: `mcp.redline` in `opencode.json`.
  - Configs are backed up once to `<file>.redline.bak` before the first edit (for Claude, `~/.claude.json` or `<config dir>/.claude.json`). Key order and comments are kept, and a trailing comment such as another tool's `# END … managed block` marker stays above the redline entry.
- **The user runs Claude with another config dir** (a second profile, e.g. `CLAUDE_CONFIG_DIR=~/.claude-work`): ask about this, then add `--claude-config-dir <dir>` (repeatable). `$CLAUDE_CONFIG_DIR` is picked up automatically if set.
- **Another skills dir:** add `--skill-dir <dir>`.
- **A target already exists and isn't redline:** it is skipped. Tell the user. Pass `--force` (backs it up, then replaces it) only if they agree.
- **A config isn't plain JSON** (comments, as in JSONC): that agent is reported as FAILED along with the exact entry to add. Add it by hand only if the user agrees.
- **CLI link:** `redline` is linked into `~/.local/bin`. If that dir isn't on PATH, ask where to put it (`--bin-dir`). On Windows that's `%USERPROFILE%\.local\bin`, which usually isn't on PATH: offer to add it (`[Environment]::SetEnvironmentVariable('Path', "$env:Path;$HOME\.local\bin", 'User')`, then a new terminal), or pick another dir. It gets two small files there: `redline.cmd` (cmd, PowerShell) and `redline` (Git Bash).
- **The user wants the skill but no MCP:** `--no-mcp`. Agents then use the CLI, which does the same thing.

## 3. Explain (short)

> redline lets you comment inline on things your agent builds. The agent puts one script tag in the page. You select text and comment, then send each comment or submit a batch. The agent gets the comments in its session, makes the changes, and replies under each comment. Everything stays on this machine (127.0.0.1).

## 4. Check

Run `redline doctor` and report the results. Every detected agent should show `skill ok` and `mcp ok`.

Then tell the user:
- Restart open agent sessions so they load the skill and the MCP server.
- Try it: ask the agent to "build a small HTML page and open it for redline review".
- **Claude Code only, optional (channels, research preview):** comments can be pushed into the session without a waiting tool call. Only offer this if the user asks; most setups can't use it yet.
  1. Requirements: a claude.ai login or Console API key (not Bedrock/Vertex). On Team and Enterprise plans an Owner must turn channels on (`channelsEnabled: true` in managed settings); otherwise Claude Code says "blocked by org policy" and drops the messages.
  2. Re-register the server with the flag: `claude mcp remove --scope user redline`, then `claude mcp add --scope user redline -- <clone>/bin/redline mcp --channel` (Windows: `<clone>\target\bin\redline.exe` in place of `<clone>/bin/redline`). It behaves the same in sessions without channels. (`--mcp-config` servers don't count as channels.)
  3. Start Claude with `claude --dangerously-load-development-channels server:redline` and accept the prompt. Interactive sessions only; `claude -p` ignores it.
  Details: https://code.claude.com/docs/en/channels-reference

## Update

```sh
redline update    # runs <clone>/scripts/checkout-release.sh (.ps1 on Windows), then rebuilds (or downloads) the new binary
```

The running daemon is replaced automatically, and nothing needs reinstalling; restart open agent sessions so their MCP server is the new one. It refuses to run over local edits in the clone. redline also checks for a release once a day and mentions it (see `SKILL.md` › Updates); `redline update skip` / `off` silence it.

## Uninstall

```sh
redline uninstall    # removes skill links, the CLI link, and the MCP entries it added
```

Only links (and, on Windows, CLI shims) that point into this repo are removed. A config whose only change was redline's entry is restored byte for byte from its `.redline.bak`; otherwise just the entry (and an `mcpServers` section it emptied) is removed. Comment history stays in `~/.local/state/redline/` (Windows: `%LOCALAPPDATA%\redline\`) until the user deletes it.
