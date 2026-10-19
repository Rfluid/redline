// Browser test: bindings/react/example.html (own comment UI via useRedline). Needs network for the
// React CDN; skips if esm.sh is unreachable. Run: `node tests/browser/react.test.mjs`.

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { Agent, Browser, repo, serve, steps } from "./harness.mjs";

const CDN = "https://esm.sh/react@18.3.1";
try { await fetch(CDN, { signal: AbortSignal.timeout(5000) }); }
catch { console.log("skip - React CDN unreachable"); process.exit(0); }

const SESSION = "ses_react";
const agent = new Agent();
const reg = agent.json("register", "React example", "--session", SESSION, "--kind", "react");
const clientTag = reg.embed.split("\n").find((l) => l.includes("/client.js")); // client.js only: own UI
const dir = join(repo, "bindings/react");
const site = await serve({
  "/": readFileSync(join(dir, "example.html"), "utf8").replace("<!-- redline:embed (client.js tag only) -->", clientTag),
  "/useRedline.js": readFileSync(join(dir, "useRedline.js"), "utf8"),
  "/redline-store.js": readFileSync(join(dir, "redline-store.js"), "utf8"),
});
const b = await Browser.launch();
const card = (key) => `document.querySelector('.card[data-key="${key}"]')`;
const type = (key, text) => `(() => { const ta = ${card(key)}.querySelector('textarea');
  Object.getOwnPropertyDescriptor(HTMLTextAreaElement.prototype, 'value').set.call(ta, ${JSON.stringify(text)});
  ta.dispatchEvent(new Event('input', { bubbles: true })); return true; })()`;
let thread;

await steps([
  ["example renders live", async () => {
    await b.goto(site.url + "/");
    await b.waitFor("document.querySelectorAll('.card').length === 3 && document.querySelector('header .live.on') !== null", 20000);
  }],

  ["own UI comment reaches the agent; reply shows in the card", async () => {
    const waiting = agent.wait(SESSION);
    await b.eval(type("churn", "is this monthly?"));
    await b.eval(`${card("churn")}.querySelector('[data-a="send"]').click()`);
    const { events } = await waiting;
    assert.deepEqual(events[0].thread.anchor, { kind: "app", key: "churn", label: "Churn" });
    thread = events[0].thread.id;
    agent.run("reply", thread, "Yes, monthly.");
    await b.waitFor(`${card("churn")}.textContent.includes('Yes, monthly.') && ${card("churn")}.textContent.includes('Answered')`);
  }],

  ["drafts → Send drafts", async () => {
    for (const k of ["revenue", "nps"]) {
      await b.eval(type(k, `draft on ${k}`));
      await b.eval(`${card(k)}.querySelector('[data-a="draft"]').click()`);
    }
    await b.waitFor("document.querySelector('[data-a=\"send-drafts\"]')?.textContent === 'Send 2 drafts'");
    const waiting = agent.wait(SESSION);
    await b.eval("document.querySelector('[data-a=\"send-drafts\"]').click()");
    assert.equal((await waiting).events.length, 2);
  }],

  ["no console errors", () => assert.deepEqual(b.errors, [])],
], async () => {
  await b.close();
  site.close();
  agent.stop();
});
