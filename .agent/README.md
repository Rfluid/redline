# .agent: workspace for coding agents developing redline

This folder is the **agent-agnostic source of truth** for how to work on this repo, whichever agent you are (Claude Code, Codex, Cursor, Gemini, opencode, Antigravity…). `CLAUDE.md`, `AGENTS.md` and `AGENT.md` at the repo root only point here.

> Not to be confused with `SKILL.md` / `INSTALL.md` at the repo root. Those are for agents *using* redline. This folder is for agents *building* it.

## Start here

1. Read [`context.md`](context.md): what redline is, plus its invariants. Five minutes.
2. Read [`rules.md`](rules.md): hard rules. Non-negotiable.
3. Pick work from [`../docs/ROADMAP.md`](../docs/ROADMAP.md), and follow [`workflows/implement-task.md`](workflows/implement-task.md).
4. Before you stop, add an entry to [`log/`](log/README.md).

## Layout

| Path | What | When to use |
|---|---|---|
| `context.md` | Project brief, architecture in one screen, invariants | Every session, first |
| `rules.md` | Hard rules: safety, quality gates, docs sync | Every session |
| `workflows/` | Step-by-step procedures for recurring tasks | When the task matches one |
| `prompts/` | Ready-to-paste prompts the user gives agents | When starting or delegating work |
| `skills/` | Agent skills for developing redline (portable SKILL.md format) | Load if your agent supports skills; otherwise read them as docs |
| `log/` | Session handoff notes, newest first | Read the latest at start; write one at the end |
| `../.design/` | Design system: principles, tokens, components, logo | Before any change to what redline draws (overlay, bindings' UI, logo) |

## Workflows

| Workflow | Use for |
|---|---|
| [implement-task](workflows/implement-task.md) | Any roadmap task, end to end |
| [add-mcp-tool](workflows/add-mcp-tool.md) | A new MCP tool and its CLI twin |
| [add-agent-support](workflows/add-agent-support.md) | Supporting another coding agent in `install` |
| [add-migration](workflows/add-migration.md) | A database schema change |
| [manual-browser-test](workflows/manual-browser-test.md) | Checking the overlay or `client.js` in a real browser |
| [self-review](workflows/self-review.md) | Before calling any change done |

## Prompts

| Prompt | Use for |
|---|---|
| [next-task](prompts/next-task.md) | The next roadmap task |
| [test-clean-install](prompts/test-clean-install.md) | Verifying the user's clean install end to end (after they install with the README sentence). Pending: ROADMAP P1.1 |
| [review](prompts/review.md) | Reviewing a change |
| [debug](prompts/debug.md) | Debugging a problem |

## Project docs (human + agent)

| Doc | |
|---|---|
| [`docs/ARCHITECTURE.md`](../docs/ARCHITECTURE.md) | Components, data model, delivery, security |
| [`docs/PROTOCOL.md`](../docs/PROTOCOL.md) | HTTP / WS / client.js / MCP / CLI contracts |
| [`docs/CODE_STYLE.md`](../docs/CODE_STYLE.md) | Code patterns and conventions |
| [`docs/DEVELOPMENT.md`](../docs/DEVELOPMENT.md) | Build, test, tooling, layout |
| [`docs/DECISIONS.md`](../docs/DECISIONS.md) | Why things are the way they are |
| [`docs/ROADMAP.md`](../docs/ROADMAP.md) | Implementation roadmap: phases, tasks, acceptance criteria |

## Keeping this folder useful

- **Change in how we work** (a new recurring task, a gotcha that cost you time): update or add a workflow here in the same change.
- **Keep files short and imperative.** Agents read them literally.
- **Put agent-specific config in that agent's own files** (`.claude/settings.json`, `.cursor/`, …). Shared knowledge belongs here.
