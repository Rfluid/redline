/* redline overlay view: the overlay's state and model (threads, drafts, numbers, where anchors are,
 * seen replies) and how it paints them (markers, highlights, dock, drawer, theme). overlay.js owns
 * the popover and the actions. Never touches the page's DOM. Served in front of overlay.js.
 */
(function () {
  "use strict";
  const R = window.Redline;

  function create({ c, host, markers, dock, phone }) {
    const ui = R.ui;
    // pop: {kind: "compose", anchor, image, hl} | {kind: "thread" | "draft", id}
    // selecting: the drawer is in selection mode; picked: its selected rows (keys: thread or draft ids);
    // shown: the drawer's rows in the current tab.
    const S = { threads: [], where: new Map(), pop: null, sel: null, selecting: false, picked: new Set(), shown: [], hover: null, tab: "open", conn: "connecting", theme: "", hidden: false, update: null };
    // Toggled off ("clean view"): remembered per artifact, in this browser.
    const hiddenKey = `redline:hidden:${c.artifact}`;
    try { S.hidden = localStorage.getItem(hiddenKey) === "1"; } catch {}
    function setHidden(on) {
      S.hidden = on;
      try { localStorage.setItem(hiddenKey, on ? "1" : "0"); } catch {}
      host.classList.toggle("off", on);
    }

    // ---- seen agent replies (per browser), for "new reply" badges ----------------------------------
    const seenKey = `redline:seen:${c.artifact}`;
    let seen = {};
    try { seen = JSON.parse(localStorage.getItem(seenKey) || "{}"); } catch {}
    const agentCount = (t) => t.messages.filter((m) => m.author === "agent").length;
    const unseen = (t) => agentCount(t) > (seen[t.id] || 0);
    function markSeen(t) {
      if (!unseen(t)) return;
      seen[t.id] = agentCount(t);
      try { localStorage.setItem(seenKey, JSON.stringify(seen)); } catch {}
    }

    // ---- model ------------------------------------------------------------------------------------
    const newDrafts = () => c.drafts().filter((d) => !d.thread_id);
    // Thread numbers follow creation order; unsent drafts come after them.
    function numbers() {
      const m = new Map();
      [...S.threads].sort((a, b) => a.created - b.created).forEach((t, i) => m.set(t.id, i + 1));
      newDrafts().forEach((d) => m.set(d.id, m.size + 1));
      return m;
    }
    function locateAll() {
      S.where = new Map();
      for (const t of S.threads) S.where.set(t.id, R.anchors.locate(t.anchor));
      for (const d of newDrafts()) S.where.set(d.id, R.anchors.locate(d.anchor));
    }
    const focusId = () => S.hover || (S.pop && S.pop.id);
    // A snapshot from GET /threads can be older than a WebSocket event already delivered. Keep the newer.
    const newer = (a, b) => (!a || b.updated > a.updated || (b.updated === a.updated && b.messages.length > a.messages.length)) ? b : a;
    function upsert(t) {
      const i = S.threads.findIndex((x) => x.id === t.id);
      if (i === -1) S.threads.push(t); else S.threads[i] = newer(S.threads[i], t);
    }

    // ---- painting ---------------------------------------------------------------------------------
    function place() {
      const nums = numbers(), showResolved = dock.isOpen() && S.tab === "resolved", f = focusId(), list = [];
      for (const t of S.threads) {
        if (t.status === "resolved" && !showResolved) continue;
        list.push({ id: t.id, n: nums.get(t.id), state: t.status, ...S.where.get(t.id), on: f === t.id, unseen: unseen(t), note: t.note, wait: R.ui.popover.pending(t) });
      }
      for (const d of newDrafts()) list.push({ id: d.id, n: nums.get(d.id), state: "draft", ...S.where.get(d.id), on: f === d.id });
      markers.draw(list, phone());
    }

    function highlight() {
      if (!window.CSS || !CSS.highlights || typeof Highlight === "undefined") return;
      const g = { "redline-draft": [], "redline-open": [], "redline-answered": [], "redline-focus": [], "redline-focus-agent": [] }, f = focusId();
      const add = (id, state) => {
        const r = (S.where.get(id) || {}).target;
        if (!(r instanceof Range)) return;
        const agent = state === "answered";
        g[id === f ? (agent ? "redline-focus-agent" : "redline-focus") : state === "draft" ? "redline-draft" : agent ? "redline-answered" : "redline-open"].push(r);
      };
      if (!S.hidden) {
        for (const t of S.threads) if (t.status !== "resolved") add(t.id, t.status);
        for (const d of newDrafts()) add(d.id, "draft");
      }
      for (const [name, ranges] of Object.entries(g)) CSS.highlights.set(name, new Highlight(...ranges));
    }

    function renderDock() {
      const open = S.threads.filter((t) => t.status !== "resolved").length;
      dock.render({ theme: S.theme, open, resolved: S.threads.length - open, unseen: S.threads.filter(unseen).length, drafts: c.drafts().length, conn: S.conn, hidden: S.hidden, update: S.update });
    }

    function renderDrawer() {
      const nums = numbers(), y = (id) => { const el = markers.el(id); return el ? parseFloat(el.style.top) : Infinity; };
      const rows = c.drafts().map((d) => d.thread_id
        ? { id: d.thread_id, key: d.id, n: nums.get(d.thread_id), state: "reply", anchor: (S.threads.find((t) => t.id === d.thread_id) || {}).anchor }
        : { id: d.id, key: d.id, n: nums.get(d.id), state: "draft", anchor: d.anchor });
      for (const t of [...S.threads].sort((a, b) => y(a.id) - y(b.id) || a.created - b.created)) {
        rows.push({ id: t.id, key: t.id, n: nums.get(t.id), state: t.status, anchor: t.anchor, unseen: unseen(t), resolved: t.status === "resolved" });
      }
      // The selection only ever holds rows on screen: switching tabs, sending or resolving drops the rest.
      S.shown = rows.filter((r) => !!r.resolved === (S.tab === "resolved"));
      const keys = new Set(S.shown.map((r) => r.key));
      for (const k of S.picked) if (!keys.has(k)) S.picked.delete(k);
      dock.renderDrawer(rows, S.tab, c.drafts().length, S.selecting ? S.picked : null);
    }

    function paint() { place(); highlight(); renderDock(); if (dock.isOpen()) renderDrawer(); }

    // ---- theme: from the page, and the document's ::highlight() sheet (CSSOM only, no element) ----
    let hlSheet = null;
    try { hlSheet = new CSSStyleSheet(); document.adoptedStyleSheets = [...document.adoptedStyleSheets, hlSheet]; } catch { hlSheet = null; }
    function applyTheme() {
      const t = ui.theme.detect();
      if (t === S.theme) return;
      S.theme = t;
      host.dataset.theme = t;
      if (hlSheet) hlSheet.replaceSync(ui.theme.highlightCSS(host));
    }

    function targetRect(id) {
      const w = S.where.get(id) || {};
      if (w.point) return new DOMRect(w.point.x - scrollX, w.point.y - scrollY, 0, 0);
      const t = w.target;
      return t && t.getClientRects ? (t.getClientRects()[0] || null) : null;
    }

    return { S, setHidden, numbers, locateAll, upsert, unseen, markSeen, newDrafts, focusId, place, highlight, renderDock, renderDrawer, paint, applyTheme, targetRect };
  }

  window.Redline = Object.assign(R, { ui: Object.assign(R.ui || {}, { view: { create } }) });
})();
