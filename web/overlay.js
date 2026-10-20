/* redline overlay: optional inline-comment UI on top of any HTML page (design: .design/).
 * Load after client.js. It never touches the page's DOM tree: highlights use the CSS Custom
 * Highlight API and the UI lives in its own shadow root.
 *
 * Select text -> "Comment" (or the dock's spot button -> click a chart or image) -> Send, or Save
 * draft and send drafts from the dock. Threads show as numbered markers in the margin; a marker
 * opens its thread, where the agent's replies appear live.
 */
(function () {
  "use strict";
  const R = window.Redline;
  if (!R || !R.client) { console.warn("redline overlay: load client.js with data-redline-artifact first"); return; }
  const c = R.client, ui = R.ui, P = ui.popover;
  const MAC = /Mac|iP(hone|ad|od)/.test(navigator.platform || navigator.userAgent);

  const host = document.createElement("div");
  host.setAttribute("data-redline", "");
  document.documentElement.appendChild(host);
  const root = host.attachShadow({ mode: "open" });
  root.innerHTML = `<style>${R.overlayStyle}</style>
<button class="chip" data-a="chip" aria-label="Comment on the selection">${ui.icons.pen}Comment <kbd>${MAC ? "⌥C" : "Alt+C"}</kbd></button>
<div class="scrim"></div><div class="pop" role="dialog" aria-modal="false"></div>`;
  const chipEl = root.querySelector(".chip"), pop = root.querySelector(".pop"), scrim = root.querySelector(".scrim");
  const regions = R.regions.mount(root), markers = ui.markers.mount(root), dock = ui.dock.mount(root, host);
  const phone = () => matchMedia("(max-width: 600px)").matches;

  const V = ui.view.create({ c, host, markers, dock, phone });
  const { S, setHidden, numbers, upsert, newDrafts, place, highlight, renderDock, renderDrawer, paint, applyTheme, targetRect, locateAll, markSeen } = V;
  const upd = ui.update.mount(root, { c, S, renderDock, toast: dock.toast });
  const bulk = ui.bulk.create({ c, S, dock, renderDrawer, refresh, fail: (m) => fail(m), closePop });

  // ---- popover ----------------------------------------------------------------------------------
  function openPop(p) {
    S.pop = p;
    chipEl.classList.remove("show");
    pop.className = `pop show${p.kind === "compose" ? " compose" : ""}`;
    pop.dataset.id = p.id || "";
    renderPop(true);
    paint();
  }

  function closePop(restoreFocus = true) {
    if (!S.pop) return;
    const id = S.pop.id;
    S.pop = null;
    pop.className = "pop";
    pop.dataset.id = "";
    scrim.classList.remove("show");
    paint();
    if (restoreFocus && id && markers.el(id)) markers.el(id).focus({ preventScroll: true });
  }

  function renderPop(first) {
    const p = S.pop;
    if (!p) return;
    const old = pop.querySelector("textarea"), typed = old ? old.value : "", had = old && root.activeElement === old;
    const caret = old ? [old.selectionStart, old.selectionEnd] : null, nums = numbers();
    if (p.kind === "compose") {
      if (!first) return;
      pop.innerHTML = P.compose(p.anchor, p.image);
      pop.setAttribute("aria-label", "New comment");
    } else if (p.kind === "draft") {
      const d = newDrafts().find((x) => x.id === p.id);
      if (!d) return closePop(false);
      pop.innerHTML = P.draft(d, nums.get(d.id), (S.where.get(d.id) || {}).match);
      pop.setAttribute("aria-label", `Draft ${nums.get(d.id)}`);
    } else {
      const t = S.threads.find((x) => x.id === p.id);
      if (!t) return closePop(false);
      pop.innerHTML = P.thread(t, { n: nums.get(t.id), match: (S.where.get(t.id) || {}).match, replyDrafts: c.drafts().filter((d) => d.thread_id === t.id) });
      pop.setAttribute("aria-label", `Thread ${nums.get(t.id)}`);
      markSeen(t);
    }
    for (const img of pop.querySelectorAll("img[data-att]")) c.attachmentURL(img.dataset.att).then((u) => { img.src = u; }, () => img.remove());
    const ta = pop.querySelector("textarea");
    if (first) {
      placePop();
      (ta || pop.querySelector("button")).focus({ preventScroll: true });
    } else if (ta) {
      ta.value = typed;
      if (had) { ta.focus({ preventScroll: true }); ta.setSelectionRange(...caret); }
    }
    if (ta) grow(ta);
  }
  function placePop() {
    const p = S.pop, m = p.id && markers.el(p.id);
    scrim.classList.toggle("show", phone());
    P.place(pop, m ? m.getBoundingClientRect() : null, p.hl || (p.id && targetRect(p.id)), p.kind === "compose");
  }
  const grow = (ta) => { if (ta.name === "redline-reply") { ta.style.height = "auto"; ta.style.height = `${ta.scrollHeight}px`; } };

  function reveal(id, smooth) {
    const r = targetRect(id);
    if (r) scrollTo({ top: r.top + scrollY - innerHeight / 3, behavior: smooth && !matchMedia("(prefers-reduced-motion: reduce)").matches ? "smooth" : "auto" });
  }

  // ---- actions ----------------------------------------------------------------------------------
  const fail = (msg, retry) => dock.toast(msg, { error: true, ...(retry ? { action: { label: "Retry", run: retry } } : {}) });

  async function submitCompose(action) {
    const p = S.pop, ta = pop.querySelector("textarea"), body = ta.value.trim();
    if (!body) { ta.focus(); return; }
    const draft = { anchor: p.anchor, body, ...(p.image ? { image: p.image } : {}) };
    closePop(false);
    if (action === "draft") { c.addDraft(draft); return; }
    try { await c.comment(p.anchor, body, p.image); dock.announce("Comment sent. Waiting for the agent."); } catch {
      c.addDraft(draft);
      const id = c.drafts().at(-1).id;
      fail("Couldn't send. Your comment is kept as a draft.", () => sendDrafts([id]));
    }
    await refresh();
  }

  async function sendDrafts(ids) {
    try { await c.submitDrafts(ids); } catch { fail("Couldn't send. Your drafts are kept.", () => sendDrafts(ids)); }
    await refresh();
  }

  async function sendReply(asDraft) {
    const id = S.pop.id, ta = pop.querySelector("textarea"), body = ta.value.trim();
    if (!body) return;
    ta.value = ""; // clear first: the live event for this reply re-renders the popover mid-request
    grow(ta);
    if (asDraft) { c.addDraft({ thread_id: id, body }); return; }
    try { await c.reply(id, body); dock.announce("Reply sent. Waiting for the agent."); } catch {
      const now = pop.querySelector("textarea");
      if (now && !now.value) now.value = body;
      fail("Couldn't send your reply. It's back in the box.");
    }
    await refresh();
  }

  async function setStatus(id, status) {
    const n = numbers().get(id);
    if (status === "resolved") closePop();
    try { await c.setStatus(id, status); } catch { fail("Couldn't update the thread."); return; }
    if (status === "resolved") dock.toast(`Thread ${n} resolved.`, { action: { label: "Undo", run: () => setStatus(id, "open") } });
    await refresh();
  }

  async function pinSpot() {
    closePop(false);
    const p = await regions.pick();
    if (!p) return;
    host.style.visibility = "hidden"; // so elementFromPoint sees the page, not our UI
    const anchor = R.anchors.fromPoint(p.x, p.y), el = document.elementFromPoint(p.x, p.y);
    host.style.visibility = "";
    const image = el ? await R.anchors.snapshot(el, p.x, p.y).catch(() => null) : null;
    openPop({ kind: "compose", anchor, image, hl: new DOMRect(p.x, p.y, 0, 0) });
  }

  /** Everything off (a clean page, but for a faint mark to turn it back on), or back on. */
  function toggleHidden(on = !S.hidden) {
    if (on === S.hidden) return;
    if (on) { closePop(false); toggleDrawer(false); upd.close(); chipEl.classList.remove("show"); }
    setHidden(on);
    paint();
    dock.announce(on ? "redline hidden. Alt+Shift+R shows it again." : "redline shown.");
  }

  function toggleDrawer(open) {
    if (!open) { S.selecting = false; S.picked.clear(); }
    dock.setDrawer(open);
    if (open) renderDrawer();
    place();
  }

  const isDraft = (id) => newDrafts().some((d) => d.id === id);
  const ACTIONS = {
    chip: () => S.sel && openPop({ kind: "compose", anchor: S.sel.anchor, hl: S.sel.rect }),
    marker: (id) => (S.pop && S.pop.id === id ? closePop() : openPop({ kind: isDraft(id) ? "draft" : "thread", id })),
    close: () => closePop(),
    send: () => submitCompose("send"),
    draft: () => submitCompose("draft"),
    reply: () => sendReply(false),
    resolve: () => setStatus(S.pop.id, "resolved"),
    reopen: () => setStatus(S.pop.id, "open"),
    "send-draft": (id) => sendDrafts([id]),
    "drop-draft": (id) => { c.removeDraft(id); if (S.pop && S.pop.id === id) closePop(false); },
    goto: () => S.pop && S.pop.id && reveal(S.pop.id, true),
    "toggle-hidden": () => toggleHidden(),
    drawer: () => toggleDrawer(!dock.isOpen()),
    "close-drawer": () => toggleDrawer(false),
    tab: (_, el) => { S.tab = el.dataset.tab; renderDrawer(); place(); },
    row: (id) => {
      toggleDrawer(false);
      reveal(id, false);
      openPop({ kind: isDraft(id) ? "draft" : "thread", id });
    },
    spot: pinSpot,
    "send-drafts": () => sendDrafts(),
    ...upd.actions,
    ...bulk.actions,
  };
  root.addEventListener("click", (e) => {
    const el = e.target.closest && e.target.closest("[data-a]");
    if (el && ACTIONS[el.dataset.a]) ACTIONS[el.dataset.a](el.dataset.id, el, e);
  });
  pop.addEventListener("input", (e) => { if (e.target.tagName === "TEXTAREA") grow(e.target); });

  R.ui.overlayInput.mount({ root, host, pop, chipEl, S, ACTIONS, closePop, openPop, submitCompose, sendReply, toggleDrawer, toggleHidden, selectMode: bulk.mode, dock, markers, highlight, place });

  // ---- data -------------------------------------------------------------------------------------
  function dataChanged() { locateAll(); paint(); if (S.pop && S.pop.kind !== "compose") renderPop(false); }
  async function refresh() {
    try { (await c.threads()).forEach(upsert); } catch (e) { console.warn("redline:", e); }
    dataChanged();
  }

  // The page may change under us (HMR, scripts, the agent's edits): re-anchor after it settles.
  // Observing never mutates the page; our own UI lives outside <body>.
  let settle = 0;
  function relocate() {
    const before = S.pop && (S.where.get(S.pop.id) || {}).match;
    applyTheme();
    locateAll();
    paint();
    if (S.pop && S.pop.kind !== "compose" && (S.where.get(S.pop.id) || {}).match !== before) renderPop(false);
  }
  const later = () => { clearTimeout(settle); settle = setTimeout(relocate, 150); };
  new MutationObserver(later).observe(document.body, { subtree: true, childList: true, characterData: true });
  new MutationObserver(later).observe(document.documentElement, { attributes: true, attributeFilter: ["class", "style", "data-theme"] });
  new MutationObserver(later).observe(document.body, { attributes: true, attributeFilter: ["class", "style", "data-theme"] });
  addEventListener("resize", later);
  matchMedia("(prefers-color-scheme: dark)").addEventListener("change", later);
  // Crossing the phone breakpoint (rotation, resize): the open popover becomes a sheet, or back.
  matchMedia("(max-width: 600px)").addEventListener("change", () => { later(); if (S.pop) requestAnimationFrame(placePop); });
  let frame = 0;
  addEventListener("scroll", () => {
    chipEl.classList.remove("show");
    if (!frame) frame = requestAnimationFrame(() => { frame = 0; place(); });
  }, { capture: true, passive: true });

  let lastNote = 0;
  c.on("event", (ev) => {
    upsert(ev.thread);
    dataChanged();
    if (ev.author !== "agent") return;
    const t = ev.thread, n = numbers().get(t.id);
    if (ev.kind === "message") {
      if (!(S.pop && S.pop.id === t.id)) markers.fresh(t.id);
      dock.announce(`Agent replied on thread ${n}`);
    } else if (t.status === "working" && t.note && Date.now() - lastNote > 10000) {
      lastNote = Date.now();
      dock.announce(`Agent is working on thread ${n}: ${t.note}`);
    }
  });
  c.on("open", () => { S.conn = "live"; refresh(); upd.check(); });
  c.on("close", () => { S.conn = "off"; renderDock(); });
  c.on("drafts", dataChanged);
  // For agents, tests and screenshots: Redline.overlay.hide() / show() / toggle() / hidden.
  R.overlay = { hide: () => toggleHidden(true), show: () => toggleHidden(false), toggle: () => toggleHidden(), get hidden() { return S.hidden; } };
  host.classList.toggle("off", S.hidden);
  applyTheme();
  paint();
  c.connect();
})();
