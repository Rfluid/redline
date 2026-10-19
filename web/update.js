/* redline update notice: the dock's "Update" chip and the card it opens (.design/components.md › Update
 * notice). Skip and Turn off are saved in redline's config.toml through the daemon, so they hold for the
 * CLI and agents too; Later is this browser only (a day, localStorage). Inside the overlay's shadow root.
 * Served in front of overlay.js.
 */
(function () {
  "use strict";
  const R = window.Redline;
  const LATER = "redline:update:later", DAY = 864e5;

  function mount(root, { c, S, renderDock, toast }) {
    const { icons } = R.ui, { esc } = R.ui.popover;
    const card = document.createElement("div");
    card.className = "upd";
    card.setAttribute("role", "dialog");
    card.setAttribute("aria-label", "redline update");
    root.append(card);
    let info = null;

    const snoozed = () => {
      try { const l = JSON.parse(localStorage.getItem(LATER) || "null"); return !!l && l.version === info.latest && l.until > Date.now(); } catch { return false; }
    };
    function sync() {
      S.update = info && info.notify && !snoozed() ? info.latest : null;
      if (!S.update) close();
      renderDock();
    }
    async function check() {
      try { info = await c.update(); } catch { info = null; } // an older daemon has no such route: no notice
      sync();
    }

    function open() {
      const v = esc(info.latest), notes = /^https:\/\//.test(info.notes || "") ? esc(info.notes) : "";
      card.innerHTML = `<div class="uh"><h2>redline ${v} is available</h2>
          <button class="btn g sm i" data-a="upd-close" aria-label="Close">${icons.close}</button></div>
        <p>You have ${esc(info.current)}. Run <code>${esc(info.command)}</code>, or ask your agent to update redline.</p>
        <div class="ua">${notes ? `<a class="btn sm" href="${notes}" target="_blank" rel="noopener noreferrer">What's new</a>` : ""}<span class="sp"></span>
          <button class="btn g sm" data-a="upd-later">Later</button><button class="btn sm" data-a="upd-skip">Skip ${v}</button></div>
        <button class="lnk" data-a="upd-off">Turn off update notices</button>`;
      card.classList.add("show");
      card.querySelector("button").focus({ preventScroll: true });
    }
    function close() { card.classList.remove("show"); }
    card.addEventListener("keydown", (e) => { if (e.key === "Escape") { e.stopPropagation(); close(); } });

    async function dismiss(body, done) {
      try { info = await c.dismissUpdate(body); sync(); toast(done); } catch { toast("Couldn't save that. The notice stays for now.", { error: true }); }
    }
    const actions = {
      update: () => (card.classList.contains("show") ? close() : open()),
      "upd-close": close,
      "upd-later": () => {
        try { localStorage.setItem(LATER, JSON.stringify({ version: info.latest, until: Date.now() + DAY })); } catch {}
        sync();
      },
      "upd-skip": () => dismiss({ version: info.latest }, `Skipped ${info.latest}. A newer release will still show.`),
      "upd-off": () => dismiss({ all: true }, "Update notices are off. To turn them back on: redline update on"),
    };
    return { check, close, actions };
  }

  window.Redline = Object.assign(R, { ui: Object.assign(R.ui || {}, { update: { mount } }) });
})();
