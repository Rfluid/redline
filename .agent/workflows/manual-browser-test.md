# Workflow: test the browser side by hand

Use this for changes to `web/client.js` or `web/overlay.js`, or to anything the page sees. First extend and run the automated test (`just browser`, `tests/browser/overlay.test.mjs`); do the manual pass for what it can't judge (layout, looks, real mouse selection).

```sh
export REDLINE_HOME="$(mktemp -d)" REDLINE_PORT=7499
cargo build --release
reg="$(target/release/redline register Demo --session dev)"
emb="$(printf '%s' "$reg" | jq -r .embed)"          # printf, not echo: zsh echo mangles \n
python3 - "$emb" "$REDLINE_HOME/demo.html" <<'PY'
import sys; s=open('examples/demo.html').read()
open(sys.argv[2],'w').write(s.replace(s[s.index('<!-- redline:embed'):s.index('-->',s.index('<!-- redline:embed'))+3], sys.argv[1]))
PY
(cd "$REDLINE_HOME" && python3 -m http.server 7498) &
```

1. **Open** `http://127.0.0.1:7498/demo.html`. If you have a browser tool (chrome-devtools MCP, Playwright), drive it; otherwise ask the user to.
2. **Check:**
   - select text → the **Comment** chip appears → Send; a numbered red marker appears in the margin
   - Save draft → a dashed marker, and **Send 1 draft** in the dock
   - the highlight shows on the commented text
   - the dock says **Live**
   - compare with `.design/` (components.md, layout.md) and its mockups: light page, a dark page (`document.body.style.background = '#111'`), and 390 px wide.
3. **Act as the agent:**
   ```sh
   target/release/redline wait --session dev --timeout 5
   target/release/redline reply <thread> "done"
   ```
   The marker must turn blue (answered) and pulse, the dock must show **1 new reply**, and the thread popover must show the reply live with an **Answered** chip.
4. **Clean up:** kill the daemon (pid in `$REDLINE_HOME/daemon.json`) and the http server.
