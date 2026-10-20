/* redline overlay input: the selection chip, keyboard shortcuts, the popover's focus trap,
 * outside clicks and marker hover (.design/accessibility.md › Keyboard). Wired up by overlay.js,
 * which owns the state. Listens to the page; never changes it. Served in front of overlay.js.
 */
(function () {
  "use strict";
  const R = window.Redline;
  const editable = (el) => el && (el.isContentEditable || /^(INPUT|TEXTAREA|SELECT)$/.test(el.tagName));

  function mount(o) {
    const { root, host, pop, chipEl, S } = o;
    const ours = (e) => e.composedPath().includes(host);

    // ---- selection -> chip: mouseup for mice, touchend for phones (no mouseup after a long press) --
    function onSelectEnd(e) {
      if (ours(e) || S.hidden) return;
      setTimeout(() => {
        const a = R.anchors.fromSelection();
        if (!a) { chipEl.classList.remove("show"); S.sel = null; return; }
        const range = getSelection().getRangeAt(0), rects = range.getClientRects();
        const last = rects[rects.length - 1] || range.getBoundingClientRect();
        S.sel = { anchor: a, rect: range.getBoundingClientRect() };
        chipEl.classList.add("show");
        const w = chipEl.offsetWidth, touch = e.type === "touchend";
        const left = Math.max(8, Math.min(last.right - w, innerWidth - w - 8));
        const top = touch ? last.bottom + 12 : last.top - 8 - chipEl.offsetHeight;
        chipEl.style.left = `${left}px`;
        chipEl.style.top = `${Math.max(8, Math.min(top, innerHeight - chipEl.offsetHeight - 8))}px`;
      }, e.type === "touchend" ? 300 : 0);
    }
    document.addEventListener("mouseup", onSelectEnd);
    document.addEventListener("touchend", onSelectEnd);

    // ---- Alt+Shift+R: redline off / on. Alt+C (Option+C): comment on the selection. ---------------
    // Neither fires while typing in the page's own fields.
    document.addEventListener("keydown", (e) => {
      if (!e.altKey || e.ctrlKey || e.metaKey || (!ours(e) && editable(document.activeElement))) return;
      if (e.shiftKey && e.code === "KeyR") { e.preventDefault(); o.toggleHidden(); return; }
      if (e.shiftKey || e.code !== "KeyC" || ours(e) || S.hidden) return;
      const a = R.anchors.fromSelection();
      if (!a) return;
      e.preventDefault();
      o.openPop({ kind: "compose", anchor: a, hl: getSelection().getRangeAt(0).getBoundingClientRect() });
    });

    // ---- inside our UI: Esc, Ctrl/⌘+Enter, Alt+Enter, and Tab kept inside the popover ------------
    root.addEventListener("keydown", (e) => {
      if (e.key === "Escape") {
        if (S.pop) { e.preventDefault(); o.closePop(); } else if (o.dock.isOpen()) { e.preventDefault(); if (S.selecting) o.selectMode(false); else o.toggleDrawer(false); }
        return;
      }
      if (!S.pop || !e.composedPath().includes(pop)) return;
      const compose = S.pop.kind === "compose", thread = S.pop.kind === "thread";
      if (e.key === "Enter" && (e.ctrlKey || e.metaKey)) {
        e.preventDefault();
        if (compose) o.submitCompose("send"); else if (thread) o.sendReply(false);
      } else if (e.key === "Enter" && e.altKey) {
        e.preventDefault();
        if (compose) o.submitCompose("draft"); else if (thread) o.sendReply(true);
      } else if (e.key === "Tab") {
        const f = [...pop.querySelectorAll("button, textarea, a[href]")].filter((x) => !x.disabled);
        if (!f.length) return;
        const first = f[0], last = f[f.length - 1];
        if (e.shiftKey && root.activeElement === first) { e.preventDefault(); last.focus(); }
        else if (!e.shiftKey && root.activeElement === last) { e.preventDefault(); first.focus(); }
      }
    });

    // ---- a click outside the popover and its marker closes it (not a compose box with text in it) -
    document.addEventListener("pointerdown", (e) => {
      if (!S.pop) return;
      const path = e.composedPath();
      if (path.includes(pop) || path.some((n) => n.dataset && (n.dataset.a === "marker" || n.dataset.a === "row"))) return;
      const ta = pop.querySelector("textarea");
      if (S.pop.kind === "compose" && ta && ta.value.trim()) return;
      o.closePop(false);
    }, true);

    // ---- marker hover / focus -> focused highlight ----------------------------------------------
    const hover = (id) => { if (S.hover !== id) { S.hover = id; o.highlight(); } };
    const markerOf = (e) => e.target.closest && e.target.closest(".mk");
    for (const [on, off] of [["pointerover", "pointerout"], ["focusin", "focusout"]]) {
      o.markers.layer.addEventListener(on, (e) => { const m = markerOf(e); if (m) hover(m.dataset.id); });
      o.markers.layer.addEventListener(off, (e) => {
        const to = e.relatedTarget;
        if (!(to && to.closest && to.closest(".mk"))) hover(null);
      });
    }
  }

  window.Redline = Object.assign(R, { ui: Object.assign(R.ui || {}, { overlayInput: { mount } }) });
})();
