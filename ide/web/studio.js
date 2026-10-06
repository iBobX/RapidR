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
  if (params.has("fresh")) args.push("--fresh");
  if (params.get("do")) args.push("--do", params.get("do"));
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

// ---- the program under development (RPROGRAMSESSION's web host) ----------
// The runtime's studio_web.rs calls run / send / stop. The program runs in
// run.html, an opaque-origin sandboxed frame (it can't reach the IDE's
// page, storage or files), speaking the session protocol over a private
// MessagePort; its events go back to the runtime (studio_session_incoming).

let runtimeFiles = null;
async function loadRuntimeFiles() {
  if (!runtimeFiles) {
    const [js, wasm] = await Promise.all([
      fetch("runtime/rapidrintr.js").then((r) => r.text()),
      fetch("runtime/rapidrintr_bg.wasm").then((r) => r.arrayBuffer()),
    ]);
    runtimeFiles = { runtimeJs: js, runtimeWasm: wasm };
  }
  return runtimeFiles;
}

const run = { box: null, frame: null, port: null, ready: false, queue: [], generation: 0 };

function frameMessage(e) {
  const d = e.data || {};
  if (typeof d.__rapidr_session === "string") {
    let event = null;
    try { event = JSON.parse(d.__rapidr_session); } catch { /* text */ }
    if (event && event.type === "ready" && !run.ready) {
      run.ready = true;
      for (const j of run.queue.splice(0)) run.port.postMessage({ __rapidr_session: j });
    }
    window.rr.studio_session_incoming(d.__rapidr_session);
    if (event && event.type === "exited") closeFrame();
  } else if (d.__rapidr_font) {
    const { id, file } = d.__rapidr_font;
    fetch(new URL("runtime/fonts/" + file, location.href))
      .then((r) => (r.ok ? r.arrayBuffer() : null))
      .then((bytes) => run.port && run.port.postMessage({ __rapidr_font_reply: { id, bytes } }, bytes ? [bytes] : []))
      .catch(() => run.port && run.port.postMessage({ __rapidr_font_reply: { id, bytes: null } }));
  } else if (d.__rapidr_console) {
    const { level, text } = d.__rapidr_console;
    window.rr.studio_session_incoming(JSON.stringify({ type: "output", stream: level === "error" ? "stderr" : "stdout", text: text + "\n" }));
  }
}

function closeFrame() {
  run.generation++;
  if (run.port) run.port.close();
  if (run.box) run.box.remove();
  Object.assign(run, { box: null, frame: null, port: null, ready: false, queue: [] });
}

window.RAPIDR_STUDIO_HOST = {
  run(bytes, program, args) {
    closeFrame();
    const generation = run.generation;
    const box = document.createElement("div");
    box.id = "studio-run";
    box.setAttribute("aria-label", "Running: " + program);
    const frame = document.createElement("iframe");
    frame.setAttribute("sandbox", "allow-scripts allow-modals allow-downloads");
    frame.setAttribute("title", "The running program");
    frame.src = "run.html";
    box.appendChild(frame);
    document.body.appendChild(box);
    Object.assign(run, { box, frame });
    const hello = async (e) => {
      if (e.source !== frame.contentWindow || !e.data || !e.data.__rapidr_hello) return;
      window.removeEventListener("message", hello);
      if (generation !== run.generation) return;
      const files = await loadRuntimeFiles();
      const channel = new MessageChannel();
      run.port = channel.port1;
      run.port.onmessage = frameMessage;
      frame.contentWindow.postMessage({
        __rapidr_boot: { ...files, session: { bytes, program }, args: Array.from(args || []), storage: {}, filePickers: false },
      }, "*", [channel.port2]);
    };
    window.addEventListener("message", hello);
  },
  send(json) {
    if (run.port && run.ready) run.port.postMessage({ __rapidr_session: json });
    else run.queue.push(json);
  },
  stop() {
    const had = !!run.box;
    closeFrame();
    if (had && window.rr) window.rr.studio_session_ended(0);
  },
};

main().catch((e) => {
  console.error(e);
  say("RapidR Studio could not start: " + (e && e.message ? e.message : e));
});
