/* redline overlay styles (shadow root only), from .design/. The tokens mirror .design/tokens.css
 * (`python3 .design/check.py` keeps them in sync). The overlay sets data-theme on the host from the
 * page's own background. Served in front of overlay.js.
 */
(function () {
  "use strict";
  const TOKENS = `
  :host { all: initial;
    --rl-fg: #1B1A17; --rl-muted: #5E5A53; --rl-hint: #6E695F;
    --rl-surface: #FBFAF7; --rl-raised: #FFFFFF; --rl-line: #E4E0D8; --rl-line-soft: #F0EDE7; --rl-code: #EFECE6;
    --rl-primary: #1B1A17; --rl-on-primary: #F3EFE7; --rl-focus: #2B4C7E;
    --rl-mark: #D7261E; --rl-mark-fg: #B01E17;
    --rl-mark-tint-draft: rgba(215, 38, 30, .06); --rl-mark-tint: rgba(215, 38, 30, .12); --rl-mark-tint-focus: rgba(215, 38, 30, .22);
    --rl-agent: #2B4C7E; --rl-agent-tint: rgba(43, 76, 126, .12); --rl-agent-tint-focus: rgba(43, 76, 126, .22);
    --rl-note: #B26A00; --rl-note-fg: #8A4B00; --rl-note-tint: #FFF4E0; --rl-note-line: #F2D9A8;
    --rl-done: #2F6B3F; --rl-done-tint: rgba(47, 107, 63, .10);
    --rl-shadow-pop: 0 12px 32px rgba(27, 26, 23, .14), 0 2px 6px rgba(27, 26, 23, .06);
    --rl-shadow-dock: 0 6px 20px rgba(27, 26, 23, .12); --rl-shadow-sheet: 0 -10px 30px rgba(27, 26, 23, .18);
    --rl-scrim: rgba(27, 26, 23, .18);
    --rl-font-ui: system-ui, -apple-system, "Segoe UI", Roboto, sans-serif;
    --rl-font-quote: "Iowan Old Style", "Charter", "Georgia", "Cambria", serif;
    --rl-font-mono: ui-monospace, "SF Mono", Menlo, Consolas, monospace;
    --rl-ease-out: cubic-bezier(.2, .8, .2, 1); --rl-dur-fast: 120ms; --rl-dur-slide: 180ms; }
  :host([data-theme="dark"]) {
    --rl-fg: #F3EFE7; --rl-muted: #B9B3A8; --rl-hint: #8E887D;
    --rl-surface: #1A1916; --rl-raised: #23221E; --rl-line: #3A3832; --rl-line-soft: #2C2B27; --rl-code: #2C2B27;
    --rl-primary: #F3EFE7; --rl-on-primary: #1B1A17; --rl-focus: #8DB0E8;
    --rl-mark: #FF5A4E; --rl-mark-fg: #FF8A80;
    --rl-mark-tint-draft: rgba(255, 90, 78, .10); --rl-mark-tint: rgba(255, 90, 78, .20); --rl-mark-tint-focus: rgba(255, 90, 78, .32);
    --rl-agent: #8DB0E8; --rl-agent-tint: rgba(141, 176, 232, .16); --rl-agent-tint-focus: rgba(141, 176, 232, .28);
    --rl-note: #E0A84A; --rl-note-fg: #F2C77A; --rl-note-tint: rgba(224, 168, 74, .14); --rl-note-line: rgba(224, 168, 74, .35);
    --rl-done: #7FC08F; --rl-done-tint: rgba(127, 192, 143, .14);
    --rl-shadow-pop: 0 16px 40px rgba(0, 0, 0, .5); --rl-shadow-dock: 0 8px 24px rgba(0, 0, 0, .45);
    --rl-shadow-sheet: 0 -10px 30px rgba(0, 0, 0, .5); --rl-scrim: rgba(0, 0, 0, .4); }
  @media (prefers-reduced-motion: reduce) { :host { --rl-dur-fast: 0ms; --rl-dur-slide: 0ms; } }`;

  const BASE = `
  * { box-sizing: border-box; margin: 0; font: 400 14px/20px var(--rl-font-ui); color: inherit; }
  .sr { position: absolute; width: 1px; height: 1px; overflow: hidden; clip: rect(0 0 0 0); white-space: nowrap; }
  kbd { font: 400 11px/1 var(--rl-font-mono); padding: 2px 5px; border-radius: 4px; background: var(--rl-code); color: var(--rl-muted); }
  button { font: 500 13px/18px var(--rl-font-ui); cursor: pointer; }
  :focus-visible { outline: 2px solid var(--rl-focus); outline-offset: 2px; }
  svg { display: block; flex: none; }
  .btn { height: 34px; padding: 0 12px; display: inline-flex; align-items: center; justify-content: center; gap: 6px; border-radius: 8px;
         border: 1px solid var(--rl-line); background: var(--rl-raised); color: var(--rl-fg); white-space: nowrap; }
  .btn:hover { background: var(--rl-line-soft); } .btn:active { transform: translateY(.5px); }
  .btn.p { border-color: var(--rl-primary); background: var(--rl-primary); color: var(--rl-on-primary); font-weight: 600; padding: 0 14px; }
  .btn.p:hover { filter: brightness(1.08); } :host([data-theme="dark"]) .btn.p:hover { filter: brightness(.92); }
  .btn.g { border-color: transparent; background: transparent; color: var(--rl-muted); }
  .btn.g:hover { background: var(--rl-line-soft); color: var(--rl-fg); }
  .btn.i { width: 34px; padding: 0; } .btn.sm { height: 28px; } .btn.sm.i { width: 28px; }
  .btn svg { width: 14px; height: 14px; }
  .btn:disabled { opacity: .45; pointer-events: none; }
  @supports not selector(::-webkit-scrollbar) { * { scrollbar-width: thin; scrollbar-color: var(--rl-line) transparent; } }
  ::-webkit-scrollbar { width: 10px; height: 10px; } ::-webkit-scrollbar-track, ::-webkit-scrollbar-corner { background: transparent; }
  ::-webkit-scrollbar-thumb { background: var(--rl-line); border: 3px solid transparent; border-radius: 999px; background-clip: padding-box; }
  ::-webkit-scrollbar-thumb:hover { background-color: var(--rl-hint); } ::-webkit-scrollbar-button { display: none; }`;

  const MARKS = `
  .layer { position: absolute; top: 0; left: 0; width: 0; height: 0; z-index: 2147483640; }
  .mk { position: absolute; width: 26px; height: 26px; padding: 0; border-radius: 50%; border: 2px solid var(--rl-mark); background: var(--rl-raised);
        color: var(--rl-mark); font: 600 12px/1 var(--rl-font-ui); font-feature-settings: "tnum"; display: flex; align-items: center; justify-content: center;
        animation: rl-in var(--rl-dur-fast) var(--rl-ease-out); transition: box-shadow var(--rl-dur-fast); }
  .mk.draft { border-style: dashed; background: transparent; }
  .mk.working { border-color: var(--rl-agent); color: var(--rl-agent); }
  .mk.answered { border-color: var(--rl-agent); background: var(--rl-agent); color: #FFFFFF; }
  :host([data-theme="dark"]) .mk.answered { color: #1B1A17; }
  .mk.resolved { width: 18px; height: 18px; margin: 4px; border: 0; background: var(--rl-line); color: var(--rl-muted); }
  .mk.resolved svg { width: 10px; height: 10px; stroke-width: 3; }
  .mk.spot { border-radius: 50% 50% 50% 4px; }
  .mk.moved { -webkit-mask: conic-gradient(from 35deg, transparent 0 20deg, #000 20deg); mask: conic-gradient(from 35deg, transparent 0 20deg, #000 20deg); }
  .mk:hover, .mk.on { box-shadow: 0 0 0 4px var(--rl-mark-tint); }
  .mk.working:hover, .mk.working.on, .mk.answered:hover, .mk.answered.on { box-shadow: 0 0 0 4px var(--rl-agent-tint-focus); }
  .mk.fresh { animation: rl-halo .6s ease-out 2; }
  .mk.wait::before { content: ""; position: absolute; inset: -6px; border-radius: 50%; pointer-events: none;
        background: conic-gradient(transparent 0 35%, currentColor);
        -webkit-mask: radial-gradient(farthest-side, transparent calc(100% - 2px), #000 calc(100% - 1.5px));
        mask: radial-gradient(farthest-side, transparent calc(100% - 2px), #000 calc(100% - 1.5px)); animation: rl-spin 1.1s linear infinite; }
  .mk.wait.inline::before { inset: -4px; }
  .mk.inline { width: 16px; height: 16px; font-size: 9.5px; border-width: 1.5px; }
  .leader { position: absolute; height: 1px; background: var(--rl-line); transform-origin: 0 50%; pointer-events: none; }
  .wnote { position: absolute; height: 22px; padding: 0 10px; display: flex; align-items: center; gap: 8px; border-radius: 999px; pointer-events: none;
           background: var(--rl-surface); border: 1px solid var(--rl-line); font: 400 12px/1 var(--rl-font-ui); color: var(--rl-muted); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .wnote.flip { transform: translateX(-100%); }
  .dots { display: inline-flex; gap: 3px; } .dots i { width: 4px; height: 4px; border-radius: 50%; background: var(--rl-agent); animation: rl-pulse 1.2s infinite; }
  .dots i:nth-child(2) { animation-delay: .2s; } .dots i:nth-child(3) { animation-delay: .4s; }
  @keyframes rl-spin { to { transform: rotate(1turn); } }
  @keyframes rl-in { from { opacity: 0; transform: scale(.8); } }
  @keyframes rl-halo { 50% { box-shadow: 0 0 0 8px var(--rl-agent-tint-focus); } }
  @keyframes rl-pulse { 0%, 100% { opacity: .35; } 50% { opacity: 1; } }
  @keyframes rl-rise { from { opacity: 0; transform: translateY(4px); } }
  @keyframes rl-up { from { transform: translateY(100%); } }
  @keyframes rl-left { from { transform: translateX(100%); } }`;

  const POP = `
  .chip { position: fixed; z-index: 2147483646; display: none; height: 32px; padding: 0 6px 0 12px; align-items: center; gap: 8px; border: 0; border-radius: 8px;
          background: var(--rl-primary); color: var(--rl-on-primary); box-shadow: var(--rl-shadow-dock); animation: rl-rise var(--rl-dur-fast) var(--rl-ease-out); }
  .chip.show { display: inline-flex; } .chip svg { width: 14px; height: 14px; color: var(--rl-mark); }
  .chip kbd { background: var(--rl-on-primary); color: var(--rl-primary); opacity: .75; }
  .pop { position: absolute; z-index: 2147483646; display: none; flex-direction: column; width: 360px; max-width: calc(100vw - 16px);
         max-height: min(560px, calc(100vh - 16px)); background: var(--rl-surface); color: var(--rl-fg); border: 1px solid var(--rl-line); border-radius: 12px;
         box-shadow: var(--rl-shadow-pop); animation: rl-rise var(--rl-dur-fast) var(--rl-ease-out); }
  .pop.show { display: flex; } .pop.compose { width: 380px; }
  .ph { display: flex; align-items: center; gap: 8px; padding: 10px 10px 10px 16px; border-bottom: 1px solid var(--rl-line); }
  .ph .num { font: 600 12px/16px var(--rl-font-ui); color: var(--rl-muted); font-feature-settings: "tnum"; } .ph .sp { flex: 1; }
  .pb { overflow: auto; padding: 12px 16px 4px; display: flex; flex-direction: column; gap: 12px; min-height: 0; }
  .pf { padding: 10px 12px 12px; display: flex; align-items: center; justify-content: flex-end; gap: 8px; flex-wrap: wrap; }
  .q { font: italic 400 15.5px/22px var(--rl-font-quote); color: var(--rl-muted); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; cursor: pointer; }
  .q u { text-decoration: none; border-bottom: 1.5px solid var(--rl-mark); color: var(--rl-fg); font: inherit; }
  .q svg { display: inline; vertical-align: -2px; margin-right: 4px; width: 14px; height: 14px; color: var(--rl-mark); }
  .thumb { display: block; max-width: 100%; max-height: 96px; border: 1px solid var(--rl-line); border-radius: 6px; }
  .msg { display: flex; gap: 10px; }
  .av { flex: none; width: 22px; height: 22px; border-radius: 50%; display: flex; align-items: center; justify-content: center;
        background: var(--rl-mark); color: #FFFFFF; font: 600 11px/1 var(--rl-font-ui); }
  .av.agent { background: var(--rl-agent); color: var(--rl-surface); } .av svg { width: 12px; height: 12px; stroke-width: 2.2; }
  .who { font: 400 12px/16px var(--rl-font-ui); color: var(--rl-muted); } .who b { font: 600 12px/16px var(--rl-font-ui); color: var(--rl-fg); }
  .who b.agent { color: var(--rl-agent); }
  .body { white-space: pre-wrap; overflow-wrap: anywhere; min-width: 0; }
  .body.md { white-space: normal; } .body.md p, .body.md ul, .body.md ol, .body.md pre { margin: 2px 0 6px; }
  .body.md ul, .body.md ol { padding-left: 18px; } .body.md strong, .body.md b { font-weight: 600; } .body.md em { font-style: italic; }
  .body code { font: 400 12.5px/1.4 var(--rl-font-mono); background: var(--rl-code); padding: 1px 4px; border-radius: 4px; }
  .body pre { overflow: auto; background: var(--rl-code); padding: 8px; border-radius: 6px; } .body pre code { padding: 0; background: none; }
  .body a { color: var(--rl-agent); text-decoration: underline; }
  .note { display: flex; gap: 10px; align-items: center; font: italic 400 13px/18px var(--rl-font-ui); color: var(--rl-agent); animation: rl-rise var(--rl-dur-fast) var(--rl-ease-out); }
  .note .bub { flex: none; display: inline-flex; align-items: center; height: 22px; padding: 0 9px; border-radius: 11px 11px 11px 4px; background: var(--rl-agent-tint); }
  .note .nt { min-width: 0; overflow-wrap: anywhere; }
  .note.wait { color: var(--rl-muted); } .note.wait .av { background: transparent; color: var(--rl-agent); box-shadow: inset 0 0 0 1.5px var(--rl-agent); opacity: .55; }
  .note.wait .bub { background: var(--rl-code); } .note.wait .dots i { background: var(--rl-muted); }
  .rdraft { border: 1px dashed var(--rl-line); border-radius: 9px; padding: 8px 10px; display: flex; flex-direction: column; gap: 6px; }
  .row { display: flex; gap: 6px; justify-content: flex-end; flex-wrap: wrap; }
  textarea { width: 100%; min-height: 84px; resize: vertical; padding: 10px 12px; border: 1px solid var(--rl-line); border-radius: 9px;
             background: var(--rl-raised); color: var(--rl-fg); font: 400 14px/20px var(--rl-font-ui); }
  textarea:focus { outline: 3px solid var(--rl-agent-tint); border-color: var(--rl-focus); }
  .reply { flex: 1 1 100%; display: flex; align-items: flex-end; gap: 6px; padding: 4px 4px 4px 0; border: 1px solid var(--rl-line); border-radius: 9px; background: var(--rl-raised); }
  .reply:focus-within { border-color: var(--rl-focus); outline: 3px solid var(--rl-agent-tint); }
  .reply textarea { min-height: 30px; max-height: 120px; padding: 5px 0 5px 12px; border: 0; resize: none; outline: 0; background: transparent; }
  .reply .btn { height: 30px; width: 30px; border-radius: 7px; }
  .hint { font: 400 11.5px/16px var(--rl-font-ui); color: var(--rl-hint); margin-right: auto; } .hint kbd { background: none; padding: 0; color: inherit; font: inherit; }
  .scrim { position: fixed; inset: 0; z-index: 2147483645; display: none; background: var(--rl-scrim); } .scrim.show { display: block; }`;

  const TAGS = `
  .st { display: inline-flex; align-items: center; gap: 5px; padding: 3px 8px; border-radius: 999px; font: 600 11.5px/16px var(--rl-font-ui); }
  .st::before { content: ""; width: 6px; height: 6px; border-radius: 50%; border: 1.5px solid currentColor; }
  .st.open { color: var(--rl-mark-fg); background: var(--rl-mark-tint-draft); } .st.draft { color: var(--rl-mark-fg); background: var(--rl-mark-tint-draft); }
  .st.draft::before { border-style: dashed; }
  .st.working, .st.answered { color: var(--rl-agent); background: var(--rl-agent-tint); } .st.answered::before { background: currentColor; }
  .st.resolved { color: var(--rl-done); background: var(--rl-done-tint); }
  .st.resolved::before { border: 0; width: 8px; height: 5px; border-radius: 0; border-left: 2px solid; border-bottom: 2px solid; transform: rotate(-45deg) translate(1px, -1px); }
  .tag { align-self: flex-start; font: 400 12px/16px var(--rl-font-ui); color: var(--rl-muted); border: 1px solid var(--rl-line); padding: 3px 8px; border-radius: 6px; }
  .tag.detached { color: var(--rl-note-fg); background: var(--rl-note-tint); border-color: var(--rl-note-line); }`;

  const DOCK = `
  .dock { position: fixed; right: 24px; bottom: 24px; z-index: 2147483643; display: flex; align-items: center; gap: 4px; padding: 6px;
          background: var(--rl-surface); color: var(--rl-fg); border: 1px solid var(--rl-line); border-radius: 999px; box-shadow: var(--rl-shadow-dock);
          transition: right var(--rl-dur-slide) var(--rl-ease-out), bottom var(--rl-dur-slide) var(--rl-ease-out); }
  .dock .btn { border-radius: 999px; height: 36px; } .dock .btn.i { width: 36px; }
  .dk-mark { width: 36px; height: 36px; padding: 0; border: 0; border-radius: 999px; background: transparent; display: flex; align-items: center; justify-content: center; }
  .dk-sum { border: 0; background: transparent; height: 36px; padding: 0 12px; border-radius: 999px; display: inline-flex; gap: 4px; align-items: center;
            color: var(--rl-muted); white-space: nowrap; transition: background var(--rl-dur-fast), color var(--rl-dur-fast); }
  .dk-sum b { font: 600 13px/18px var(--rl-font-ui); color: var(--rl-fg); font-variant-numeric: tabular-nums; }
  .dk-sum:hover, .dk-sum[aria-expanded="true"] { background: var(--rl-line-soft); color: var(--rl-fg); } .dk-sum .new { font: 600 13px/18px var(--rl-font-ui); color: var(--rl-agent); }
  .dk-empty { padding: 0 12px 0 4px; font: 400 13px/18px var(--rl-font-ui); color: var(--rl-muted); white-space: nowrap; }
  .conn { display: inline-flex; align-items: center; gap: 6px; height: 28px; padding: 0 10px; border-left: 1px solid var(--rl-line); font: 400 12px/1 var(--rl-font-ui); color: var(--rl-muted); white-space: nowrap; }
  .conn::before { content: ""; width: 7px; height: 7px; border-radius: 50%; border: 1.5px solid currentColor; }
  .conn.live { color: var(--rl-done); } .conn.live::before { background: currentColor; } .conn.off { color: var(--rl-note-fg); } .conn.off::before { color: var(--rl-note); }
  .dock.collapsed > :not(.dk-mark) { display: none; }
  .dock.collapsed { padding: 4px; opacity: .35; box-shadow: none; transition: opacity var(--rl-dur-fast); }
  .dock.collapsed:hover, .dock.collapsed:focus-within { opacity: 1; }
  .dk-mark { position: relative; } .dk-mark .badge { position: absolute; top: 3px; right: 3px; width: 9px; height: 9px; border-radius: 50%;
    background: var(--rl-agent); border: 2px solid var(--rl-surface); }
  :host(.off) .layer, :host(.off) .chip, :host(.off) .pop, :host(.off) .scrim, :host(.off) .drawer, :host(.off) .toast, :host(.off) .upd { display: none !important; }
  .drawer { position: fixed; top: 0; right: 0; bottom: 0; z-index: 2147483644; width: 360px; display: none; flex-direction: column;
            background: var(--rl-surface); color: var(--rl-fg); border-left: 1px solid var(--rl-line); box-shadow: var(--rl-shadow-pop);
            animation: rl-left var(--rl-dur-slide) var(--rl-ease-out); }
  .drawer.show { display: flex; } :host(.drawer-open) .dock { right: 384px; }
  .dh { display: flex; align-items: center; gap: 10px; padding: 12px 12px 12px 16px; border-bottom: 1px solid var(--rl-line); }
  .dh h2 { font: 600 14px/20px var(--rl-font-ui); } .dh .sp { flex: 1; }
  .seg { display: inline-flex; gap: 2px; padding: 2px; background: var(--rl-code); border-radius: 7px; }
  .seg button { border: 0; background: transparent; padding: 3px 9px; border-radius: 5px; font: 400 12px/16px var(--rl-font-ui); color: var(--rl-muted); white-space: nowrap; }
  .seg button[aria-pressed="true"] { background: var(--rl-raised); color: var(--rl-fg); font-weight: 600; }
  .dl { overflow: auto; flex: 1; }
  .dr { width: 100%; display: flex; align-items: center; gap: 12px; padding: 11px 16px; border: 0; border-bottom: 1px solid var(--rl-line-soft); background: transparent; text-align: left; }
  .dr:hover { background: var(--rl-line-soft); }
  .dr .mk { position: static; flex: none; width: 22px; height: 22px; font-size: 11px; animation: none; }
  .dr .q { display: block; flex: 1; min-width: 0; font-size: 14.5px; color: var(--rl-fg); } .dr .s { font: 400 12px/16px var(--rl-font-ui); color: var(--rl-muted); white-space: nowrap; }
  .dr .s.new { color: var(--rl-agent); font-weight: 600; }
  .dempty { padding: 32px 24px; text-align: center; color: var(--rl-muted); }
  .toast { position: fixed; right: 24px; bottom: 84px; z-index: 2147483647; display: none; align-items: center; gap: 10px; max-width: calc(100vw - 32px);
           padding: 10px 10px 10px 14px; border-radius: 10px; background: var(--rl-primary); color: var(--rl-on-primary); font: 400 13px/18px var(--rl-font-ui);
           box-shadow: var(--rl-shadow-pop); animation: rl-rise var(--rl-dur-fast) var(--rl-ease-out); }
  .toast.show { display: flex; } .toast::before { content: ""; flex: none; width: 7px; height: 7px; border-radius: 50%; background: var(--rl-mark); }
  .toast .btn { height: 28px; background: transparent; color: inherit; border-color: color-mix(in srgb, currentColor 30%, transparent); }
  .dk-upd { display: inline-flex; align-items: center; gap: 6px; height: 28px; padding: 0 10px; border: 0; border-radius: 999px; white-space: nowrap;
            background: var(--rl-note-tint); color: var(--rl-note-fg); font: 600 12px/1 var(--rl-font-ui); }
  .dk-upd::before { content: ""; width: 6px; height: 6px; border-radius: 50%; background: var(--rl-note); } .dk-upd:hover { box-shadow: inset 0 0 0 1px var(--rl-note-line); }
  .upd { position: fixed; right: 24px; bottom: 84px; z-index: 2147483646; width: 320px; max-width: calc(100vw - 32px); display: none; flex-direction: column; gap: 10px;
         padding: 12px 12px 12px 16px; background: var(--rl-surface); color: var(--rl-fg); border: 1px solid var(--rl-line); border-radius: 12px;
         box-shadow: var(--rl-shadow-pop); animation: rl-rise var(--rl-dur-fast) var(--rl-ease-out); }
  .upd.show { display: flex; } :host(.drawer-open) .upd { right: 384px; }
  .uh { display: flex; align-items: center; gap: 8px; } .uh h2 { flex: 1; font: 600 14px/20px var(--rl-font-ui); }
  .upd p { padding-right: 4px; font-size: 13px; line-height: 18px; color: var(--rl-muted); }
  .upd code { font: 400 12px/1 var(--rl-font-mono); padding: 2px 5px; border-radius: 4px; background: var(--rl-code); color: var(--rl-fg); }
  .ua { display: flex; align-items: center; gap: 6px; } .ua .sp { flex: 1; } a.btn { text-decoration: none; }
  .upd .lnk { align-self: flex-start; padding: 0; border: 0; background: none; font: 400 12px/16px var(--rl-font-ui); color: var(--rl-muted);
              text-decoration: underline; text-underline-offset: 2px; } .upd .lnk:hover { color: var(--rl-fg); }`;

  const PHONE = `
  @media (max-width: 600px), (pointer: coarse) {
    .keys { display: none; } .btn, .dock .btn { height: 44px; } .btn.i, .dock .btn.i { width: 44px; } .mk:not(.inline)::after { content: ""; position: absolute; inset: -9px; }
  }
  @media (max-width: 600px) {
    .body, textarea, .reply textarea { font-size: 15px; line-height: 22px; } button, .dk-empty { font-size: 14px; }
    .pop, .pop.compose { position: fixed; left: 0 !important; right: 0; top: auto !important; bottom: 0; width: 100%; max-width: none; max-height: 80vh;
           border-radius: 18px 18px 0 0; border-width: 1px 0 0; box-shadow: var(--rl-shadow-sheet); animation: rl-up var(--rl-dur-slide) var(--rl-ease-out); }
    .pop::before { content: ""; align-self: center; width: 40px; height: 4px; margin: 8px 0 2px; border-radius: 2px; background: var(--rl-line); }
    .drawer { top: auto; left: 0; width: 100%; max-height: 80vh; border-left: 0; border-radius: 18px 18px 0 0; animation: rl-up var(--rl-dur-slide) var(--rl-ease-out); }
    .dock { left: 16px; right: 16px; bottom: 16px; } .dock .conn { margin-left: auto; } :host(.drawer-open) .dock { right: 16px; }
    .toast, .upd { left: 16px; right: 16px; bottom: 80px; width: auto; } :host(.drawer-open) .upd { right: 16px; }
  }
  @media (forced-colors: active) {
    .mk, .pop, .dock, .drawer, .chip, .toast, .upd, .dk-upd { forced-color-adjust: none; background: Canvas; color: CanvasText; border: 1px solid CanvasText; box-shadow: none; }
    .mk.answered { background: Highlight; color: HighlightText; }
  }
  @media (prefers-reduced-motion: reduce) { .dots i, .mk.fresh, .mk.wait::before { animation: none; opacity: 1; } .mk.fresh { box-shadow: 0 0 0 4px var(--rl-agent-tint-focus); } }`;

  window.Redline = Object.assign(window.Redline || {}, { overlayStyle: TOKENS + BASE + MARKS + POP + TAGS + DOCK + PHONE });
})();
