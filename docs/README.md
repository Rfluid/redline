# redline developer docs

| Doc | What's in it |
|---|---|
| [ARCHITECTURE.md](ARCHITECTURE.md) | Components, data model, how comments reach agents, security model |
| [PROTOCOL.md](PROTOCOL.md) | HTTP + WebSocket API, anchors, `client.js` API, MCP tools, CLI |
| [CODE_STYLE.md](CODE_STYLE.md) | Code patterns, module layout, error/async/persistence rules, tooling and lints |
| [DEVELOPMENT.md](DEVELOPMENT.md) | Repo layout, build, `just` recipes, tests, database migrations |
| [DECISIONS.md](DECISIONS.md) | Dependency and design decisions, with the alternatives considered |
| [ROADMAP.md](ROADMAP.md) | Implementation roadmap: phases, tasks, acceptance criteria |

The user-facing README is [`../README.md`](../README.md). Agent-facing files:
- [`../SKILL.md`](../SKILL.md): the review-loop workflow agents follow.
- [`../INSTALL.md`](../INSTALL.md): the install steps an agent runs for the user.

The design system (principles, tokens, components, logo) is in [`../.design/`](../.design/README.md).

Coding agents working on this repo should start at [`../.agent/README.md`](../.agent/README.md).
