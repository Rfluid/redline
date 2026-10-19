# Prompt: debug a problem

```
Something is wrong in redline: <symptom, exact error, steps>.
Work in an isolated sandbox (REDLINE_HOME=$(mktemp -d), REDLINE_PORT=7499); never touch ~/.local/state/redline.
Reproduce it first with the smallest command sequence or test. Find the root cause (daemon.log in REDLINE_HOME helps), fix it, and add a regression test (unit or tests/e2e.sh). Run `just check`. Report the cause, the fix, and the test.
```
