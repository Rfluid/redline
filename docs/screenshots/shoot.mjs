// Regenerates the README screenshots in this folder: a sample report an agent built, reviewed
// through redline in headless Chrome against a sandboxed daemon (never your real state).
// Run: `just screenshots` (builds the release binary first). Needs Chrome (or CHROME=/path).

import { writeFileSync } from "node:fs";
import { join } from "node:path";
import { Agent, Browser, repo, serve } from "../../tests/browser/harness.mjs";

const OUT = join(repo, "docs/screenshots");
const SESSION = "ses_readme";
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

// The artifact: what a coding agent might build when asked for a Q3 summary.
const page = (embed, dark) => {
  const c = dark
    ? { bg: "#16171b", fg: "#e8e6e1", muted: "#9a978f", card: "#1f2026", line: "#2c2d34", bar: "#6b8afd", bar2: "#3a3f55" }
    : { bg: "#fbfaf7", fg: "#1d1c1a", muted: "#6d6a63", card: "#ffffff", line: "#e7e4dc", bar: "#3557e0", bar2: "#c9d2f6" };
  const q = [["Q4 '25", 3.1], ["Q1 '26", 3.4], ["Q2 '26", 3.9], ["Q3 '26", 4.6]];
  const bars = q.map(([l, v], i) => {
    const h = v * 34, x = 40 + i * 120;
    return `<rect x="${x}" y="${190 - h}" width="64" height="${h}" rx="5" fill="${i === 3 ? c.bar : c.bar2}"/>
      <text x="${x + 32}" y="${182 - h}" text-anchor="middle" font-size="13" font-weight="600" fill="${c.fg}">$${v}M</text>
      <text x="${x + 32}" y="212" text-anchor="middle" font-size="12" fill="${c.muted}">${l}</text>`;
  }).join("");
  return `<!doctype html><html><head><meta charset="utf-8"><meta name="viewport" content="width=device-width">
<title>Q3 summary</title><style>
  html{scrollbar-width:none}
  body{margin:0;background:${c.bg};color:${c.fg};font:16px/1.6 Inter,ui-sans-serif,system-ui,-apple-system,"Segoe UI",sans-serif}
  main{max-width:680px;margin:0 auto;padding:48px 24px}
  .eyebrow{font-size:12px;letter-spacing:.08em;text-transform:uppercase;color:${c.muted};margin:0}
  h1{font-size:34px;line-height:1.2;margin:6px 0 8px;letter-spacing:-.02em}
  .lede{color:${c.muted};margin:0 0 28px}
  .kpis{display:grid;grid-template-columns:repeat(3,1fr);gap:12px;margin-bottom:28px}
  .kpi{background:${c.card};border:1px solid ${c.line};border-radius:12px;padding:14px 16px}
  .kpi b{display:block;font-size:24px;letter-spacing:-.01em}.kpi span{font-size:13px;color:${c.muted}}
  figure{margin:0 0 28px;background:${c.card};border:1px solid ${c.line};border-radius:12px;padding:16px 16px 8px}
  figcaption{font-size:13px;color:${c.muted};margin-bottom:4px}
  h2{font-size:19px;margin:28px 0 6px}
</style></head><body><main>
  <p class="eyebrow">Finance · Quarterly review</p>
  <h1 id="title">Q3 2026 summary</h1>
  <p class="lede" id="lede">Revenue grew 18% year over year, led by the enterprise tier.</p>
  <div class="kpis">
    <div class="kpi"><b>$4.6M</b><span>Revenue</span></div>
    <div class="kpi"><b>1,284</b><span>Paying teams</span></div>
    <div class="kpi"><b>2.1%</b><span>Monthly churn</span></div>
  </div>
  <figure id="chart"><figcaption>Revenue by quarter</figcaption>
    <svg viewBox="0 0 520 224" width="100%" role="img" aria-label="Revenue by quarter">${bars}</svg></figure>
  <h2>What drove it</h2>
  <p id="p1">Enterprise deals closed faster after the security review shipped in July, and expansion revenue doubled.</p>
  <p id="p2">Self-serve signups were flat. Churn ticked down for the third quarter in a row.</p>
  <h2>Next quarter</h2>
  <p id="p3">We plan to raise prices on the team plan in November and launch the EU data region.</p>
</main>${embed}</body></html>`;
};

const HELPERS = `
  window.$rl = (s) => document.querySelector('[data-redline]').shadowRoot.querySelector(s);
  window.select = (id, text) => {
    const node = document.getElementById(id).firstChild, at = node.data.indexOf(text);
    const r = document.createRange(); r.setStart(node, at); r.setEnd(node, at + text.length);
    getSelection().removeAllRanges(); getSelection().addRange(r);
    document.dispatchEvent(new MouseEvent('mouseup', { bubbles: true }));
  };
  window.compose = async (id, text, body) => {
    select(id, text);
    for (let i = 0; i < 40 && !$rl('.chip').classList.contains('show'); i++) await new Promise((r) => setTimeout(r, 25));
    $rl('.chip').click();
    $rl('.pop textarea').value = body;
  };
  window.comment = async (id, text, body, action) => { await compose(id, text, body); $rl('.pop [data-a="' + action + '"]').click(); };
  true`;

async function shoot(b, name, clip) {
  await sleep(400); // let transitions and the marker comet settle
  const { data } = await b.send("Page.captureScreenshot", { format: "png", ...(clip && { clip: { ...clip, scale: 1 } }) });
  writeFileSync(join(OUT, name), Buffer.from(data, "base64"));
  console.log(`wrote docs/screenshots/${name}`);
}

async function review(dark) {
  const agent = new Agent();
  const reg = agent.json("register", "Q3 summary", "--session", SESSION);
  const site = await serve({ "/": page(reg.embed, dark) });
  const b = await Browser.launch({ dark });
  const theme = dark ? "dark" : "light";
  try {
    await b.send("Emulation.setDeviceMetricsOverride", { width: 1280, height: 860, deviceScaleFactor: 2, mobile: false });
    await b.goto(site.url + "/");
    await b.eval(HELPERS);
    await b.waitFor("$rl('.conn').classList.contains('live')");

    // You comment; the agent answers one, is working on another, hasn't seen the third yet.
    const next = async (id, text, body) => {
      const waiting = agent.wait(SESSION);
      await b.eval(`comment(${JSON.stringify(id)}, ${JSON.stringify(text)}, ${JSON.stringify(body)}, 'send')`);
      return (await waiting).events[0].thread.id;
    };
    const t1 = await next("lede", "grew 18% year over year", "Compare with Q2 as well, not only last year.");
    agent.run("reply", t1, "Done. The line now reads **+18% YoY, +6% vs Q2**, and the chart highlights Q2 too.");
    const t2 = await next("p1", "expansion revenue doubled", "Add the actual numbers here.");
    agent.run("status", t2, "working", "--note", "Pulling the numbers");
    await next("p3", "raise prices on the team plan", "Not decided yet. Say \"we're evaluating\".");
    await b.waitFor(`$rl('.mk.answered[data-id="${t1}"]') !== null`);

    // 1. Hero: the answered thread open on the page.
    await b.eval(`$rl('.mk[data-id="${t1}"]').click()`);
    await b.waitFor("$rl('.pop.show') !== null");
    await shoot(b, `review-${theme}.png`);
    await b.eval("$rl('.pop [data-a=\"close\"]').click()");

    if (dark) return;

    // 2. Composing a comment on a selection.
    await b.eval("compose('p2', 'Self-serve signups were flat', 'Why flat? Show the weekly trend under this.')");
    await shoot(b, "compose.png", { x: 160, y: 330, width: 1120, height: 530 });
    await b.send("Input.dispatchKeyEvent", { type: "keyDown", key: "Escape", code: "Escape", windowsVirtualKeyCode: 27 });
    await b.waitFor("!$rl('.pop.show')");

    // 3. The dock's thread list.
    await b.eval("getSelection().removeAllRanges(); $rl('[data-a=\"drawer\"]').click()");
    await b.waitFor("$rl('.drawer.show') !== null");
    await shoot(b, "threads.png", { x: 920, y: 0, width: 360, height: 196 });
    await b.eval("$rl('[data-a=\"drawer\"]').click()");

    // 4. On a phone, threads open as a sheet.
    await b.send("Emulation.setDeviceMetricsOverride", { width: 390, height: 844, deviceScaleFactor: 3, mobile: true });
    await b.goto(site.url + "/");
    await b.eval(HELPERS);
    await b.waitFor(`$rl('.mk[data-id="${t1}"]') !== null`);
    await b.eval(`$rl('.mk[data-id="${t1}"]').click()`);
    await b.waitFor("$rl('.pop.show') !== null");
    await shoot(b, "phone.png");
  } finally {
    await b.close().catch(() => {}); // Chrome may still be writing its profile
    site.close();
    agent.stop();
  }
}

await review(false);
await review(true);
