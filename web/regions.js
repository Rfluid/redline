/* redline regions: the overlay's "comment on a spot" picker. UI only, inside the overlay's shadow
 * root; never touches the page's DOM. Region threads are drawn by markers.js as spot pins.
 * Served in front of overlay.js (one /overlay.js).
 */
(function () {
  "use strict";

  const CSS_TEXT = `
  .capture { position: fixed; inset: 0; z-index: 2147483647; display: none; cursor: crosshair; background: var(--rl-mark-tint-draft); }
  .capture .hint { position: fixed; top: 16px; left: 50%; transform: translateX(-50%); display: flex; align-items: center; gap: 8px; margin: 0;
                   padding: 8px 14px; border-radius: 8px; background: var(--rl-primary); color: var(--rl-on-primary); box-shadow: var(--rl-shadow-dock);
                   font: 500 13px/18px var(--rl-font-ui); white-space: nowrap; }
  .capture .hint kbd { background: none; color: inherit; opacity: .75; }`;

  /** Adds the picker to `root`. Returns { pick }. */
  function mount(root) {
    const style = document.createElement("style");
    style.textContent = CSS_TEXT;
    const capture = document.createElement("div");
    capture.className = "capture";
    capture.innerHTML = `<div class="hint" role="status">Click the spot to comment on · <kbd>Esc</kbd> cancels</div>`;
    root.append(style, capture);

    /** Waits for one click; resolves {x, y} (viewport), or null on Esc. */
    function pick() {
      capture.style.display = "block";
      return new Promise((resolve) => {
        const done = (v) => { capture.style.display = "none"; capture.onclick = null; removeEventListener("keydown", esc, true); resolve(v); };
        const esc = (e) => { if (e.key === "Escape") { e.preventDefault(); done(null); } };
        addEventListener("keydown", esc, true);
        capture.onclick = (e) => { e.preventDefault(); e.stopPropagation(); done({ x: e.clientX, y: e.clientY }); };
      });
    }
    return { pick };
  }

  window.Redline = Object.assign(window.Redline || {}, { regions: { mount } });
})();
