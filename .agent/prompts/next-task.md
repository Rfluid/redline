# Prompt: continue the roadmap

Paste into a fresh agent session at the repo root:

```
You are developing redline in this repo. Read .agent/README.md and follow it: read .agent/context.md and .agent/rules.md and the newest entry in .agent/log/, then run `just check`.
Take the next unchecked task in the current phase of docs/ROADMAP.md and complete it using .agent/workflows/implement-task.md.
Stop after that one task: tick it in the roadmap, write a .agent/log/ entry, and report what changed, how you verified it, and what's next. Ask me before anything that edits my agent configs, commits, or changes the protocol or security model.
```

Variant, for a specific task: replace the second paragraph with `Implement roadmap task <ID> (docs/ROADMAP.md).`
