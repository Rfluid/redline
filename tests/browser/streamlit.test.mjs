// Browser test: the Streamlit binding's example app, end to end. Skips when Streamlit isn't installed.
// Run: `node tests/browser/streamlit.test.mjs` (after `cargo build --release`).

import assert from "node:assert/strict";
import { execFileSync, spawn } from "node:child_process";
import { join } from "node:path";
import { Agent, Browser, python, repo, steps } from "./harness.mjs";

try { execFileSync(python, ["-c", "import streamlit"], { stdio: "ignore" }); }
catch { console.log("skip - streamlit not installed"); process.exit(0); }

const SESSION = "ses_streamlit";
const agent = new Agent();
const reg = agent.json("register", "Pricing review", "--session", SESSION, "--kind", "streamlit");
const port = String(21000 + Math.floor(Math.random() * 9000));
const app = spawn(python, ["-m", "streamlit", "run", join(repo, "bindings/streamlit/example_app.py"),
  "--server.headless", "true", "--server.port", port, "--browser.gatherUsageStats", "false"], {
  env: { ...process.env, REDLINE_EMBED: reg.embed, PYTHONPATH: join(repo, "bindings/streamlit") },
  stdio: "ignore",
});
const b = await Browser.launch();
// Component iframes that rendered a button (same origin as the app, so we can reach into them), in page order.
const buttons = `[...document.querySelectorAll('iframe')].map((f) => f.contentDocument).filter((d) => d && d.querySelector('#root button'))`;
let thread;

await steps([
  ["example app renders one comment button per widget", async () => {
    for (let i = 0; i < 60; i++) {
      try { await b.goto(`http://127.0.0.1:${port}/`); break; } catch { await new Promise((r) => setTimeout(r, 500)); }
    }
    await b.waitFor(`${buttons}.length === 2`, 30000);
  }],

  ["comment on a widget reaches the agent with an app anchor", async () => {
    const waiting = agent.wait(SESSION, 30);
    await b.eval(`(() => { const d = ${buttons}[0]; d.querySelector('[data-a="toggle"]').click(); return true; })()`);
    await b.waitFor(`${buttons}[0].querySelector('textarea') !== null`);
    await b.eval(`(() => { const d = ${buttons}[0]; d.querySelector('textarea').value = 'show this as ARR'; d.querySelector('[data-a="send"]').click(); return true; })()`);
    const { events } = await waiting;
    assert.equal(events.length, 1);
    assert.deepEqual(events[0].thread.anchor, { kind: "app", key: "mrr", label: "MRR metric", page: "/" });
    thread = events[0].thread.id;
  }],

  ["agent reply appears in the widget's component, live", async () => {
    agent.run("reply", thread, "Switched to ARR.");
    await b.waitFor(`${buttons}[0].body.textContent.includes('Switched to ARR.')`, 15000);
    await b.waitFor(`${buttons}[0].querySelector('[data-a="toggle"]').textContent.toLowerCase().includes('answered')`);
  }],

  ["drafts from two widgets go out together with Send drafts", async () => {
    const draft = (i, text) => b.eval(`(() => { const d = ${buttons}[${i}];
      if (!d.querySelector('textarea')) d.querySelector('[data-a="toggle"]').click();
      return true; })()`).then(() => b.waitFor(`${buttons}[${i}].querySelector('textarea') !== null`))
      .then(() => b.eval(`(() => { const d = ${buttons}[${i}]; d.querySelector('textarea').value = ${JSON.stringify(text)};
        d.querySelector('[data-a="draft"]').click(); return true; })()`));
    await draft(0, "draft on MRR");
    await draft(1, "draft on plans");
    await b.waitFor(`${buttons}[1].querySelector('[data-a="send-drafts"]')?.textContent === 'Send 2 drafts'`);
    await b.waitFor(`${buttons}[0].querySelector('[data-a="toggle"]').textContent.includes('2 drafts')`, 5000);
    const waiting = agent.wait(SESSION, 30);
    await b.eval(`${buttons}[1].querySelector('[data-a="send-drafts"]').click()`);
    const { events } = await waiting;
    assert.deepEqual(events.map((e) => e.thread.anchor.key).sort(), ["mrr", "pricing_table"]);
  }],

  ["no console errors", () => assert.deepEqual(b.errors.filter((e) => !/streamlit|websocket/i.test(e)), [])],
], async () => {
  await b.close();
  app.kill();
  agent.stop();
});
