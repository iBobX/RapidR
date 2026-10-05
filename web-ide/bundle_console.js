// On-page console for exported web bundles (`rapidr bundle-bc` and the IDE's
// Build): shows what the program PRINTs, with RapidQ's CLS / COLOR / LOCATE
// rendered by ansi_screen.js, the same screen the IDE's Output panel uses.
//
// The web runtime passes every printed text to `window.__rapidr_print`
// (crates/rapidr-runtime-web/src/builtins.rs). The panel appears on the first
// PRINT: it fills the page for a console program and docks at the bottom
// when the program also shows forms.

import { AnsiScreen } from "./ansi_screen.js";

export function installConsole() {
  let screen = null;
  let el = null;
  // (docked while a window of the program shows: the UI kernel's windows,
  // `.rr-kwin`, shown or hidden by their style — watched, since a form may
  // show after the last PRINT)
  const shown = () => [...document.querySelectorAll(".rr-kwin")].some((w) => w.style.display !== "none") || document.querySelector(".rr-form") !== null;
  const dock = () => el?.classList.toggle("docked", shown());
  const watched = new WeakSet();
  const watch = () => {
    for (const w of document.querySelectorAll(".rr-kwin")) {
      if (watched.has(w)) continue;
      watched.add(w);
      new MutationObserver(dock).observe(w, { attributes: true, attributeFilter: ["style"] });
    }
    dock();
  };
  new MutationObserver(watch).observe(document.body, { childList: true });
  window.__rapidr_print = (text) => {
    if (!screen) {
      const style = document.createElement("style");
      style.textContent = CONSOLE_CSS;
      document.head.appendChild(style);
      el = document.createElement("pre");
      el.id = "rapidr-console";
      el.setAttribute("role", "log");
      el.setAttribute("aria-label", "Program output");
      document.body.appendChild(el);
      screen = new AnsiScreen(el);
    }
    dock();
    screen.write(String(text));
  };
}

// Styles for the panel, added to the page with it.
export const CONSOLE_CSS = `#rapidr-console { position: fixed; inset: 0; margin: 0; padding: 12px 14px; overflow: auto;
  background: #0c0c0c; color: #cccccc; font: 14px/1.35 ui-monospace, Menlo, Consolas, "Courier New", monospace;
  white-space: pre; z-index: 1000; }
#rapidr-console.docked { top: auto; height: 30vh; border-top: 1px solid #444; }`;
