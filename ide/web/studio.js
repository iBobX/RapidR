// RapidR Studio's web page: loads the web runtime (runtime/), then runs the
// shell's bytecode (studio.rrbc) — the very program `rapidr ide` runs on the
// desktop. Its arguments are the desktop's (`--home`, `--theme`, a file).
//
// Tests set window.RAPIDR_STUDIO_TEST (the test hooks' variables: the GUI
// suites' RAPIDR_CAPTURE, RAPIDR_TEST_EVENTS …) before the page loads.

const status = document.getElementById("studio-status");
const say = (text) => { if (status) status.textContent = text; };
const t0 = performance.now();

async function main() {
  const params = new URLSearchParams(location.search);
  const args = ["--home", "."];
  if (params.get("theme")) args.push("--theme", params.get("theme"));
  if (params.get("open")) args.push(params.get("open"));
  window.RAPIDR_ARGS = args;
  window.RAPIDR_FONTS = new URL("runtime/fonts/", location.href).href;

  const [rt, bytes] = await Promise.all([
    import("./runtime/rapidrintr.js").then(async (m) => { await m.default(); return m; }),
    fetch("studio.rrbc").then((r) => {
      if (!r.ok) throw new Error(`studio.rrbc: ${r.status}`);
      return r.arrayBuffer();
    }),
  ]);
  window.rr = rt;
  if (window.RAPIDR_STUDIO_TEST) rt.rapidr_set_test_env(window.RAPIDR_STUDIO_TEST);
  say("");
  rt.rapidr_run_bc(new Uint8Array(bytes));
  // (cold start, page load to the shell running: docs/ide-plan.md §6.2)
  window.RAPIDR_STUDIO_STARTED = performance.now() - t0;
}

main().catch((e) => {
  console.error(e);
  say("RapidR Studio could not start: " + (e && e.message ? e.message : e));
});
