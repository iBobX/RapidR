// The spike's page glue (docs/web-host-plan.md): the DOM side of
// crates/rapidr-ui-host-web. Rust does the form (the kernel's tree, input
// routing, painting, rasterizing, the accessibility tree and what ARIA says
// about it); this file only passes the browser's events in, puts frames on
// the canvas when the kernel says one is due, keeps the ARIA mirror's
// elements in step with the description Rust gives, and moves the DOM focus
// where the kernel's focus is.
//
// window.spike is what tests/web_host_spike.mjs reads: the forms, their
// pixels, accessibility trees and the timings measured here.

const params = new URLSearchParams(location.search);
const gpuWanted = params.get("gpu") === "1";
const steady = params.get("steady") === "1";
// (?pkg=<dir under target/>: another build of the spike, e.g.
// web-host-spike-scalar, without wasm SIMD; tools/build_web_host_spike.sh)
const pkgDir = /^[\w-]+$/.test(params.get("pkg") || "") ? params.get("pkg") : gpuWanted ? "web-host-spike-gpu" : "web-host-spike";
const pkg = `../target/${pkgDir}/rapidr_ui_host_web.js`;
const mac = /Mac|iPhone|iPad/.test(navigator.platform);
const status = document.getElementById("status");
const logEl = document.getElementById("log");

const metrics = { start: performance.now(), load: {}, frames: {}, latency: [], handler: [] };
const spike = { metrics, wins: [], ready: false, errors: [] };
window.spike = spike;
window.addEventListener("error", (e) => spike.errors.push(String(e.message || e)));
window.addEventListener("unhandledrejection", (e) => spike.errors.push(String(e.reason)));

const t0 = performance.now();
const wasm = await import(pkg);
await wasm.default();
metrics.load.wasm = performance.now() - t0;
metrics.load.fonts = wasm.warm_up();

let dpr = window.devicePixelRatio || 1;
let active = null;
let clipboardText = null;

function log(lines) {
  if (!lines.length) return;
  logEl.textContent += lines.join("\n") + "\n";
  logEl.scrollTop = logEl.scrollHeight;
}

// ------------------------------------------------------------ windows --

function makeWindow(name) {
  const t = performance.now();
  const form = new wasm.SpikeForm(name, dpr, mac);
  if (steady) form.set_blinks(false);
  const [w, h] = form.size();
  const win = document.createElement("div");
  win.className = "rr-window";
  const title = document.createElement("div");
  title.className = "rr-title";
  title.textContent = form.caption();
  title.setAttribute("aria-hidden", "true");
  const client = document.createElement("div");
  client.className = "rr-client";
  client.style.width = w + "px";
  client.style.height = h + "px";
  const canvas = document.createElement("canvas");
  canvas.className = "rr-surface";
  canvas.dataset.form = name;
  // (the picture is the mirror's to describe)
  canvas.setAttribute("aria-hidden", "true");
  const mirror = document.createElement("div");
  mirror.className = "rr-a11y";
  client.append(canvas, mirror);
  win.append(title, client);
  document.getElementById("desk").append(win);
  const ww = { name, form, win, client, canvas, mirror, ctx: null, gpu: null, nodes: new Map(), pending: [], composing: false, size: [w, h] };
  sizeCanvas(ww);
  ww.ctx = canvas.getContext("2d", { alpha: false });
  // A 2D canvas can lose its (GPU) backing and come back blank — the GPU
  // process restarting, memory pressure: the frame is drawn again.
  canvas.addEventListener("contextlost", () => { metrics.contextLost = (metrics.contextLost || 0) + 1; });
  canvas.addEventListener("contextrestored", () => {
    metrics.contextRestored = (metrics.contextRestored || 0) + 1;
    ww.form.render(ww.ctx, true);
  });
  metrics.load[`build_${name}`] = performance.now() - t;
  metrics.frames[name] = [];
  wireCanvas(ww);
  wireMirror(ww);
  return ww;
}

function sizeCanvas(w) {
  const [cw, ch] = w.form.size();
  w.size = [cw, ch];
  w.client.style.width = cw + "px";
  w.client.style.height = ch + "px";
  w.canvas.style.width = cw + "px";
  w.canvas.style.height = ch + "px";
  // (device pixels: as the desktop host's device_size rounds them)
  w.canvas.width = Math.max(1, Math.round(cw * dpr));
  w.canvas.height = Math.max(1, Math.round(ch * dpr));
}

// ------------------------------------------------------------- frames --

let rafPending = false;
let wakeTimer = null;
function invalidate() {
  if (rafPending) return;
  rafPending = true;
  requestAnimationFrame(frame);
}

function frame() {
  rafPending = false;
  for (const w of spike.wins) {
    if (!w.form.dirty()) continue;
    const t = performance.now();
    const times = w.gpu ? JSON.parse(w.gpu.render(w.form)) : JSON.parse(w.form.render(w.ctx, false) || "null");
    if (!times) continue;
    const a = performance.now();
    syncMirror(w);
    times.a11y = performance.now() - a;
    times.total = performance.now() - t;
    metrics.frames[w.name].push(times);
    if (metrics.firstFrame === undefined) metrics.firstFrame = performance.now();
    const now = performance.now();
    for (const ts of w.pending) metrics.latency.push(now - ts);
    w.pending = [];
    log(w.form.take_log());
  }
  scheduleWake();
}

// (the kernel's deadlines: the caret's blink, a held scroll bar arrow)
function scheduleWake() {
  if (wakeTimer) clearTimeout(wakeTimer);
  wakeTimer = null;
  let next = Infinity;
  for (const w of spike.wins) {
    const ms = w.form.wake_in_ms();
    if (ms >= 0) next = Math.min(next, ms);
  }
  if (next === Infinity) return;
  wakeTimer = setTimeout(() => {
    wakeTimer = null;
    for (const w of spike.wins) w.form.tick();
    invalidate();
  }, Math.max(1, next));
}

// ---------------------------------------------------------- the mirror --

function syncMirror(w) {
  const specs = JSON.parse(w.form.aria());
  const seen = new Set();
  let focused = null;
  for (const s of specs) {
    seen.add(s.id);
    let el = w.nodes.get(s.id);
    if (!el || el.tagName.toLowerCase() !== s.tag) {
      el?.remove();
      el = document.createElement(s.tag);
      el.id = `rrn-${w.name}-${s.id}`;
      el.dataset.node = s.id;
      if (s.tag === "input") el.type = "text";
      if (s.tag === "input" || s.tag === "textarea") {
        el.spellcheck = false;
        el.autocomplete = "off";
        el.setAttribute("autocapitalize", "off");
        wireField(w, el);
      }
      // A screen reader's click (no pointer reaches the mirror).
      el.addEventListener("click", (e) => {
        e.stopPropagation();
        if (w.form.access_action(s.id, "click")) afterInput(w, e);
      });
      w.nodes.set(s.id, el);
    }
    const parent = s.parent ? w.nodes.get(s.parent) : w.mirror;
    if (el.parentNode !== parent) parent.append(el);
    // (attributes: the role's, the name's, the states')
    const want = new Map(s.attrs);
    for (const a of [...el.attributes]) {
      if ((a.name === "role" || a.name.startsWith("aria-")) && !want.has(a.name)) el.removeAttribute(a.name);
    }
    for (const [k, v] of want) if (el.getAttribute(k) !== v) el.setAttribute(k, v);
    if (s.focusable) el.tabIndex = -1;
    else el.removeAttribute("tabindex");
    if (s.tag === "div" && el.childElementCount === 0 && el.textContent !== s.text) el.textContent = s.text;
    if ((s.tag === "input" || s.tag === "textarea") && !w.composing && el.value !== s.value) el.value = s.value;
    const [x, y, bw, bh] = s.bounds;
    const st = el.style;
    const px = (v) => v + "px";
    if (st.left !== px(x)) st.left = px(x);
    if (st.top !== px(y)) st.top = px(y);
    if (st.width !== px(bw)) st.width = px(bw);
    if (st.height !== px(bh)) st.height = px(bh);
    if (s.focused) focused = el;
  }
  for (const [id, el] of w.nodes) {
    if (!seen.has(id)) {
      el.remove();
      w.nodes.delete(id);
    }
  }
  w.focusEl = focused;
  syncFocus(w);
}

// The DOM focus where the kernel's is (only for the window in use, so the
// page's own controls keep theirs); a text field's selection as the
// kernel's, so an input method starts where the kernel's caret is.
function syncFocus(w) {
  if (active !== w) return;
  const target = w.focusEl || w.nodes.values().next().value;
  if (!target) return;
  if (document.activeElement !== target) {
    w.focusing = true;
    target.focus({ preventScroll: true });
    w.focusing = false;
  }
  if ((target.tagName === "INPUT" || target.tagName === "TEXTAREA") && !w.composing) {
    const f = JSON.parse(w.form.focus_info());
    if (target.value !== f.text) target.value = f.text;
    if (target.selectionStart !== f.start || target.selectionEnd !== f.end) target.setSelectionRange(f.start, f.end);
  }
}

// ------------------------------------------------------------- input --

function afterInput(w, e) {
  if (e && e.timeStamp) w.pending.push(e.timeStamp);
  log(w.form.take_log());
  invalidate();
}

function activate(w) {
  active = w;
}

const utf8 = new TextEncoder();
const mods = (e) => [e.shiftKey, e.ctrlKey, e.altKey, e.metaKey];

function wireCanvas(w) {
  const c = w.canvas;
  const at = (e) => {
    const r = c.getBoundingClientRect();
    return [e.clientX - r.left, e.clientY - r.top];
  };
  c.addEventListener("pointerdown", (e) => {
    const h = performance.now();
    // (keeps the focus where the kernel will put it, not on the page)
    e.preventDefault();
    activate(w);
    c.setPointerCapture(e.pointerId);
    w.form.pointer_down(...at(e), e.button, ...mods(e));
    // (the focus moves now, inside the user's gesture: a phone shows its
    // keyboard only then)
    syncMirror(w);
    afterInput(w, e);
    metrics.handler.push(performance.now() - h);
  });
  c.addEventListener("pointermove", (e) => {
    w.form.pointer_move(...at(e), ...mods(e));
    if (w.form.dirty()) afterInput(w, e);
  });
  c.addEventListener("pointerup", (e) => {
    const h = performance.now();
    const copied = w.form.pointer_up(...at(e), e.button, ...mods(e), clipboardText ?? undefined);
    if (copied != null) {
      clipboardText = copied;
      navigator.clipboard?.writeText(copied).catch(() => {});
    }
    syncMirror(w);
    afterInput(w, e);
    metrics.handler.push(performance.now() - h);
  });
  c.addEventListener("pointerleave", (e) => {
    if (!c.hasPointerCapture(e.pointerId)) {
      w.form.pointer_leave();
      afterInput(w, e);
    }
  });
  c.addEventListener("contextmenu", (e) => e.preventDefault());
  c.addEventListener("wheel", (e) => {
    e.preventDefault();
    // (notches, positive down: a line mode's 3 lines, a pixel mode's 48
    // logical pixels — the desktop host's touchpad rule)
    const k = e.deltaMode === 1 ? 1 / 3 : e.deltaMode === 2 ? 1 : 1 / 48;
    w.form.wheel(...at(e), e.deltaX * k, e.deltaY * k, ...mods(e));
    afterInput(w, e);
  }, { passive: false });
}

function wireMirror(w) {
  // Keys reach the mirror's focused element; they all go to the kernel.
  w.mirror.addEventListener("keydown", (e) => {
    const h = performance.now();
    activate(w);
    // (an input method's: its composition events carry the text)
    if (e.isComposing || e.keyCode === 229 || e.key === "Dead" || e.key === "Process") return;
    const cmd = mac ? e.metaKey : e.ctrlKey;
    const k = e.key.toLowerCase();
    // (Copy, Cut, Paste: the clipboard events below, which carry the
    // clipboard's text without asking for a permission)
    if (cmd && (k === "c" || k === "x" || k === "v")) return;
    // (the browser keeps its own: reload, devtools, zoom, tabs)
    if (cmd && ["r", "t", "w", "l", "n", "+", "-", "=", "0"].includes(k)) return;
    if (e.key === "F5" || e.key === "F12") return;
    const printable = [...e.key].length === 1 && !e.ctrlKey && !e.metaKey;
    if (w.form.key_down(e.key, e.code, printable ? e.key : "", ...mods(e))) e.preventDefault();
    syncMirror(w);
    afterInput(w, e);
    metrics.handler.push(performance.now() - h);
  });
  w.mirror.addEventListener("keyup", (e) => {
    if (e.isComposing || e.keyCode === 229) return;
    w.form.key_up(e.key, e.code, ...mods(e));
    afterInput(w, e);
  });
  w.mirror.addEventListener("copy", (e) => clip(w, e, "c"));
  w.mirror.addEventListener("cut", (e) => clip(w, e, "x"));
  w.mirror.addEventListener("paste", (e) => {
    e.preventDefault();
    const text = e.clipboardData?.getData("text/plain") ?? "";
    clipboardText = text;
    w.form.clipboard_key("v", text);
    syncMirror(w);
    afterInput(w, e);
  });
  // A screen reader moving the focus onto a node: the kernel's focus.
  w.mirror.addEventListener("focusin", (e) => {
    if (w.focusing) return;
    activate(w);
    const id = e.target?.dataset?.node;
    if (id && e.target !== w.focusEl && w.form.access_action(id, "focus")) afterInput(w, e);
  });
}

function clip(w, e, letter) {
  const text = w.form.clipboard_key(letter, clipboardText ?? undefined);
  if (text != null) {
    clipboardText = text;
    e.clipboardData.setData("text/plain", text);
  }
  e.preventDefault();
  syncMirror(w);
  afterInput(w, e);
}

// The text fields (an edit's, a memo's mirror element): what input methods,
// mobile keyboards, dictation and the emoji picker type arrives here.
function wireField(w, el) {
  el.addEventListener("compositionstart", () => { w.composing = true; });
  el.addEventListener("compositionupdate", (e) => {
    const n = utf8.encode(e.data || "").length;
    w.form.ime_preedit(e.data || "", n, n);
    afterInput(w, e);
  });
  el.addEventListener("compositionend", (e) => {
    w.composing = false;
    w.form.ime_commit(e.data || "");
    syncMirror(w);
    afterInput(w, e);
  });
  el.addEventListener("beforeinput", (e) => {
    if (e.isComposing || w.composing) return;
    const key = (name) => {
      w.form.key_down(name, name, "", false, false, false, false);
      w.form.key_up(name, name, false, false, false, false);
    };
    switch (e.inputType) {
      case "insertText":
      case "insertReplacementText":
        w.form.ime_commit(e.data ?? e.dataTransfer?.getData("text/plain") ?? "");
        break;
      case "deleteContentBackward": key("Backspace"); break;
      case "deleteContentForward": key("Delete"); break;
      case "insertLineBreak":
      case "insertParagraph": key("Enter"); break;
      default: return;
    }
    e.preventDefault();
    syncMirror(w);
    afterInput(w, e);
  });
}

// ------------------------------------------------------------ the page --

// The screen's scale changed (another monitor, the browser's zoom).
function watchScale() {
  const mq = matchMedia(`(resolution: ${dpr}dppx)`);
  mq.addEventListener("change", () => {
    dpr = window.devicePixelRatio || 1;
    for (const w of spike.wins) {
      w.form.set_scale(dpr);
      sizeCanvas(w);
    }
    invalidate();
    watchScale();
  }, { once: true });
}

const names = (params.get("forms") || wasm.form_names().join(",")).split(",").filter(Boolean);
for (const n of names) spike.wins.push(makeWindow(n));
watchScale();

if (gpuWanted) {
  if (!navigator.gpu || !wasm.SpikeGpu) {
    metrics.gpu = "no WebGPU in this browser (or the build lacks feature gpu): the CPU renderer draws";
  } else {
    for (const w of spike.wins) {
      try {
        // (a WebGPU canvas can't be a 2D one too: a fresh canvas)
        const c = w.canvas.cloneNode();
        w.canvas.replaceWith(c);
        w.canvas = c;
        wireCanvas(w);
        w.gpu = await wasm.SpikeGpu.create(c, c.width, c.height);
        metrics.gpu = w.gpu.adapter();
      } catch (err) {
        metrics.gpu = `WebGPU failed (${err}): the CPU renderer draws`;
        w.gpu = null;
      }
    }
  }
}

// The first frame, at once (not a frame later).
for (const w of spike.wins) {
  const t = performance.now();
  const times = w.gpu ? JSON.parse(w.gpu.render(w.form)) : JSON.parse(w.form.render(w.ctx, true));
  times.first = true;
  times.total = performance.now() - t;
  metrics.frames[w.name].push(times);
  syncMirror(w);
}
metrics.firstFrame = performance.now();
activate(spike.wins[0]);
syncFocus(spike.wins[0]);
scheduleWake();

// ---------------------------------------------------- for the test --

spike.win = (name) => spike.wins.find((w) => w.name === name);
spike.pixels = (name) => Array.from(spike.win(name).form.pixels());
spike.canvasPixels = (name) => {
  const w = spike.win(name);
  return Array.from(w.ctx.getImageData(0, 0, w.canvas.width, w.canvas.height).data);
};
spike.access = (name) => spike.win(name).form.access_json();
spike.dump = (name) => spike.win(name).form.dump();
spike.focusInfo = (name) => JSON.parse(spike.win(name).form.focus_info());
/// `n` frames of form `name` drawn again (forced), their timings.
spike.bench = (name, n) => {
  const w = spike.win(name);
  const out = [];
  for (let i = 0; i < n; i++) {
    const t = performance.now();
    const times = w.gpu ? JSON.parse(w.gpu.render(w.form)) : JSON.parse(w.form.render(w.ctx, true));
    times.total = performance.now() - t;
    out.push(times);
  }
  return out;
};
spike.idle = () => new Promise((res) => requestAnimationFrame(() => requestAnimationFrame(res)));

spike.ready = true;
status.textContent = `dpr ${dpr} · ${spike.wins.map((w) => w.gpu ? "WebGPU" : "vello_cpu").join(", ")} · wasm ${metrics.load.wasm.toFixed(1)} ms · fonts ${metrics.load.fonts.toFixed(1)} ms · first frame at ${metrics.firstFrame.toFixed(1)} ms${metrics.gpu ? " · " + metrics.gpu : ""}`;
