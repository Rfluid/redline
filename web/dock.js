/* redline dock: the always-there entry point (summary, spot, drafts, connection), the all-threads
 * drawer, the toast that replaces alert(), and the screen-reader live region
 * (.design/components.md › Dock / Drawer / Toast). Inside the overlay's shadow root.
 * Served in front of overlay.js.
 */
(function () {
  "use strict";
  const R = window.Redline;
  const plural = (n, one, many) => `${n} ${n === 1 ? one : many}`;
  const MAC = /Mac|iP(hone|ad|od)/.test(navigator.platform || navigator.userAgent);

  function mount(root, host) {
    const { icons } = R.ui, { esc } = R.ui.popover;
    const tpl = document.createElement("template");
    tpl.innerHTML = `<div class="dock" role="region" aria-label="redline"></div>
      <div class="drawer" role="dialog" aria-label="redline threads"></div>
      <div class="toast"></div><div class="sr" aria-live="polite"></div>`;
    root.append(tpl.content);
    const dock = root.querySelector(".dock"), drawer = root.querySelector(".drawer");
    const toastEl = root.querySelector(".toast"), live = root.querySelector('[aria-live]');
    /** s = {theme, open, resolved, unseen, drafts, conn: "connecting"|"live"|"off", hidden, update: newer version or null} */
    function render(s) {
      const total = s.open + s.resolved + s.drafts;
      const count = (n, one, many) => `<b>${n}</b> ${n === 1 ? one : many}`;
      const summary = s.open ? count(s.open, "thread", "threads") : s.resolved ? count(s.resolved, "resolved", "resolved") : count(s.drafts, "draft", "drafts");
      const conn = { connecting: ["", "Connecting…"], live: ["live", "Live"], off: ["off", "Reconnecting… comments are kept"] }[s.conn];
      const keys = MAC ? "⌥⇧R" : "Alt+Shift+R", tip = `${s.hidden ? "Show" : "Hide"} redline (${keys})`;
      dock.classList.toggle("collapsed", s.hidden);
      dock.innerHTML = `<button class="dk-mark" data-a="toggle-hidden" aria-pressed="${s.hidden}" aria-label="${tip}" title="${tip}">${icons.mark(s.theme)}${s.hidden && s.unseen ? `<span class="badge" aria-hidden="true"></span>` : ""}</button>
        ${total ? `<button class="dk-sum" data-a="drawer" aria-expanded="${drawer.classList.contains("show")}">${summary}${s.unseen ? ` <span class="new">· ${plural(s.unseen, "new reply", "new replies")}</span>` : ""}</button>`
          : `<span class="dk-empty">Select text to comment, or press <kbd>${MAC ? "⌥C" : "Alt+C"}</kbd></span>`}
        <button class="btn g i" data-a="spot" aria-label="Comment on a spot" title="Comment on a spot: a chart, an image, anything">${icons.spot}</button>
        ${s.drafts ? `<button class="btn p" data-a="send-drafts">Send ${plural(s.drafts, "draft", "drafts")}${icons.send}</button>` : ""}
        ${s.update ? `<button class="dk-upd" data-a="update" aria-haspopup="dialog" title="redline ${esc(s.update)} is available">Update</button>` : ""}
        <span class="conn ${conn[0]}" title="${s.conn === "live" ? "Connected to your agent" : "Not connected to the local redline service"}">${conn[1]}</span>`;
    }

    /** rows = [{id, n, state, anchor, status, unseen, resolved}], drafts first; tab = "open" | "resolved" */
    function renderDrawer(rows, tab, drafts) {
      const open = rows.filter((r) => !r.resolved), done = rows.filter((r) => r.resolved), shown = tab === "resolved" ? done : open;
      const status = (r) => r.unseen ? ["new", "New reply"] : ["", { draft: "Draft", reply: "Draft reply", open: "Open", working: "Working", answered: "Answered", resolved: "Resolved" }[r.state]];
      drawer.innerHTML = `<div class="dh"><h2>Threads</h2><div class="seg" role="group" aria-label="Show">
          <button data-a="tab" data-tab="open" aria-pressed="${tab !== "resolved"}">Open ${open.length}</button>
          <button data-a="tab" data-tab="resolved" aria-pressed="${tab === "resolved"}">Resolved ${done.length}</button></div><span class="sp"></span>
          ${drafts ? `<button class="btn p sm" data-a="send-drafts">Send ${plural(drafts, "draft", "drafts")}</button>` : ""}
          <button class="btn g sm i" data-a="close-drawer" aria-label="Close">${icons.close}</button></div>
        <div class="dl">${shown.length ? shown.map((r) => {
          const [cls, label] = status(r);
          const a = r.anchor, words = a && a.kind === "region" ? `${icons.spot}${esc(a.selector || "spot")}` : `“${esc((a && (a.quote || a.label || a.selector || a.key)) || "page")}”`;
          return `<button class="dr" data-a="row" data-id="${esc(r.id)}"><span class="mk ${r.state === "reply" ? "draft" : r.state}">${r.state === "resolved" ? icons.check : r.n}</span>
            <span class="q">${words}</span><span class="s ${cls}">${label}</span></button>`;
        }).join("") : `<div class="dempty">${tab === "resolved" ? "Nothing resolved yet." : "No open comments. Select any text on the page to start."}</div>`}</div>`;
    }

    const isOpen = () => drawer.classList.contains("show");
    function setDrawer(open) {
      drawer.classList.toggle("show", open);
      host.classList.toggle("drawer-open", open);
      const sum = dock.querySelector('[data-a="drawer"]');
      if (sum) sum.setAttribute("aria-expanded", String(open));
    }

    let toastTimer = 0;
    /** A short message above the dock; `action` = {label, run}. Errors use role=alert. */
    function toast(msg, { action, error } = {}) {
      clearTimeout(toastTimer);
      toastEl.setAttribute("role", error ? "alert" : "status");
      toastEl.innerHTML = `<span>${esc(msg)}</span>${action ? `<button class="btn sm" data-a="toast-action">${esc(action.label)}</button>` : ""}`;
      toastEl.classList.add("show");
      toastEl.onclick = (e) => { if (e.target.closest('[data-a="toast-action"]')) { hide(); action.run(); } };
      const hide = () => toastEl.classList.remove("show");
      if (!action) toastTimer = setTimeout(hide, 6000);
    }

    /** Say something to screen readers (polite). */
    function announce(text) {
      live.textContent = "";
      requestAnimationFrame(() => { live.textContent = text; });
    }

    return { render, renderDrawer, setDrawer, isOpen, toast, announce, dock, drawer };
  }

  window.Redline = Object.assign(R, { ui: Object.assign(R.ui || {}, { dock: { mount } }) });
})();
