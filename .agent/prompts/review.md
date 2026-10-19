# Prompt: review a change

```
Review the current diff of redline (git diff, or branch <name> vs main). Use .agent/workflows/self-review.md as the checklist and docs/CODE_STYLE.md for conventions, and check the invariants in .agent/context.md.
Report findings as: file:line, severity (bug / risk / style), problem, fix. No praise, no restating the diff. Verify each finding against the code before reporting it. Run `just check` and include the result.
```
