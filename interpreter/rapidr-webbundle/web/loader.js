// RapidR web bundle loader (interpreter/rapidr-webbundle): boots the web
// runtime (rapidrintr.js / rapidrintr_bg.wasm) and runs the program's
// bytecode, named by the page's <meta name="rapidr-program">. Nothing in
// this file is generated: no project or file name is ever written into a
// script (docs/security-audit.md SEC-16).
import init, { rapidr_run_bc } from "./rapidrintr.js";

const status = document.getElementById("rapidr-status");
const setStatus = (msg) => { if (status) status.textContent = msg; };
const program = document.querySelector('meta[name="rapidr-program"]')?.content || "program.rrbc";

(async function () {
  try {
    // PRINT output on the page (bundle_console.js).
    await import("./bundle_console.js").then((m) => m.installConsole()).catch((e) => console.warn(e));
    setStatus("init wasm…");
    await init();
    setStatus("fetch program…");
    const resp = await fetch("./" + encodeURIComponent(program));
    if (!resp.ok) throw new Error(`fetch ${program}: ${resp.status}`);
    const bytes = new Uint8Array(await resp.arrayBuffer());
    setStatus("running…");
    rapidr_run_bc(bytes);
    setStatus("");
  } catch (e) {
    console.error(e);
    setStatus("error: " + (e && e.message ? e.message : e));
  }
})();
