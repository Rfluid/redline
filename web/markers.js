/* redline markers: numbered thread markers in the right margin of the text they comment on, spot
 * pins for region threads, leaders for markers pushed off their line, and the agent's working note
 * (.design/layout.md › Margin markers). Inside the overlay's shadow root, in page coordinates, so
 * they scroll with the page. Served in front of overlay.js.
 */
(function () {
  "use strict";
  const R = window.Redline;
  const SIZE = 26, GAP = 4, PUSH_LEADER = 48, EDGE = 40;
  const LABEL = { draft: "draft", open: "open", working: "agent working", answered: "answered", resolved: "resolved" };

  // The nearest block-level ancestor wide enough to be a text column.
  function blockOf(node) {
    for (let el = node.nodeType === 1 ? node : node.parentElement; el && el !== document.body; el = el.parentElement) {
      if (/^(block|flex|grid|flow-root|list-item|table)$/.test(getComputedStyle(el).display) && el.getBoundingClientRect().width >= 240) return el;
    }
    return document.body;
  }

  // Where an item's marker goes, in page px: {x, y, kind: "margin"|"spot"|"inline", line?: {x, y}}.
  function spot(it, phone) {
    if (it.point) return { x: it.point.x, y: it.point.y - SIZE, kind: "spot" };
    const t = it.target;
    let first, last, right;
    if (t instanceof Range) {
      const rects = [...t.getClientRects()].filter((r) => r.width > 0 || r.height > 0);
      if (!rects.length) return null;
      [first, last] = [rects[0], rects[rects.length - 1]];
      right = blockOf(t.endContainer).getBoundingClientRect().right;
    } else if (t && t.getBoundingClientRect) {
      first = last = t.getBoundingClientRect();
      if (!first.width && !first.height) return null;
      right = first.right;
    } else return null;
    if (phone) return { x: last.right - 4 + scrollX, y: last.top - 11 + scrollY, kind: "inline" }; // a superscript at the end
    const x = Math.min(right + 16, innerWidth - EDGE);
    const mid = first.top + Math.min(first.height, 40) / 2;
    return { x: x + scrollX, y: mid - SIZE / 2 + scrollY, kind: "margin", line: { x: first.right + scrollX + 4, y: mid + scrollY } };
  }

  function mount(root) {
    const layer = document.createElement("div");
    layer.className = "layer";
    const deco = document.createElement("div"); // leaders and notes: redrawn wholesale, never focusable
    layer.append(deco);
    root.append(layer);
    const els = new Map(); // id -> marker button (kept across draws, so focus and hover survive)
    let order = "";

    /** items: [{id, n, state, target, point, match, on, unseen, note, wait}] */
    function draw(items, phone) {
      const placed = [];
      for (const it of items) {
        const at = spot(it, phone);
        if (at) placed.push({ ...it, at });
      }
      // Push overlapping margin markers down; far-pushed ones get a leader to their line.
      const margin = placed.filter((p) => p.at.kind === "margin").sort((a, b) => a.at.y - b.at.y);
      for (let i = 1; i < margin.length; i++) {
        const prev = margin[i - 1].at, cur = margin[i].at;
        if (Math.abs(cur.x - prev.x) < SIZE + GAP && cur.y < prev.y + SIZE + GAP) cur.y = prev.y + SIZE + GAP;
      }
      placed.sort((a, b) => a.at.y - b.at.y || a.at.x - b.at.x);

      const seen = new Set();
      let decoHTML = "";
      for (const p of placed) {
        seen.add(p.id);
        let el = els.get(p.id);
        if (!el) {
          el = document.createElement("button");
          el.dataset.a = "marker";
          el.dataset.id = p.id;
          els.set(p.id, el);
        }
        const cls = `mk ${p.state}${p.at.kind === "spot" ? " spot" : ""}${p.at.kind === "inline" ? " inline" : ""}${p.match === "moved" ? " moved" : ""}${p.wait ? " wait" : ""}${p.on ? " on" : ""}${el.classList.contains("fresh") ? " fresh" : ""}`;
        if (el.className !== cls) el.className = cls;
        const text = p.state === "resolved" ? "" : String(p.n);
        if (p.state === "resolved") { if (!el.querySelector("svg")) el.innerHTML = R.ui.icons.check; } else if (el.textContent !== text) el.textContent = text;
        el.style.left = `${p.at.x}px`;
        el.style.top = `${p.at.y}px`;
        const label = p.state === "draft" ? `Draft ${p.n}` : `Thread ${p.n}, ${p.wait && p.state === "open" ? "waiting for the agent" : LABEL[p.state]}${p.unseen ? ", new reply" : ""}`;
        if (el.getAttribute("aria-label") !== label) el.setAttribute("aria-label", label);
        const { line } = p.at;
        if (line && p.at.y + SIZE / 2 - line.y > PUSH_LEADER) {
          const dx = p.at.x - line.x, dy = p.at.y + SIZE / 2 - line.y;
          decoHTML += `<div class="leader" style="left:${line.x}px;top:${line.y}px;width:${Math.hypot(dx, dy)}px;transform:rotate(${Math.atan2(dy, dx)}rad)"></div>`;
        }
        if (p.state === "working" && p.note && p.at.kind !== "inline") {
          const note = p.note.length > 40 ? `${p.note.slice(0, 39)}…` : p.note;
          // Right of the marker when there's room, else left of it.
          const room = innerWidth + scrollX - (p.at.x + SIZE + 8) - 8, flip = room < 200;
          const left = flip ? p.at.x - 8 : p.at.x + SIZE + 8, max = flip ? Math.min(280, p.at.x - scrollX - 16) : room;
          decoHTML += `<div class="wnote${flip ? " flip" : ""}" style="left:${left}px;top:${p.at.y + 2}px;max-width:${max}px"><span class="dots"><i></i><i></i><i></i></span>Agent: “${esc(note)}”</div>`;
        }
      }
      for (const [id, el] of els) if (!seen.has(id)) { el.remove(); els.delete(id); }
      deco.innerHTML = decoHTML;
      // Tab order follows page order. Re-append only when it changed: moving a node drops its focus.
      const want = placed.map((p) => p.id).join();
      if (want !== order || placed.some((p) => !els.get(p.id).isConnected)) {
        const focused = root.activeElement;
        for (const p of placed) layer.append(els.get(p.id));
        if (focused && focused.dataset && els.get(focused.dataset.id) === focused && root.activeElement !== focused) focused.focus();
        order = want;
      }
    }

    /** Pulse a marker's halo twice (a new agent reply). */
    function fresh(id) {
      const el = els.get(id);
      if (!el) return;
      el.classList.remove("fresh");
      void el.offsetWidth; // restart the animation
      el.classList.add("fresh");
      setTimeout(() => el.classList.remove("fresh"), 1300);
    }

    return { draw, fresh, layer, el: (id) => els.get(id) };
  }

  const esc = (s) => String(s).replace(/[&<>"]/g, (ch) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" })[ch]);
  window.Redline = Object.assign(R || {}, { ui: Object.assign((R && R.ui) || {}, { markers: { mount } }) });
})();
