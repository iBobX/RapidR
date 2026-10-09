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
  // (the page is Studio's window: maximized, it fills the page — unless
  // ?window=normal, the tests' 1280 x 800, the desktop's)
  const args = ["--home", "."];
  if (params.get("window") !== "normal") args.push("--maximized");
  if (params.get("theme")) args.push("--theme", params.get("theme"));
  if (params.has("fresh")) args.push("--fresh");
  if (params.get("do")) args.push("--do", params.get("do"));
  if (params.get("open")) args.push(params.get("open"));
  window.RAPIDR_ARGS = args;
  // (Studio is the whole page: F5, F6 and Ctrl+Tab are its keys)
  window.RAPIDR_APP_KEYS = true;
  window.RAPIDR_FONTS = new URL("runtime/fonts/", location.href).href;

  const [rt, bytes] = await Promise.all([
    import("./runtime/rapidrintr.js").then(async (m) => { await m.default(); return m; }),
    fetch("studio.rrbc").then((r) => {
      if (!r.ok) throw new Error(`studio.rrbc: ${r.status}`);
      return r.arrayBuffer();
    }),
  ]);
  window.rr = rt;
  if (window.RAPIDR_STUDIO_TEST) {
    rt.rapidr_set_test_env(window.RAPIDR_STUDIO_TEST);
    // (a test's files in the page's store, as a folder picker leaves them)
    for (const f of window.RAPIDR_STUDIO_TEST_FILES || []) rt.rapidr_store_file(f.path, new TextEncoder().encode(f.text), undefined);
  } else {
    // (what Studio wrote before — new projects, files saved — back in the
    // page's store; and from now on each write kept too)
    await restoreFiles(rt);
    window.RAPIDR_FILE_SINK = keepFile;
  }
  say("");
  rt.rapidr_run_bc(new Uint8Array(bytes));
  // (File > Exit: Studio's program ended — the page says so, and a button
  // opens Studio again, as a reload does)
  const watch = setInterval(() => {
    if (!rt.rapidr_main_done()) return;
    clearInterval(watch);
    const closed = document.getElementById("studio-closed");
    if (!closed) return;
    closed.classList.add("shown");
    const again = document.getElementById("studio-reopen");
    again.addEventListener("click", () => {
      // (without the commands a link ran at the start: ?do=)
      const u = new URL(location.href);
      u.searchParams.delete("do");
      location.href = u.toString();
    });
    again.focus();
  }, 400);
  // (cold start, page load to the shell running: docs/ide-plan.md §6.2)
  window.RAPIDR_STUDIO_STARTED = performance.now() - t0;
}

// ---- Studio's files in the browser (the origin private file system) ------
// Every file Studio writes (RAPIDR_FILE_SINK, from the runtime's file
// writes) is kept in OPFS under rapidr-studio/, its path encoded as one
// name; at the next visit they go back into the page's store before the
// shell starts, so projects made or saved in the browser stay. Files the
// user opened from the computer are written back to those files as well
// (the runtime's pickers); OPFS keeps a copy. Without OPFS (an old
// browser, a private window) files last for the visit.

const STORE_DIR = "rapidr-studio";

async function storeDir() {
  if (!navigator.storage || !navigator.storage.getDirectory) return null;
  try {
    const root = await navigator.storage.getDirectory();
    return await root.getDirectoryHandle(STORE_DIR, { create: true });
  } catch {
    return null;
  }
}

async function restoreFiles(rt) {
  const dir = await storeDir();
  if (!dir) return 0;
  let n = 0;
  for await (const [name, h] of dir.entries()) {
    if (h.kind !== "file") continue;
    try {
      const bytes = new Uint8Array(await (await h.getFile()).arrayBuffer());
      rt.rapidr_store_file(decodeURIComponent(name), bytes, undefined);
      n++;
    } catch (e) {
      console.warn("[studio] can't restore", name, e);
    }
  }
  return n;
}

// (writes one after the other: the last of a file wins)
let writing = Promise.resolve();
function keepFile(path, bytes) {
  const copy = new Uint8Array(bytes);
  writing = writing.then(async () => {
    const dir = await storeDir();
    if (!dir) return;
    try {
      const h = await dir.getFileHandle(encodeURIComponent(path), { create: true });
      const w = await h.createWritable();
      await w.write(copy);
      await w.close();
    } catch (e) {
      console.warn("[studio] can't keep", path, e);
    }
  });
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

const run = { box: null, frame: null, port: null, ready: false, queue: [], generation: 0, program: "", files: new Map(), fileTokens: 0 };

// The program's Open / Save dialogs: its frame's opaque origin may not show
// the browser's file pickers (showOpenFilePicker / showSaveFilePicker), so
// this page shows them for it, inside the gesture of the user's click in
// the frame (crates/rapidr-runtime-web/src/file_picker_web.rs,
// RAPIDR_FILE_HOST). The frame gets the files picked and a token for each;
// it writes only through a token it was given.
async function frameFiles(d) {
  const port = run.port;
  const reply = (msg, transfer = []) => port && port.postMessage({ __rapidr_files_reply: { id: d.id, ...msg } }, transfer);
  // (only the options a program's Filter and FileName make)
  const o = d.opts && typeof d.opts === "object" ? d.opts : {};
  const opts = {};
  if (Array.isArray(o.types)) opts.types = o.types;
  if (typeof o.excludeAcceptAllOption === "boolean") opts.excludeAcceptAllOption = o.excludeAcceptAllOption;
  if (typeof o.id === "string") opts.id = o.id;
  if (typeof o.suggestedName === "string") opts.suggestedName = o.suggestedName;
  if (typeof o.multiple === "boolean") opts.multiple = o.multiple;
  try {
    if (d.op === "open") {
      const handles = await window.showOpenFilePicker(opts);
      const files = [];
      for (const h of handles) {
        const f = await h.getFile();
        const token = ++run.fileTokens;
        run.files.set(token, h);
        files.push({ name: f.name, bytes: await f.arrayBuffer(), token });
      }
      reply({ ok: true, value: files }, files.map((f) => f.bytes));
    } else if (d.op === "save") {
      const h = await window.showSaveFilePicker(opts);
      const token = ++run.fileTokens;
      run.files.set(token, h);
      reply({ ok: true, value: { name: h.name, token } });
    } else if (d.op === "write") {
      const h = run.files.get(d.token);
      if (!h) throw Object.assign(new Error("not a file the user picked"), { name: "NotFoundError" });
      const w = await h.createWritable();
      await w.write(d.bytes);
      await w.close();
      reply({ ok: true });
    }
  } catch (e) {
    reply({ ok: false, error: { name: (e && e.name) || "Error", message: (e && e.message) || String(e) } });
  } finally {
    // (the browser gives the focus back to the page that showed the picker:
    // back to the program, whose window had it)
    if (d.op !== "write" && run.frame) run.frame.focus();
  }
}

// What the program keeps in localStorage (RWEBSTORAGE): its frame has no
// storage of its own (an opaque origin), so it gets a copy at each run and
// sends back each change, kept here per program under a key of its own
// (never Studio's keys), capped so Studio's own storage stays safe.
const APP_STORAGE_LIMIT = 1024 * 1024;
const appStorageKey = (program) => `rapidr-app-storage:${program}`;
function loadAppStorage(program) {
  try { return JSON.parse(localStorage.getItem(appStorageKey(program)) || "{}") || {}; } catch { return {}; }
}
function applyAppStorageOp(program, { op, key, value }) {
  const data = loadAppStorage(program);
  if (op === "set" && typeof key === "string") data[key] = String(value);
  else if (op === "remove" && typeof key === "string") delete data[key];
  else if (op === "clear") for (const k of Object.keys(data)) delete data[k];
  else return;
  const json = JSON.stringify(data);
  if (json.length > APP_STORAGE_LIMIT) {
    window.rr.studio_session_incoming(JSON.stringify({ type: "output", stream: "stderr", text: "[storage] the program's storage is full (1 MB)\n" }));
    return;
  }
  try { localStorage.setItem(appStorageKey(program), json); } catch { /* no storage: this visit only */ }
}

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
    // The sandboxed program frame controls `file`; it must name a font file
    // inside runtime/fonts/, never reach out of it (SEC-14). Allow only a
    // plain file name (letters, digits, `_`, `-`, `.`) with no path separator
    // and no `..`, so this bridge can't become a same-origin read primitive
    // for the frame.
    if (typeof file !== "string" || !/^[\w.-]+$/.test(file) || file.includes("..")) {
      if (run.port) run.port.postMessage({ __rapidr_font_reply: { id, bytes: null } });
      return;
    }
    fetch(new URL("runtime/fonts/" + file, location.href))
      .then((r) => (r.ok ? r.arrayBuffer() : null))
      .then((bytes) => run.port && run.port.postMessage({ __rapidr_font_reply: { id, bytes } }, bytes ? [bytes] : []))
      .catch(() => run.port && run.port.postMessage({ __rapidr_font_reply: { id, bytes: null } }));
  } else if (d.__rapidr_windows && typeof d.__rapidr_windows === "object") {
    clipToWindows(d.__rapidr_windows);
  } else if (d.__rapidr_files) {
    frameFiles(d.__rapidr_files);
  } else if (d.__rapidr_storage) {
    applyAppStorageOp(run.program, d.__rapidr_storage);
  } else if (d.__rapidr_console) {
    const { level, text } = d.__rapidr_console;
    window.rr.studio_session_incoming(JSON.stringify({ type: "output", stream: level === "error" ? "stderr" : "stdout", text: text + "\n" }));
  }
}

// The program's windows over the whole page: its frame covers Studio's
// viewport and is clipped to the windows' rectangles (the frame reports
// them). clip-path also decides where the pointer lands, so a click outside
// the program's windows reaches Studio; while a button is held in the frame
// (a window dragged or resized) the whole frame shows, then it's clipped
// again. Rectangles are all the frame says about its windows; they're
// checked and kept inside the page.
// (no windows: one transparent pixel in the corner, not none — a frame
// clipped away entirely counts as hidden, and the browser stops its
// animation frames, so its first window would never be drawn)
const NO_WINDOWS = "path('M0 0h1v1h-1Z')";

function clipToWindows(w) {
  if (!run.frame) return;
  if (typeof w.held === "boolean") run.held = w.held;
  if (Array.isArray(w.rects)) {
    const vw = window.innerWidth, vh = window.innerHeight;
    run.rects = w.rects
      .slice(0, 256)
      .filter((r) => Array.isArray(r) && r.length === 4 && r.every(Number.isFinite))
      .map(([x, y, rw, rh]) => {
        const l = Math.max(0, Math.min(vw, x)), t = Math.max(0, Math.min(vh, y));
        return [l, t, Math.max(0, Math.min(vw, x + rw) - l), Math.max(0, Math.min(vh, y + rh) - t)];
      })
      .filter((r) => r[2] > 0 && r[3] > 0);
    // (for the tests: where the program's windows are)
    window.RAPIDR_STUDIO_RUN_RECTS = run.rects;
  }
  const rects = run.rects || [];
  let clip = NO_WINDOWS;
  if (run.held) clip = "none";
  else if (rects.length) clip = "path('" + rects.map(([x, y, rw, rh]) => "M" + x + " " + y + "h" + rw + "v" + rh + "h" + -rw + "Z").join("") + "')";
  run.frame.style.clipPath = clip;
}

// Studio's menus and pop-ups (a window's pop-up layer shown) stay over the
// program's windows, as the system's menus do on the desktop: the frame
// goes under Studio while one is open.
function studioPopupOpen() {
  for (const p of document.querySelectorAll("body > .rr-kwin > .rr-kpopups")) {
    if (p.style.display !== "none") return true;
  }
  return false;
}
function followPopups() {
  if (run.box) run.box.classList.toggle("under", studioPopupOpen());
}
new MutationObserver(followPopups).observe(document.body, { subtree: true, attributes: true, attributeFilter: ["style"] });

function closeFrame() {
  run.generation++;
  if (run.port) run.port.close();
  if (run.box) run.box.remove();
  run.files.clear();
  Object.assign(run, { box: null, frame: null, port: null, ready: false, queue: [], rects: [], held: false });
  window.RAPIDR_STUDIO_RUN_RECTS = [];
}

window.RAPIDR_STUDIO_HOST = {
  run(bytes, program, args, theme, assets) {
    closeFrame();
    const generation = run.generation;
    const box = document.createElement("div");
    box.id = "studio-run";
    box.setAttribute("aria-label", "Running: " + program);
    const frame = document.createElement("iframe");
    frame.setAttribute("sandbox", "allow-scripts allow-modals allow-downloads");
    frame.setAttribute("title", "The running program");
    frame.src = "run.html";
    // (clipped to nothing until the program's windows say where they are)
    frame.style.clipPath = NO_WINDOWS;
    box.appendChild(frame);
    document.body.appendChild(box);
    Object.assign(run, { box, frame, program });
    const hello = async (e) => {
      if (e.source !== frame.contentWindow || !e.data || !e.data.__rapidr_hello) return;
      window.removeEventListener("message", hello);
      if (generation !== run.generation) return;
      const files = await loadRuntimeFiles();
      const channel = new MessageChannel();
      run.port = channel.port1;
      run.port.onmessage = frameMessage;
      frame.contentWindow.postMessage({
        __rapidr_boot: {
          ...files, session: { bytes, program }, args: Array.from(args || []),
          theme: theme || "",
          // (the data files beside the program — a CSV it loads — as the
          // files of its folder: name → data URL)
          assets: assets && typeof assets === "object" ? assets : {},
          storage: loadAppStorage(program),
          // (this page shows the browser's pickers for the frame: frameFiles)
          filePickers: typeof window.showOpenFilePicker === "function" && typeof window.showSaveFilePicker === "function",
        },
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
