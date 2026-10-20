/* redline bulk actions: the drawer's selection mode (its Select toggle). Pick rows (click, Shift+click
 * for a range, or all) and act on them at once: resolve or reopen threads, send or discard drafts (.design/components.md › Drawer).
 * The selection lives in S.picked; the view prunes it to the rows on screen. Served in front of overlay.js.
 */
(function () {
  "use strict";
  const R = window.Redline;
  const plural = (n, one, many) => `${n} ${n === 1 ? one : many}`;

  function create({ c, S, dock, renderDrawer, refresh, fail, closePop }) {
    let anchor = null; // the last row clicked, where a Shift+click range starts
    // A click on a row's label reaches the checkbox as a second, synthetic click: read Shift off the pointer.
    let shift = false;
    dock.drawer.addEventListener("pointerdown", (e) => { shift = e.shiftKey; }, true);

    function pick(key, el, e) {
      const keys = S.shown.map((r) => r.key), from = keys.indexOf(anchor), to = keys.indexOf(key);
      const range = (shift || (e && e.shiftKey)) && from !== -1 && to !== -1 ? keys.slice(Math.min(from, to), Math.max(from, to) + 1) : [key];
      for (const k of range) if (el.checked) S.picked.add(k); else S.picked.delete(k);
      anchor = key;
      shift = false;
      renderDrawer();
    }
    function pickAll(_, el) {
      if (el.checked) S.shown.forEach((r) => S.picked.add(r.key)); else S.picked.clear();
      renderDrawer();
    }
    /** Selection mode on or off; leaving it drops the selection. */
    function mode(on = !S.selecting) {
      S.selecting = on;
      S.picked.clear();
      anchor = null;
      renderDrawer();
    }

    /** The selected rows, emptied from the selection, that match `test`. */
    function take(test) {
      const rows = S.shown.filter((r) => S.picked.has(r.key) && test(r));
      rows.forEach((r) => S.picked.delete(r.key));
      if (S.pop && rows.some((r) => r.id === S.pop.id || r.key === S.pop.id)) closePop(false);
      return rows;
    }

    /** Sets `status` on each thread; returns the ids that took it. */
    async function setMany(ids, status) {
      const res = await Promise.allSettled(ids.map((id) => c.setStatus(id, status)));
      const ok = ids.filter((_, i) => res[i].status === "fulfilled");
      if (ok.length < ids.length) fail(`Couldn't update ${ids.length - ok.length} of ${plural(ids.length, "thread", "threads")}.`);
      await refresh();
      return ok;
    }

    async function resolve() {
      const ok = await setMany(take((r) => ["open", "working", "answered"].includes(r.state)).map((r) => r.id), "resolved");
      if (ok.length) dock.toast(`${plural(ok.length, "thread", "threads")} resolved.`, { action: { label: "Undo", run: () => setMany(ok, "open") } });
    }
    async function reopen() {
      const ok = await setMany(take((r) => r.resolved).map((r) => r.id), "open");
      if (ok.length) dock.announce(`${plural(ok.length, "thread", "threads")} reopened.`);
    }

    const isDraft = (r) => r.state === "draft" || r.state === "reply";
    async function send() {
      const ids = take(isDraft).map((r) => r.key);
      try { await c.submitDrafts(ids); dock.announce(`${plural(ids.length, "draft", "drafts")} sent. Waiting for the agent.`); }
      catch { fail("Couldn't send. Your drafts are kept."); }
      await refresh();
    }
    function discard() {
      const ids = new Set(take(isDraft).map((r) => r.key)), gone = c.drafts().filter((d) => ids.has(d.id));
      gone.forEach((d) => c.removeDraft(d.id));
      // addDraft keeps the id it is given, so Undo brings back the same drafts and numbers.
      dock.toast(`${plural(gone.length, "draft", "drafts")} discarded.`, { action: { label: "Undo", run: () => gone.forEach((d) => c.addDraft(d)) } });
    }

    return {
      mode,
      actions: { "select-mode": () => mode(), pick, "pick-all": pickAll, "bulk-resolve": resolve, "bulk-reopen": reopen, "bulk-send": send, "bulk-discard": discard },
    };
  }

  window.Redline = Object.assign(R, { ui: Object.assign(R.ui || {}, { bulk: { create } }) });
})();
