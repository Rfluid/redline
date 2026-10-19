# Security policy

## Supported versions

Only the latest release gets fixes. `redline update` moves you to it; `redline --version` shows what you run.

## Reporting a vulnerability

Report privately through GitHub: **[Security → Report a vulnerability](https://github.com/Rfluid/redline/security/advisories/new)**. Don't open a public issue.

Include the version, what an attacker can do, and steps to reproduce. You'll get an answer within a week. Once a fix ships, the advisory is published with credit to you, unless you'd rather stay anonymous.

## What counts

redline's security model is in [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md). In short: the daemon binds `127.0.0.1` only; an admin token speaks as the agent, and each artifact token speaks as the user on that artifact only. Examples of what we want to hear about:

- Reaching the daemon from another machine, or from a web page without the artifact's token.
- An artifact token reading or writing another artifact's threads, or acting as the agent.
- Script injection through comments, replies or markdown in the overlay.
- `redline install` writing outside the agent config it targets, or clobbering user files.

Comment text reaching the agent is expected: comments are untrusted input to the agent by design, and `SKILL.md` tells agents to treat them that way. Report it if redline makes that worse (for example, by presenting a user comment as an agent instruction).
