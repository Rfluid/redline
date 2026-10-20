// Browser test: overlay.js + client.js in headless Chrome against a sandboxed daemon.
// Run: `just browser` (builds the release binary first). Needs Chrome (or CHROME=/path).

import assert from "node:assert/strict";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { Agent, Browser, serve, steps } from "./harness.mjs";

const SESSION = "ses_browser";
const agent = new Agent();
const reg = agent.json("register", "Browser test", "--session", SESSION);
const page = (embed) => `<!doctype html><html><head><meta charset="utf-8"><meta name="viewport" content="width=device-width"><title>t</title></head><body>
  <main style="max-width:640px;margin:40px auto">
  <h1 id="title">Quarterly plan</h1>
  <p id="p1">We will ship the importer in March, then harden the sync engine through April.</p>
  <p id="p2">Pricing stays flat until the enterprise tier lands.</p>
  <canvas id="chart" width="400" height="200" style="width:400px;height:200px"></canvas>
  </main>
  <script>const g = document.getElementById("chart").getContext("2d"); g.fillStyle = "#1565c0"; g.fillRect(0, 0, 400, 200);</script>
  ${embed}
</body></html>`;
const site = await serve({ "/": page(reg.embed) });
const b = await Browser.launch();

// In-page helpers, injected once per navigation.
const HELPERS = `
  window.$rl = (s) => document.querySelector('[data-redline]').shadowRoot.querySelector(s);
  window.$$rl = (s) => [...document.querySelector('[data-redline]').shadowRoot.querySelectorAll(s)];
  window.hl = (n) => CSS.highlights.get(n)?.size || 0;
  window.hlAll = () => ['redline-open', 'redline-answered', 'redline-draft', 'redline-focus', 'redline-focus-agent'].reduce((s, n) => s + hl(n), 0);
  window.select = (id, text) => {
    const node = document.getElementById(id).firstChild, at = node.data.indexOf(text);
    const r = document.createRange(); r.setStart(node, at); r.setEnd(node, at + text.length);
    getSelection().removeAllRanges(); getSelection().addRange(r);
    document.dispatchEvent(new MouseEvent('mouseup', { bubbles: true }));
  };
  window.comment = async (id, text, body, action) => {
    select(id, text);
    for (let i = 0; i < 40 && !$rl('.chip').classList.contains('show'); i++) await new Promise((r) => setTimeout(r, 25));
    $rl('.chip').click();
    $rl('.pop textarea').value = body;
    $rl('.pop [data-a="' + action + '"]').click();
  };
  // By its marker; a detached thread has none, so from the drawer.
  window.openThread = (id) => {
    if ($rl('.pop').dataset.id === id) return true;
    const mk = $rl('.mk[data-id="' + id + '"]');
    if (mk) { mk.click(); return true; }
    if (!$rl('.drawer.show')) $rl('[data-a="drawer"]').click();
    $rl('.drawer .dr[data-id="' + id + '"]').click();
    return true;
  };
  window.inPop = (s) => $rl('.pop.show') && $rl('.pop').querySelector(s);
  true`;

async function open() {
  await b.goto(site.url + "/");
  await b.eval(HELPERS);
  await b.waitFor("$rl('.conn').classList.contains('live')");
}

let thread;

await steps([
  ["overlay loads and connects; the empty dock invites a comment", async () => {
    await open();
    assert.match(await b.eval("$rl('.dock').textContent"), /Select text to comment/);
  }],

  ["select → Comment chip → Send reaches the agent", async () => {
    const waiting = agent.wait(SESSION);
    await b.eval("comment('p1', 'ship the importer', 'make this bold', 'send')");
    const { events } = await waiting;
    assert.equal(events.length, 1);
    const t = events[0].thread;
    assert.equal(t.anchor.kind, "text");
    assert.equal(t.anchor.quote, "ship the importer");
    assert.equal(t.anchor.prefix, "We will ");
    assert.equal(t.messages[0].body, "make this bold");
    thread = t.id;
  }],

  ["the thread gets a highlight and a numbered margin marker", async () => {
    await b.waitFor("hl('redline-open') === 1");
    await b.waitFor(`$rl('.mk.open[data-id="${thread}"]')?.textContent === '1'`);
    // In the margin: right of the text column, not on the text.
    assert.ok(await b.eval(`$rl('.mk[data-id="${thread}"]').getBoundingClientRect().left > document.querySelector('main').getBoundingClientRect().right`));
    assert.match(await b.eval("$rl('.dk-sum').textContent"), /1 thread/);
  }],

  ["until the agent answers, the marker spins and the popover says it is waiting", async () => {
    await b.waitFor(`$rl('.mk[data-id="${thread}"]').classList.contains('wait')`);
    assert.match(await b.eval(`$rl('.mk[data-id="${thread}"]').getAttribute('aria-label')`), /waiting for the agent/);
    await b.eval(`openThread('${thread}')`);
    await b.waitFor("inPop('.note.wait')?.textContent.trim() === 'Sent · waiting for the agent'");
    agent.run("status", thread, "working");
    await b.waitFor("inPop('.note:not(.wait)')?.textContent.trim() === 'Agent is working on it'");
    assert.ok(await b.eval(`$rl('.mk[data-id="${thread}"]').classList.contains('wait')`));
    await b.eval("inPop('[data-a=\"close\"]').click()");
  }],

  ["agent reply: marker turns answered, the dock badges it, the popover shows it live", async () => {
    agent.run("reply", thread, "Done: bold now.");
    await b.waitFor(`$rl('.mk[data-id="${thread}"]').classList.contains('answered')`);
    assert.equal(await b.eval(`$rl('.mk[data-id="${thread}"]').classList.contains('wait')`), false);
    await b.waitFor("$rl('.dk-sum .new')?.textContent.includes('1 new reply')");
    await b.eval(`openThread('${thread}')`);
    await b.waitFor("[...$rl('.pop').querySelectorAll('.msg.agent')].some((m) => m.textContent.includes('Done: bold now.'))");
    await b.waitFor("inPop('.st')?.textContent === 'Answered'");
    await b.waitFor("$rl('.dk-sum .new') === null"); // seen
    await b.waitFor("hl('redline-focus-agent') === 1");
  }],

  ["drafts stay local until sent from the dock, then arrive as one batch", async () => {
    await b.eval("$rl('[data-a=\"close\"]').click()");
    await b.eval("comment('p2', 'Pricing stays flat', 'why flat?', 'draft')");
    await b.eval("comment('title', 'Quarterly', 'rename to Q3', 'draft')");
    await b.waitFor("$rl('[data-a=\"send-drafts\"]')?.textContent.trim() === 'Send 2 drafts'");
    await b.waitFor("$$rl('.mk.draft').length === 2 && hl('redline-draft') === 2");
    const before = agent.json("list", reg.artifact_id);
    assert.equal(before.length, 1, "drafts must not reach the daemon");
    const waiting = agent.wait(SESSION);
    await b.eval("$rl('.dock [data-a=\"send-drafts\"]').click()");
    const { events } = await waiting;
    assert.equal(events.length, 2);
    assert.deepEqual(events.map((e) => e.thread.messages[0].body).sort(), ["rename to Q3", "why flat?"]);
    await b.waitFor("$rl('[data-a=\"send-drafts\"]') === null && $$rl('.mk.draft').length === 0");
  }],

  ["threads survive a reload", async () => {
    await open();
    await b.waitFor("$$rl('.mk').length === 3");
    await b.waitFor("hlAll() === 3");
  }],

  ["Reply sends from the popover and empties the box", async () => {
    await b.eval(`openThread('${thread}')`);
    const waiting = agent.wait(SESSION);
    await b.eval("inPop('textarea').value = 'follow-up'; inPop('[data-a=\"reply\"]').click()");
    const { events } = await waiting;
    assert.equal(events[0].thread.messages.at(-1).body, "follow-up");
    await b.waitFor("$rl('.pop').querySelectorAll('.msg.user').length === 2");
    assert.equal(await b.eval("inPop('textarea').value"), "", "sent text must not come back");
  }],

  ["a typed reply survives a live update", async () => {
    await b.eval("inPop('textarea').value = 'half-typed'");
    agent.run("status", thread, "working");
    await b.waitFor("inPop('.st')?.textContent === 'Working'");
    assert.equal(await b.eval("inPop('textarea').value"), "half-typed");
  }],

  ["working shows the agent's note by the marker and in the popover; a reply clears it", async () => {
    agent.run("status", thread, "working", "--note", "Rebuilding <b>the</b> chart");
    await b.waitFor("inPop('.note .nt')?.textContent === 'Rebuilding <b>the</b> chart'");
    await b.waitFor("$rl('.wnote')?.textContent.includes('Rebuilding <b>the</b> chart')");
    assert.equal(await b.eval("$rl('.wnote b') || inPop('.note b')"), null, "note is escaped");
    assert.ok(await b.eval(`$rl('.mk[data-id="${thread}"]').classList.contains('working')`));
    agent.run("reply", thread, "Rebuilt.");
    await b.waitFor("inPop('.note') === null && $rl('.wnote') === null && inPop('.st').textContent === 'Answered'");
  }],

  ["anchor without context resolves (custom clients)", async () => {
    await b.eval("Redline.client.comment({ kind: 'text', quote: 'enterprise tier' }, 'no prefix')");
    await agent.wait(SESSION); // keep the agent's cursor current
    await b.waitFor("hlAll() === 4");
  }],

  ["edited quote re-attaches by context, removed quote shows detached", async () => {
    await b.eval(`openThread('${thread}')`);
    await b.eval("document.getElementById('p1').firstChild.data = 'We will ship the new importer in March, then harden the sync engine through April.'");
    await b.waitFor("inPop('.tag')?.textContent === 'Edited'");
    await b.waitFor("hlAll() === 4");
    await b.eval("document.getElementById('p1').textContent = 'Something else entirely.'");
    await b.waitFor("inPop('.tag')?.textContent === 'Text changed'"); // answered: a softer tag than 'detached'
    await b.waitFor("hlAll() === 3");
    assert.match(await b.eval("inPop('.q').textContent"), /ship the importer/);
  }],

  ["spot on a canvas: region anchor + screenshot reach the agent, spot pin and thumbnail show", async () => {
    const waiting = agent.wait(SESSION);
    await b.eval("$rl('.dock [data-a=\"spot\"]').click()");
    await b.waitFor("$rl('.capture').style.display === 'block'");
    await b.eval(`(() => { const r = document.getElementById('chart').getBoundingClientRect();
      $rl('.capture').dispatchEvent(new MouseEvent('click', { bubbles: true, clientX: r.left + r.width / 2, clientY: r.top + r.height / 2 }));
      return true; })()`);
    await b.waitFor("$rl('.pop.compose.show') !== null");
    await b.eval("inPop('textarea').value = 'make the bars green'; inPop('[data-a=\"send\"]').click()");
    const { events } = await waiting;
    const a = events[0].thread.anchor;
    assert.equal(a.kind, "region");
    assert.equal(a.selector, "#chart");
    assert.ok(Math.abs(a.ex - 0.5) < 0.01 && Math.abs(a.ey - 0.5) < 0.01, JSON.stringify(a));
    assert.match(a.image, /^att_/);
    const out = join(agent.env.REDLINE_HOME, "shot.png");
    assert.equal(agent.json("image", a.image, "--out", out).mime, "image/png");
    assert.deepEqual([...readFileSync(out).subarray(1, 4)], [0x50, 0x4e, 0x47], "PNG magic");
    await b.waitFor("$$rl('.mk.spot').length === 1");
    await b.eval("$rl('.mk.spot').click()");
    await b.waitFor("[...$rl('.pop').querySelectorAll('img.thumb')].some((i) => i.src.startsWith('blob:') && i.naturalWidth > 0)");
  }],

  ["agent markdown renders, and hostile markdown can't inject anything", async () => {
    await b.eval(`openThread('${thread}')`);
    agent.run("reply", thread, "**Done.** See [docs](https://example.com) and `code`.\n\n- one\n- two");
    const last = "[...$rl('.pop').querySelectorAll('.msg.agent .body')].pop()";
    await b.waitFor(`${last}?.querySelector('strong')?.textContent === 'Done.' && ${last}.querySelectorAll('li').length === 2`);
    assert.equal(await b.eval(`${last}.querySelector('a').getAttribute('href')`), "https://example.com");
    agent.run("reply", thread, "<img src=x onerror=alert(1)> [x](javascript:alert(2)) **<script>alert(3)</script>**");
    await b.waitFor(`${last}?.textContent.includes('onerror')`);
    assert.equal(await b.eval(`${last}.querySelectorAll('img, script, a').length`), 0);
    assert.deepEqual(b.dialogs, []);
  }],

  ["Resolve closes the thread, offers Undo, and the drawer lists it under Resolved", async () => {
    const region = await b.eval("$rl('.mk.spot').dataset.id"); // the spot thread still has a marker
    await b.eval(`openThread('${region}')`);
    await b.eval("inPop('[data-a=\"resolve\"]').click()");
    await b.waitFor(`$rl('.mk[data-id="${region}"]') === null && $rl('.pop.show') === null && $rl('.toast.show')?.textContent.includes('resolved')`);
    assert.equal(agent.json("list", reg.artifact_id).find((t) => t.id === region).status, "resolved");
    await b.eval("$rl('[data-a=\"drawer\"]').click()");
    await b.waitFor("$rl('.drawer.show [data-tab=\"resolved\"]')?.textContent.trim() === 'Resolved 1'");
    await b.eval("$rl('.toast [data-a=\"toast-action\"]').click()"); // Undo
    await b.waitFor(`$rl('.mk[data-id="${region}"]') !== null && $rl('.drawer [data-tab="resolved"]').textContent.trim() === 'Resolved 0'`);
    await agent.wait(SESSION); // the resolve and reopen events: keep the agent's cursor current
  }],

  ["a drawer row opens its thread; Esc closes the popover", async () => {
    await b.eval(`$rl('.drawer .dr[data-id="${thread}"]').click()`);
    await b.waitFor(`$rl('.pop.show')?.dataset.id === '${thread}' && !$rl('.drawer').classList.contains('show')`);
    await b.eval("inPop('textarea').dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true, composed: true }))");
    await b.waitFor("$rl('.pop.show') === null");
  }],

  ["the drawer's Select mode picks rows (Shift+click for a range, or all) and resolves or discards them at once", async () => {
    await b.eval("Redline.client.addDraft({ anchor: { kind: 'text', page: '/', quote: 'March' }, body: 'bulk draft' })");
    await b.eval("$rl('[data-a=\"drawer\"]').click()");
    await b.waitFor("$$rl('.drawer .dr').length > 2");
    assert.equal(await b.eval("$$rl('.drawer [data-a=\"pick\"]').length + ($rl('.dbar') ? 1 : 0)"), 0); // off until asked for
    await b.eval("$rl('[data-a=\"select-mode\"]').click()");
    await b.waitFor("$$rl('.drawer [data-a=\"pick\"]').length > 2 && $rl('.dbar') !== null");
    const picks = "$$rl('.drawer [data-a=\"pick\"]')", bar = "$rl('.dbar').textContent.replace(/\\s+/g, ' ')";
    const threads = await b.eval(`${picks}.slice(1).map((x) => x.dataset.id)`); // the draft lists first
    await b.eval(`${picks}[1].closest('.dr').click()`); // a click anywhere on the row picks it, and opens nothing
    await b.eval(`${picks}.at(-1).dispatchEvent(new MouseEvent('click', { bubbles: true, shiftKey: true }))`);
    await b.waitFor(`${bar}.includes('${threads.length} selected') && $rl('[data-a="pick-all"]').indeterminate`);
    assert.equal(await b.eval("$rl('.pop.show')"), null);
    // Esc leaves Select mode first (dropping the selection), then closes the drawer.
    const esc = "$rl('.drawer').dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true, composed: true }))";
    await b.eval(esc);
    await b.waitFor(`${picks}.length === 0 && $rl('.drawer.show') !== null`);
    await b.eval("$rl('[data-a=\"select-mode\"]').click()");
    await b.waitFor(`${picks}.every((x) => !x.checked)`);
    await b.eval("$rl('[data-a=\"pick-all\"]').click()");
    await b.waitFor(`${bar}.includes('Resolve ${threads.length}') && ${bar}.includes('Discard 1')`);

    await b.eval("$rl('[data-a=\"bulk-resolve\"]').click()");
    await b.waitFor(`$rl('.toast.show')?.textContent.includes('${threads.length} threads resolved')`);
    const status = () => agent.json("list", reg.artifact_id).filter((t) => threads.includes(t.id)).map((t) => t.status);
    assert.deepEqual([...new Set(status())], ["resolved"]);
    await b.eval("$rl('.toast [data-a=\"toast-action\"]').click()"); // Undo
    await b.waitFor(`$rl('.drawer [data-tab="resolved"]').textContent.trim() === 'Resolved 0'`);
    assert.deepEqual([...new Set(status())], ["open"]);
    await agent.wait(SESSION); // the resolve and reopen events: keep the agent's cursor current

    // Resolve took only the threads: the draft is still selected, and the bar now offers only what fits it.
    await b.waitFor(`${picks}[0].checked && ${bar}.includes('1 selected') && !${bar}.includes('Resolve')`);
    await b.eval("$rl('[data-a=\"bulk-discard\"]').click()");
    await b.waitFor("Redline.client.drafts().length === 0 && $rl('.toast.show')?.textContent.includes('1 draft discarded')");
    await b.eval("$rl('.toast [data-a=\"toast-action\"]').click()");
    await b.waitFor("Redline.client.drafts()[0]?.body === 'bulk draft'");
    await b.eval("Redline.client.removeDraft(Redline.client.drafts()[0].id)");
    await b.eval(esc);
    await b.eval(esc);
    await b.waitFor("$rl('.drawer.show') === null");
  }],

  ["a failed send keeps the comment as a draft and offers Retry", async () => {
    await b.eval("window._submit = Redline.client.submit; Redline.client.submit = () => Promise.reject(new Error('offline'))");
    await b.eval("comment('p2', 'enterprise', 'will fail once', 'send')");
    await b.waitFor("$rl('.toast.show')?.getAttribute('role') === 'alert' && $rl('.toast').textContent.includes('kept as a draft')");
    await b.waitFor("Redline.client.drafts().length === 1");
    await b.eval("Redline.client.submit = window._submit");
    const waiting = agent.wait(SESSION);
    await b.eval("$rl('.toast [data-a=\"toast-action\"]').click()");
    const { events } = await waiting;
    assert.equal(events[0].thread.messages[0].body, "will fail once");
    await b.waitFor("Redline.client.drafts().length === 0");
  }],

  ["Alt+C comments on the selection; Esc closes, Ctrl+Enter sends", async () => {
    await b.eval("select('p2', 'enterprise'); document.dispatchEvent(new KeyboardEvent('keydown', { code: 'KeyC', key: 'c', altKey: true, bubbles: true }))");
    await b.waitFor("$rl('.pop.compose.show') !== null");
    await b.eval("inPop('textarea').dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true, composed: true }))");
    await b.waitFor("$rl('.pop.show') === null");
    const waiting = agent.wait(SESSION);
    await b.eval("select('p2', 'enterprise'); document.dispatchEvent(new KeyboardEvent('keydown', { code: 'KeyC', key: 'c', altKey: true, bubbles: true }))");
    await b.waitFor("$rl('.pop.compose.show') !== null");
    await b.eval("inPop('textarea').value = 'via keyboard'; inPop('textarea').dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', ctrlKey: true, bubbles: true, composed: true }))");
    const { events } = await waiting;
    assert.equal(events[0].thread.messages[0].body, "via keyboard");
    assert.equal(events[0].thread.anchor.quote, "enterprise");
  }],

  ["the theme follows the page's background, not the OS", async () => {
    const theme = "document.querySelector('[data-redline]').dataset.theme";
    const dockBg = "getComputedStyle($rl('.dock')).backgroundColor";
    assert.equal(await b.eval(theme), "light");
    assert.equal(await b.eval(dockBg), "rgb(251, 250, 247)");
    await b.send("Emulation.setEmulatedMedia", { features: [{ name: "prefers-color-scheme", value: "dark" }] });
    await new Promise((r) => setTimeout(r, 300));
    assert.equal(await b.eval(theme), "light", "a white page keeps the light theme on a dark OS");
    await b.eval("document.body.style.background = '#111318'");
    await b.waitFor(`${theme} === 'dark' && ${dockBg} === 'rgb(26, 25, 22)'`);
    await b.eval("document.body.style.background = ''");
    await b.waitFor(`${theme} === 'light'`);
    await b.send("Emulation.setEmulatedMedia", { features: [{ name: "prefers-color-scheme", value: "light" }] });
  }],

  ["phone width: a thread opens as a bottom sheet", async () => {
    await b.send("Emulation.setDeviceMetricsOverride", { width: 390, height: 800, deviceScaleFactor: 2, mobile: true });
    await b.waitFor("matchMedia('(max-width: 600px)').matches");
    await b.eval(`openThread('${thread}')`);
    await b.waitFor("(() => { const r = $rl('.pop.show')?.getBoundingClientRect(), d = document.documentElement; return !!r && Math.round(r.width) === innerWidth && Math.abs(r.bottom - innerHeight) < 2 && r.width > d.clientWidth * 0.9; })()");
    await b.eval("$rl('[data-a=\"close\"]').click()");
    await b.send("Emulation.clearDeviceMetricsOverride");
  }],

  ["toggle off: a clean page but a faint mark; remembered; Alt+Shift+R and the API bring it back", async () => {
    const visible = (sel) => `(() => { const el = $rl('${sel}'); return !!el && getComputedStyle(el).display !== 'none'; })()`;
    await b.waitFor("hlAll() > 0 && $$rl('.mk').length > 0");
    await b.eval("$rl('[data-a=\"toggle-hidden\"]').click()");
    await b.waitFor(`hlAll() === 0 && !${visible(".layer")} && Redline.overlay.hidden`);
    assert.equal(await b.eval("$rl('.dock').classList.contains('collapsed') && $rl('.dk-sum') === null || getComputedStyle($rl('.dk-sum')).display === 'none'"), true);
    assert.ok(await b.eval(visible(".dk-mark")), "the mark stays, to turn redline back on");
    await b.eval("select('p2', 'Pricing')");
    await new Promise((r) => setTimeout(r, 200));
    assert.equal(await b.eval("$rl('.chip').classList.contains('show')"), false, "no Comment chip while off");
    await open(); // remembered across reloads
    await b.waitFor(`Redline.overlay.hidden && hlAll() === 0 && !${visible(".layer")}`);
    await b.eval("document.dispatchEvent(new KeyboardEvent('keydown', { code: 'KeyR', key: 'R', altKey: true, shiftKey: true, bubbles: true }))");
    await b.waitFor(`!Redline.overlay.hidden && hlAll() > 0 && ${visible(".layer")}`);
    await b.eval("Redline.overlay.hide()");
    await b.waitFor("hlAll() === 0");
    await b.eval("Redline.overlay.show()");
    await b.waitFor("hlAll() > 0");
  }],

  ["update notice: dock chip → card; Later hides it here, Skip saves it to config.toml", async () => {
    const visible = (sel) => `(() => { const el = $rl('${sel}'); return !!el && getComputedStyle(el).display !== 'none'; })()`;
    assert.equal(await b.eval("$rl('.dk-upd')"), null, "no notice without a known release");
    writeFileSync(join(agent.env.REDLINE_HOME, "update.json"), JSON.stringify({ checked: Date.now(), latest: "99.0.0" }));
    await open();
    await b.waitFor("$rl('.dk-upd')?.textContent === 'Update'");
    await b.eval("$rl('.dk-upd').click()");
    await b.waitFor(`${visible(".upd")} && $rl('.upd h2').textContent === 'redline 99.0.0 is available'`);
    assert.match(await b.eval("$rl('.upd').textContent"), /Run redline update, or ask your agent/);
    assert.equal(await b.eval("$rl('.upd a').href"), "https://github.com/Rfluid/redline/releases/tag/v99.0.0");
    await b.eval("$rl('[data-a=\"upd-later\"]').click()");
    await b.waitFor(`$rl('.dk-upd') === null && !${visible(".upd")}`);
    await open();
    await new Promise((r) => setTimeout(r, 300));
    assert.equal(await b.eval("$rl('.dk-upd')"), null, "Later holds across reloads");
    await b.eval("localStorage.removeItem('redline:update:later')");
    await open();
    await b.waitFor("$rl('.dk-upd') !== null");
    await b.eval("$rl('.dk-upd').click()");
    await b.eval("$rl('[data-a=\"upd-skip\"]').click()");
    await b.waitFor("$rl('.dk-upd') === null && $rl('.toast').textContent.includes('Skipped 99.0.0')");
    assert.match(readFileSync(agent.env.REDLINE_CONFIG, "utf8"), /skip = \["99.0.0"\]/);
    assert.equal(agent.json("update", "skip", "99.0.0").notify, false, "the CLI sees the same setting");
  }],

  ["the overlay adds nothing to the page's DOM", async () => {
    assert.equal(await b.eval("document.head.querySelectorAll('style').length"), 0);
    assert.equal(await b.eval("document.body.querySelectorAll('[data-redline]').length"), 0);
  }],

  ["daemon.log records the WebSocket but no token", () => {
    const log = agent.log();
    assert.match(log, /ws open/);
    assert.match(log, /path=\/api\/artifacts\/art_[0-9a-f]+\/ws/);
    const artifactToken = reg.embed.match(/data-redline-token="([^"]+)"/)[1];
    for (const secret of [artifactToken, agent.adminToken()]) assert.ok(!log.includes(secret), "token in daemon.log");
  }],

  ["no console errors or alerts", () => {
    assert.deepEqual(b.errors, []);
    assert.deepEqual(b.dialogs, []);
  }],
], async () => {
  await b.close();
  site.close();
  agent.stop();
});
