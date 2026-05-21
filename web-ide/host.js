// RapidR IDE — main controller
//
// This file is being rebuilt in phases (see /memories/session/rapidr-ide-rebuild.md):
//   A — shell wiring               (this commit)
//   B — Monaco editor
//   C — real designer (drag/resize/snap)
//   D — typed property grid
//   E — event stubs / obj+evt dropdowns
//   F — multi-form project model
//   G — Run / Stop / Build wired to project
//   H — polish + Playwright tests

import init, { compile, rapidr_run_bc } from "./runtime/rapidrintr.js";
import { TOOLBOX, TOOLBOX_GROUPS, defaultsFor, isVisibleType } from "./toolbox.js";
import { COMPONENT_REGISTRY } from "./lang-data.js";
import { newProject, addForm, addWidget, removeWidget, serializeForm,
         serializeProject, setProp, findWidget, allWidgets } from "./model.js";
import { createRapidrEditor } from "./monaco-host.js";

// IDE version — single source of truth. Bumped at release time.
export const RAPIDR_IDE_VERSION = "1.0.0";

// One Monaco editor instance per form, keyed by form.id.
const _editors = new Map();

// ─── State ──────────────────────────────────────────────────────

const state = {
  wasmReady: false,
  project: newProject("untitled"),
  activeFormId: null,
  activeView: "designer",       // "designer" | "code" — per active form
  selectedTool: "pointer",
  selection: [],                // array of widget names within active form
};

// ─── DOM helpers ────────────────────────────────────────────────

const $  = (sel, root = document) => root.querySelector(sel);
const $$ = (sel, root = document) => Array.from(root.querySelectorAll(sel));

function setStatus(msg, kind = "") {
  const el = $("#status");
  el.textContent = msg;
  el.className = kind;
}

function logOutput(s) {
  const out = $('.obody[data-tab="output"]');
  out.textContent += s + "\n";
  out.scrollTop = out.scrollHeight;
}

// ─── Errors panel + iframe console capture ─────────────────────
const ERROR_COUNTERS = { error: 0, warn: 0, info: 0 };

function _renderErrorBadges() {
  const tab = $('.otab[data-tab="errors"]');
  if (!tab) return;
  const e = ERROR_COUNTERS.error, w = ERROR_COUNTERS.warn;
  tab.textContent = "Errors" + (e || w ? `  ●${e}/${w}` : "");
  tab.classList.toggle("has-errors", e > 0);
  tab.classList.toggle("has-warns", w > 0 && e === 0);
}

function _stringifyArg(a) {
  if (a == null) return String(a);
  if (typeof a === "string") return a;
  try { return JSON.stringify(a); } catch { return String(a); }
}

function logError(level, ...args) {
  const body = $('.obody[data-tab="errors"]');
  if (!body) return;
  const line = document.createElement("div");
  line.className = "err-line err-" + level;
  const ts = new Date().toLocaleTimeString();
  line.textContent = `[${ts}] [${level.toUpperCase()}] ` + args.map(_stringifyArg).join(" ");
  body.appendChild(line);
  body.scrollTop = body.scrollHeight;
  if (level === "error") ERROR_COUNTERS.error++;
  else if (level === "warn") ERROR_COUNTERS.warn++;
  else ERROR_COUNTERS.info++;
  _renderErrorBadges();
  // First error/warn auto-flips to the Errors tab so the user sees it.
  if (level === "error" && ERROR_COUNTERS.error === 1) {
    $('.otab[data-tab="errors"]')?.click();
  }
}

function clearErrorsPanel() {
  const body = $('.obody[data-tab="errors"]');
  if (body) body.innerHTML = "";
  ERROR_COUNTERS.error = 0;
  ERROR_COUNTERS.warn = 0;
  ERROR_COUNTERS.info = 0;
  _renderErrorBadges();
}

function hookPreviewConsole(iframe) {
  try {
    const w = iframe.contentWindow;
    if (!w || w.__rrConsoleHooked) return;
    w.__rrConsoleHooked = true;
    const orig = {
      log:   w.console.log.bind(w.console),
      info:  w.console.info.bind(w.console),
      warn:  w.console.warn.bind(w.console),
      error: w.console.error.bind(w.console),
    };
    const route = (level, args) => {
      const text = args.map(_stringifyArg).join(" ");
      // Runtime PRINT goes through console.log → also mirror to Output panel.
      if (level === "log" || level === "info") {
        logOutput(text);
      } else {
        logError(level, text);
      }
    };
    w.console.log   = (...a) => { try { route("log",   a); } finally { orig.log(...a); } };
    w.console.info  = (...a) => { try { route("info",  a); } finally { orig.info(...a); } };
    w.console.warn  = (...a) => { try { route("warn",  a); } finally { orig.warn(...a); } };
    w.console.error = (...a) => { try { route("error", a); } finally { orig.error(...a); } };
    // Surface uncaught errors too.
    w.addEventListener("error", (e) => {
      logError("error", `${e.message || "error"} (${e.filename || "?"}:${e.lineno || 0})`);
    });
    w.addEventListener("unhandledrejection", (e) => {
      logError("error", "Unhandled promise rejection: " + (e.reason?.message || e.reason));
    });
  } catch (err) {
    // Cross-origin? Sandbox won't allow it — log once and move on.
    logError("warn", "could not hook preview console: " + err.message);
  }
}

function logImmediate(s) {
  const log = $("#immediate-log");
  const line = document.createElement("div");
  line.textContent = s;
  log.appendChild(line);
  log.scrollTop = log.scrollHeight;
}

function setTheme(theme) {
  // theme: "light" | "dark" | null (auto)
  if (theme === null) {
    document.documentElement.removeAttribute("data-theme");
    try { localStorage.removeItem("rapidr-theme"); } catch {}
  } else {
    document.documentElement.setAttribute("data-theme", theme);
    try { localStorage.setItem("rapidr-theme", theme); } catch {}
  }
  // Re-theme Monaco editors live
  if (window.monaco?.editor) {
    const isDark = theme === "dark" ||
      (theme === null && window.matchMedia?.("(prefers-color-scheme: dark)").matches);
    window.monaco.editor.setTheme(isDark ? "rapidr-dark" : "rapidr-vb6");
  }
  setStatus("theme: " + (theme || "system"));
}

// Restore persisted theme on load
try {
  const saved = localStorage.getItem("rapidr-theme");
  if (saved === "light" || saved === "dark") {
    document.documentElement.setAttribute("data-theme", saved);
  }
} catch {}

// ─── Menu bar ───────────────────────────────────────────────────

function setupMenus() {
  const menus = $$("#menubar .menu");
  let openMenu = null;

  function closeAll() {
    menus.forEach(m => m.classList.remove("open"));
    openMenu = null;
  }

  menus.forEach(m => {
    m.addEventListener("click", (e) => {
      // Click on a dropdown item: let it bubble to the doc-level data-cmd
      // handler and just close the menu.
      if (e.target.closest(".mi")) {
        closeAll();
        return;
      }
      e.stopPropagation();
      const wasOpen = m.classList.contains("open");
      closeAll();
      if (!wasOpen) { m.classList.add("open"); openMenu = m; }
    });
    m.addEventListener("mouseenter", () => {
      if (openMenu && openMenu !== m) {
        closeAll();
        m.classList.add("open");
        openMenu = m;
      }
    });
  });

  document.addEventListener("click", closeAll);

  // Wire menu items + toolbar buttons by data-cmd.
  document.addEventListener("click", (e) => {
    const t = e.target.closest("[data-cmd]");
    if (!t) return;
    const cmd = t.dataset.cmd;
    runCommand(cmd);
  });
}

// ─── Output dock tabs ──────────────────────────────────────────

function setupOutputTabs() {
  $$("#output-tabs .otab").forEach(tab => {
    tab.addEventListener("click", () => {
      const name = tab.dataset.tab;
      $$("#output-tabs .otab").forEach(t => t.classList.toggle("active", t === tab));
      $$(".obody").forEach(b => b.classList.toggle("active", b.dataset.tab === name));
    });
  });
}

// ─── Toolbox ────────────────────────────────────────────────────

function setupToolbox() {
  const body = $("#toolbox-body");
  body.innerHTML = "";

  // Pointer (default arrow tool)
  const ptr = document.createElement("div");
  ptr.className = "tool armed";
  ptr.dataset.tool = "pointer";
  ptr.title = "Pointer (arrow) — select widgets";
  ptr.textContent = "↖";
  body.appendChild(ptr);

  for (const group of TOOLBOX_GROUPS) {
    const hdr = document.createElement("div");
    hdr.className = "tool-group-hdr";
    hdr.textContent = group.name;
    body.appendChild(hdr);
    for (const t of group.items) {
      const el = document.createElement("div");
      el.className = "tool" + (t.visible === false ? " tool-invisible" : "");
      el.dataset.tool = t.type;
      el.title = `${t.label} (${t.type})`;
      el.textContent = t.icon;
      body.appendChild(el);
    }
  }

  body.addEventListener("click", (e) => {
    const tool = e.target.closest(".tool");
    if (!tool) return;
    armTool(tool.dataset.tool);
  });
}

function armTool(toolType) {
  state.selectedTool = toolType;
  $$("#toolbox-body .tool").forEach(t =>
    t.classList.toggle("armed", t.dataset.tool === toolType)
  );
  // Crosshair on the active designer.
  const d = $(".mdi-pane.active .designer");
  if (d) d.classList.toggle("arming", toolType !== "pointer");
}

// ─── Project tree ───────────────────────────────────────────────

function renderProjectTree() {
  const tree = $("#proj-tree");
  tree.innerHTML = "";
  $("#proj-name").textContent = state.project.name;
  $("#project-title").textContent = state.project.name;

  const formsHdr = document.createElement("div");
  formsHdr.className = "tree-group";
  formsHdr.innerHTML = `<span>Forms</span><span class="tree-actions"><button class="tree-btn" title="Add Form" data-cmd="form.new">+</button></span>`;
  tree.appendChild(formsHdr);

  for (const f of state.project.forms) {
    const it = document.createElement("div");
    it.className = "tree-item" + (f.id === state.activeFormId ? " active" : "");
    it.innerHTML = `<span class="ico">▭</span><span class="tree-label">${escapeHtml(f.name)}</span><span class="tree-row-actions"><button class="tree-btn" title="Rename" data-act="rename">✎</button><button class="tree-btn" title="Remove" data-act="remove">×</button></span>`;
    it.addEventListener("click", (e) => {
      const act = e.target.closest("[data-act]")?.dataset?.act;
      if (act === "rename") return renameForm(f.id);
      if (act === "remove") return removeFormPrompt(f.id);
      switchToForm(f.id);
    });
    it.addEventListener("dblclick", () => { switchToForm(f.id); switchView("code"); });
    tree.appendChild(it);
  }

  // Modules group
  const mods = state.project.modules || [];
  const modsHdr = document.createElement("div");
  modsHdr.className = "tree-group";
  modsHdr.innerHTML = `<span>Modules</span><span class="tree-actions"><button class="tree-btn" title="Add Module" data-cmd="module.new">+</button></span>`;
  tree.appendChild(modsHdr);
  for (const m of mods) {
    const it = document.createElement("div");
    it.className = "tree-item";
    it.innerHTML = `<span class="ico">§</span><span class="tree-label">${escapeHtml(m.name)}</span><span class="tree-row-actions"><button class="tree-btn" title="Remove" data-act="mod-remove">×</button></span>`;
    it.addEventListener("click", (e) => {
      const act = e.target.closest("[data-act]")?.dataset?.act;
      if (act === "mod-remove") {
        showDialog("Remove module", `<p>Remove module <b>${escapeHtml(m.name)}</b>? This cannot be undone.</p>`, [
          { label: "Cancel" },
          { label: "Remove", primary: true, onClick: () => deleteModulePermanent(m.id) },
        ]);
        return;
      }
      // Always (re-)open and switch to the module — even if its tab was closed.
      switchToModule(m.id);
    });
    it.addEventListener("dblclick", () => switchToModule(m.id));
    tree.appendChild(it);
  }
}

function renameForm(formId) {
  const f = state.project.forms.find(x => x.id === formId);
  if (!f) return;
  promptDialog("Rename form", "Form name:", f.name, (nm) => {
    if (!nm) return;
    if (!/^[A-Za-z_][A-Za-z0-9_]*$/.test(nm)) { showDialog("Rename form", "<p>Invalid identifier.</p>", [{label:"OK",primary:true}]); return; }
    if (state.project.forms.some(x => x !== f && x.name === nm)) { showDialog("Rename form", "<p>Name already in use.</p>", [{label:"OK",primary:true}]); return; }
    _doRenameForm(f, nm);
  });
}

function _doRenameForm(f, nm) {
  f.name = nm;
  // Refresh tab label.
  const tab = document.querySelector(`#mdi-tabs .mtab[data-form="${f.id}"] .mtab-name`);
  if (tab) tab.textContent = `${f.name} [${state.activeView === "code" ? "Code" : "Design"}]`;
  renderProjectTree();
  renderProperties();
  renderActiveDesigner();
}

function removeFormPrompt(formId) {
  if (state.project.forms.length <= 1) {
    showDialog("Remove form", "<p>Project must contain at least one form.</p>", [{label:"OK",primary:true}]);
    return;
  }
  const f = state.project.forms.find(x => x.id === formId);
  if (!f) return;
  showDialog("Remove form", `<p>Remove form <b>${escapeHtml(f.name)}</b>? This cannot be undone.</p>`, [
    { label: "Cancel" },
    { label: "Remove", primary: true, onClick: () => { deleteFormPermanent(formId); renderProjectTree(); } },
  ]);
}

// ─── MDI tabs / panes ───────────────────────────────────────────

function ensureFormPane(form) {
  // Tab
  let tab = document.querySelector(`#mdi-tabs .mtab[data-form="${form.id}"]`);
  if (!tab) {
    tab = document.createElement("div");
    tab.className = "mtab";
    tab.dataset.form = form.id;
    tab.innerHTML = `<span class="mtab-name">${escapeHtml(form.name)} [Design]</span><span class="x" title="Close">×</span>`;
    tab.addEventListener("click", (e) => {
      if (e.target.classList.contains("x")) {
        closeForm(form.id);
      } else {
        switchToForm(form.id);
      }
    });
    $("#mdi-tabs").appendChild(tab);
  }

  // Pane
  let pane = document.querySelector(`.mdi-pane[data-form="${form.id}"]`);
  if (!pane) {
    pane = document.createElement("div");
    pane.className = "mdi-pane";
    pane.dataset.form = form.id;
    pane.innerHTML = `
      <div class="designer" data-role="designer"></div>
      <div class="code-pane" data-role="code" style="display:none">
        <div class="code-toolbar">
          <select class="obj-dropdown" title="Object"></select>
          <select class="evt-dropdown" title="Event"></select>
        </div>
        <div class="monaco-host"></div>
      </div>
    `;
    $("#mdi-area").appendChild(pane);
  }
}

function switchToForm(formId) {
  state.activeFormId = formId;
  const form = state.project.forms.find(f => f.id === formId);
  if (!form) return;

  // Re-open the tab/pane if it was closed previously.
  ensureFormPane(form);
  // Clear any widget selection so the props pane shows form-level properties.
  state.selection = [];

  $$(".mtab").forEach(t => t.classList.toggle("active", t.dataset.form === formId));
  $$(".mdi-pane").forEach(p => p.classList.toggle("active", p.dataset.form === formId));
  switchView(state.activeView);
  renderActiveDesigner();
  renderProjectTree();
  renderProperties();
}

function switchView(view) {
  state.activeView = view;
  const pane = $(".mdi-pane.active");
  if (!pane) return;
  pane.querySelector('[data-role="designer"]').style.display = view === "designer" ? "" : "none";
  pane.querySelector('[data-role="code"]').style.display     = view === "code"     ? "" : "none";
  // update tab label
  const tab = document.querySelector(`#mdi-tabs .mtab[data-form="${state.activeFormId}"] .mtab-name`);
  const f = state.project.forms.find(f => f.id === state.activeFormId);
  if (tab && f) tab.textContent = `${f.name} [${view === "designer" ? "Design" : "Code"}]`;
  if (view === "code" && f) {
    ensureCodeEditor(f, pane).catch(err => {
      logOutput("monaco load failed: " + err);
      $('.otab[data-tab="errors"]').click();
    });
  }
}

// ─── Monaco-backed code view ───────────────────────────────────

async function ensureCodeEditor(form, pane) {
  if (_editors.has(form.id)) {
    // Already created; refresh layout (in case the pane was hidden).
    const ed = _editors.get(form.id);
    ed.layout();
    populateObjEvtDropdowns(form, pane, ed);
    return ed;
  }
  const host = pane.querySelector(".monaco-host");
  // Defensive: if a previous editor left state on this DOM node, clear it.
  if (host.firstChild) host.innerHTML = "";
  if (!form.code) form.code = { handlers: {}, source: defaultCodeSource(form) };
  if (!form.code.source)   form.code.source = defaultCodeSource(form);
  const ed = await createRapidrEditor(host, {
    value: form.code.source,
    onChange: (txt) => { form.code.source = txt; },
  });
  _editors.set(form.id, ed);
  populateObjEvtDropdowns(form, pane, ed);
  return ed;
}

function defaultCodeSource(form) {
  const lines = [
    `' Code-behind for ${form.name}`,
    `' Add SUB handlers below; designer-defined widgets are auto-emitted.`,
    "",
  ];
  return lines.join("\n");
}

function populateObjEvtDropdowns(form, pane, editor) {
  const objSel = pane.querySelector(".obj-dropdown");
  const evtSel = pane.querySelector(".evt-dropdown");
  if (!objSel || !evtSel) return;

  // Objects: (General) + the form + every widget.
  const objs = ["(General)", form.name, ...form.children.map(w => w.name)];
  objSel.innerHTML = objs.map(n => `<option>${escapeHtml(n)}</option>`).join("");

  function typeOf(obj) {
    if (obj === form.name) return "RForm";
    return form.children.find(w => w.name === obj)?.type || "";
  }
  function refreshEvents() {
    const obj = objSel.value;
    if (obj === "(General)") {
      evtSel.innerHTML = `<option>(declarations)</option>`;
      return;
    }
    const events = eventsFor(typeOf(obj));
    evtSel.innerHTML = events.map(e => `<option>${escapeHtml(e)}</option>`).join("");
  }
  // VB6-style: changing the Object dropdown should jump to (or create) that
  // object's default event handler so the cursor lands inside the right SUB.
  objSel.onchange = () => {
    refreshEvents();
    if (objSel.value === "(General)") return;
    insertHandlerStub(form, objSel.value, evtSel.value, editor);
  };
  refreshEvents();

  evtSel.onchange = () => {
    if (objSel.value === "(General)") return;
    insertHandlerStub(form, objSel.value, evtSel.value, editor);
  };
  objSel.ondblclick = () => {
    if (objSel.value === "(General)") return;
    insertHandlerStub(form, objSel.value, evtSel.value, editor);
  };
}

// Event metadata per component. Each entry is [EventName, [param,...]].
const EVENT_META = {
  RForm: [
    ["OnLoad", []],
    ["OnShow", []],
    ["OnClose", []],
    ["OnResize", ["newWidth", "newHeight"]],
    ["OnKeyDown", ["key"]],
    ["OnKeyUp",   ["key"]],
    ["OnMouseMove", ["x", "y"]],
  ],
  RButton: [
    ["OnClick", []],
    ["OnDblClick", []],
    ["OnMouseEnter", []],
    ["OnMouseLeave", []],
    ["OnFocus", []],
    ["OnBlur", []],
  ],
  RCheckBox: [
    ["OnClick",  []],
    ["OnChange", ["checked"]],
  ],
  RRadioBtn: [
    ["OnClick",  []],
    ["OnChange", ["checked"]],
  ],
  REdit: [
    ["OnChange", ["text"]],
    ["OnEnter",  []],
    ["OnExit",   []],
    ["OnKeyDown", ["key"]],
    ["OnFocus",   []],
    ["OnBlur",    []],
  ],
  RComboBox: [
    ["OnChange", ["selectedIndex"]],
    ["OnFocus",  []],
    ["OnBlur",   []],
  ],
  RListBox: [
    ["OnChange",   ["selectedIndex"]],
    ["OnDblClick", ["selectedIndex"]],
  ],
  RTimer: [
    ["OnTimer", []],
  ],
  RImage:    [["OnClick", []], ["OnDblClick", []]],
  RPanel:    [["OnClick", []]],
  RGroupBox: [["OnClick", []]],
};

function eventsFor(type) {
  return (EVENT_META[type] || [["OnClick", []]]).map(e => e[0]);
}

function eventParamsFor(type, evtName) {
  const arr = EVENT_META[type] || [];
  const m = arr.find(e => e[0] === evtName);
  return m ? m[1] : [];
}

function insertHandlerStub(form, objName, evtName, editor) {
  if (!objName || objName === "(General)" || !evtName) return;
  const subName = `${objName}_${evtName.replace(/^On/, "")}`;
  const text = editor.getValue();
  // If the sub already exists, jump to it instead of adding a duplicate.
  const re = new RegExp(`^\\s*SUB\\s+${subName}\\b`, "im");
  const m = re.exec(text);
  if (m) {
    const line = text.slice(0, m.index).split("\n").length;
    editor.revealLineInCenter(line);
    editor.setPosition({ lineNumber: line + 1, column: 3 });
    editor.focus();
    return;
  }
  // Find type for parameters.
  const wType = (objName === form.name) ? "RForm"
    : (form.children.find(w => w.name === objName)?.type || "");
  const params = eventParamsFor(wType, evtName);
  const paramList = params.length ? `(${params.join(", ")})` : "";
  const stub = `\nSUB ${subName}${paramList}\n  \nEND SUB\n`;
  const lineCount = editor.getModel().getLineCount();
  editor.executeEdits("add-stub", [{
    range: new (window.monaco.Range)(lineCount + 1, 1, lineCount + 1, 1),
    text: stub,
    forceMoveMarkers: true,
  }]);
  editor.setPosition({ lineNumber: lineCount + 3, column: 3 });
  editor.focus();
  // Mark binding in model.
  if (objName === form.name) {
    form.code.handlers[evtName] = subName;
  } else {
    const w = findWidget(form, objName);
    if (w) {
      if (!w.code) w.code = { handlers: {} };
      w.code.handlers[evtName] = subName;
    }
  }
  form.code.source = editor.getValue();
}

// Close a form TAB (keeps the form in the project — destructive
// removal happens via removeFormPrompt from the project tree).
function closeForm(formId) {
  document.querySelector(`#mdi-tabs .mtab[data-form="${formId}"]`)?.remove();
  document.querySelector(`.mdi-pane[data-form="${formId}"]`)?.remove();
  if (_editors.has(formId)) { _editors.get(formId).dispose(); _editors.delete(formId); }
  if (state.activeFormId === formId) {
    const next = state.project.forms.find(f => f.id !== formId);
    if (next) switchToForm(next.id);
    else state.activeFormId = null;
  }
  renderProjectTree();
}

// Permanently delete a form from the project.
function deleteFormPermanent(formId) {
  if (state.project.forms.length <= 1) return;
  state.project.forms = state.project.forms.filter(f => f.id !== formId);
  closeForm(formId);
}

// ─── Designer (Phase C: drag/resize/snap/multiselect) ────────────────

const SNAP = 8;
const snap = (n) => Math.round(n / SNAP) * SNAP;

function renderActiveDesigner() {
  const form = state.project.forms.find(f => f.id === state.activeFormId);
  if (!form) return;
  const designer = document.querySelector(`.mdi-pane[data-form="${form.id}"] .designer`);
  if (!designer) return;

  designer.innerHTML = "";
  const formEl = document.createElement("div");
  formEl.className = "design-form";
  formEl.style.width  = (form.props.width  ?? 480) + "px";
  formEl.style.height = (form.props.height ?? 320) + "px";
  formEl.dataset.formId = form.id;
  applyVisualProps(formEl, form.props, "RForm");

  const tb = document.createElement("div");
  tb.className = "form-titlebar";
  const tbTitle = document.createElement("span");
  tbTitle.className = "form-title-text";
  tbTitle.textContent = form.props.caption || form.name;
  tb.appendChild(tbTitle);
  const tbBtns = document.createElement("span");
  tbBtns.className = "form-title-btns";
  tbBtns.innerHTML =
    '<button class="form-min" title="Minimize">\u2014</button>' +
    '<button class="form-max" title="Maximize">\u25a1</button>' +
    '<button class="form-close" title="Close">\u2715</button>';
  tb.appendChild(tbBtns);
  formEl.appendChild(tb);
  tbBtns.querySelector(".form-min").addEventListener("click", (e) => {
    e.stopPropagation();
    form.props.windowstate = (form.props.windowstate | 0) === 1 ? 0 : 1;
    renderActiveDesigner(); renderProperties();
  });
  tbBtns.querySelector(".form-max").addEventListener("click", (e) => {
    e.stopPropagation();
    if ((form.props.windowstate | 0) === 2) {
      const g = form._savedGeom;
      if (g) { form.props.width = g.w; form.props.height = g.h; }
      form.props.windowstate = 0;
    } else {
      form._savedGeom = { w: form.props.width ?? 480, h: form.props.height ?? 320 };
      const designerRect = designer.getBoundingClientRect();
      form.props.width  = Math.max(200, Math.round(designerRect.width  - 24));
      form.props.height = Math.max(120, Math.round(designerRect.height - 24));
      form.props.windowstate = 2;
    }
    renderActiveDesigner(); renderProperties();
  });
  tbBtns.querySelector(".form-close").addEventListener("click", (e) => {
    e.stopPropagation();
    closeForm(form.id);
  });

  for (const w of form.children) {
    if (!isVisibleType(w.type)) continue;
    formEl.appendChild(buildWidgetEl(w, state.selection.includes(w.name)));
  }

  // Form-level resize handles (VB6-style 8 handles)
  for (const dir of ["nw", "n", "ne", "e", "se", "s", "sw", "w"]) {
    const h = document.createElement("div");
    h.className = "form-handle " + dir;
    h.dataset.handle = dir;
    h.addEventListener("mousedown", (ev) => beginFormResize(ev, form, dir));
    formEl.appendChild(h);
  }

  designer.appendChild(formEl);
  designer.classList.toggle("arming", state.selectedTool !== "pointer");

  // Non-visual tray for invisible components (Timer, SQLite, HTTP, …).
  const trayItems = form.children.filter(w => !isVisibleType(w.type));
  if (trayItems.length) {
    const tray = document.createElement("div");
    tray.className = "design-tray";
    tray.title = "Non-visual components (not rendered on form)";
    for (const w of trayItems) {
      const chip = document.createElement("div");
      chip.className = "tray-chip" + (state.selection.includes(w.name) ? " selected" : "");
      chip.dataset.widget = w.name;
      chip.textContent = `⚙ ${w.name} (${w.type})`;
      chip.addEventListener("click", (e) => {
        e.stopPropagation();
        state.selection = e.shiftKey
          ? Array.from(new Set([...state.selection, w.name]))
          : [w.name];
        renderActiveDesigner();
        renderProperties();
      });
      chip.addEventListener("dblclick", (e) => {
        e.stopPropagation();
        // Open code-behind for default event of this component.
        onDesignerDoubleClick({ target: chip, currentTarget: formEl, stopPropagation(){} });
      });
      tray.appendChild(chip);
    }
    designer.appendChild(tray);
  }

  formEl.addEventListener("mousedown", onDesignerMouseDown);
  formEl.addEventListener("dblclick",  onDesignerDoubleClick);

  updateLayoutDock();
}

function beginFormResize(ev, form, dir) {
  ev.preventDefault();
  ev.stopPropagation();
  const startX = ev.clientX, startY = ev.clientY;
  const startW = form.props.width  ?? 480;
  const startH = form.props.height ?? 320;
  const formEl = ev.currentTarget.parentElement;
  const onMove = (e) => {
    const dx = e.clientX - startX;
    const dy = e.clientY - startY;
    let w = startW, h = startH;
    if (dir.includes("e")) w = startW + dx;
    if (dir.includes("w")) w = startW - dx;
    if (dir.includes("s")) h = startH + dy;
    if (dir.includes("n")) h = startH - dy;
    form.props.width  = Math.max(120, Math.round(w));
    form.props.height = Math.max(80,  Math.round(h));
    formEl.style.width  = form.props.width + "px";
    formEl.style.height = form.props.height + "px";
  };
  const onUp = () => {
    document.removeEventListener("mousemove", onMove);
    document.removeEventListener("mouseup", onUp);
    renderProperties();
  };
  document.addEventListener("mousemove", onMove);
  document.addEventListener("mouseup", onUp);
}

function buildWidgetEl(w, selected) {
  const el = document.createElement("div");
  el.className = "dwidget dw-" + w.type + (selected ? " selected" : "");
  el.dataset.name = w.name;
  el.dataset.type = w.type;
  el.style.left   = (w.props.left   ?? 0) + "px";
  el.style.top    = (w.props.top    ?? 0) + "px";
  el.style.width  = (w.props.width  ?? 80) + "px";
  el.style.height = (w.props.height ?? 24) + "px";
  // Build the inner "real" component preview so the designer reflects
  // sizes / fonts / colors / etc. live.
  const inner = renderRealComponent(w);
  el.appendChild(inner);
  applyVisualProps(inner, w.props, w.type);
  if (selected) {
    for (const dir of ["nw","n","ne","e","se","s","sw","w"]) {
      const h = document.createElement("div");
      h.className = "handle " + dir;
      h.dataset.handle = dir;
      el.appendChild(h);
    }
  }
  return el;
}

function renderRealComponent(w) {
  const txt = String(w.props.caption ?? w.props.text ?? w.name);
  let inner;
  switch (w.type) {
    case "RButton": {
      inner = document.createElement("button");
      inner.type = "button";
      inner.textContent = txt;
      inner.disabled = !!w.props.enabled === false ? false : !((w.props.enabled ?? 1) | 0);
      break;
    }
    case "RLabel": {
      inner = document.createElement("span");
      inner.textContent = txt;
      break;
    }
    case "REdit": {
      inner = document.createElement("input");
      inner.type = "text";
      inner.value = txt;
      inner.readOnly = !!Number(w.props.readonly);
      break;
    }
    case "RCheckBox": {
      inner = document.createElement("label");
      const c = document.createElement("input");
      c.type = "checkbox";
      c.checked = !!Number(w.props.checked);
      const s = document.createElement("span");
      s.textContent = " " + txt;
      inner.appendChild(c); inner.appendChild(s);
      break;
    }
    case "RRadioBtn": {
      inner = document.createElement("label");
      const r = document.createElement("input");
      r.type = "radio"; r.checked = !!Number(w.props.checked);
      const s = document.createElement("span");
      s.textContent = " " + txt;
      inner.appendChild(r); inner.appendChild(s);
      break;
    }
    case "RComboBox": {
      inner = document.createElement("select");
      const items = String(w.props.items ?? "").split(/[,;\n]/).map(s => s.trim()).filter(Boolean);
      for (const it of (items.length ? items : [txt])) {
        const o = document.createElement("option"); o.textContent = it; inner.appendChild(o);
      }
      break;
    }
    case "RListBox": {
      inner = document.createElement("div");
      inner.className = "dw-listbox";
      const items = String(w.props.items ?? "").split(/[,;\n]/).map(s => s.trim()).filter(Boolean);
      for (const it of (items.length ? items : ["item1","item2"])) {
        const r = document.createElement("div"); r.textContent = it; inner.appendChild(r);
      }
      break;
    }
    case "RImage": {
      inner = document.createElement("div");
      inner.className = "dw-image";
      if (w.props.picture) {
        inner.style.backgroundImage = `url(${JSON.stringify(w.props.picture)})`;
      } else inner.textContent = "🖼 " + txt;
      break;
    }
    case "RPanel": {
      inner = document.createElement("div");
      inner.className = "dw-panel";
      break;
    }
    case "RGroupBox": {
      inner = document.createElement("fieldset");
      const lg = document.createElement("legend");
      lg.textContent = txt;
      inner.appendChild(lg);
      break;
    }
    case "RTimer": {
      inner = document.createElement("div");
      inner.className = "dw-timer";
      inner.textContent = "⏱ " + (w.props.interval ?? 1000) + "ms";
      break;
    }
    case "RMemo":
    case "RRichEdit": {
      inner = document.createElement("textarea");
      inner.value = String(w.props.text ?? "");
      inner.readOnly = true;
      break;
    }
    case "RCanvas": {
      inner = document.createElement("div");
      inner.className = "dw-canvas";
      inner.style.background = "#fff";
      inner.style.border = "1px dashed #aaa";
      inner.style.display = "flex";
      inner.style.alignItems = "center";
      inner.style.justifyContent = "center";
      inner.style.color = "#888";
      inner.textContent = "🎨 " + w.name;
      break;
    }
    case "RProgressBar": {
      inner = document.createElement("progress");
      inner.max = Number(w.props.max ?? 100);
      inner.value = Number(w.props.value ?? w.props.position ?? 0);
      inner.style.width = "100%";
      break;
    }
    case "RTrackBar":
    case "RScrollBar": {
      inner = document.createElement("input");
      inner.type = "range";
      inner.min = Number(w.props.min ?? 0);
      inner.max = Number(w.props.max ?? 100);
      inner.value = Number(w.props.value ?? w.props.position ?? 0);
      inner.style.width = "100%";
      break;
    }
    case "RUpDown": {
      inner = document.createElement("input");
      inner.type = "number";
      inner.value = Number(w.props.value ?? 0);
      inner.style.width = "100%";
      break;
    }
    case "RDateTimePicker": {
      inner = document.createElement("input");
      inner.type = "datetime-local";
      inner.style.width = "100%";
      break;
    }
    case "RStringGrid":
    case "RListView":
    case "RDataFrame": {
      inner = document.createElement("div");
      inner.className = "dw-grid";
      inner.style.background = "#fff";
      inner.style.border = "1px solid #aaa";
      inner.style.overflow = "hidden";
      inner.style.fontFamily = "monospace";
      inner.style.fontSize = "11px";
      const rows = Number(w.props.rowcount ?? 4);
      const cols = Number(w.props.colcount ?? 3);
      const tbl = document.createElement("table");
      tbl.style.width = "100%";
      tbl.style.borderCollapse = "collapse";
      for (let r = 0; r < Math.min(rows, 6); r++) {
        const tr = document.createElement("tr");
        for (let c = 0; c < Math.min(cols, 6); c++) {
          const td = document.createElement(r === 0 ? "th" : "td");
          td.style.border = "1px solid #ddd";
          td.style.padding = "1px 4px";
          td.style.background = r === 0 ? "#eee" : "white";
          td.textContent = r === 0 ? `Col${c+1}` : "";
          tr.appendChild(td);
        }
        tbl.appendChild(tr);
      }
      inner.appendChild(tbl);
      break;
    }
    case "RTreeView": {
      inner = document.createElement("div");
      inner.className = "dw-tree";
      inner.style.background = "#fff";
      inner.style.border = "1px solid #aaa";
      inner.style.padding = "4px 8px";
      inner.style.fontFamily = "monospace";
      inner.style.fontSize = "11px";
      inner.innerHTML = "▸ Root<br>&nbsp;&nbsp;▸ Child 1<br>&nbsp;&nbsp;▸ Child 2";
      break;
    }
    case "RTabControl": {
      inner = document.createElement("div");
      inner.className = "dw-tabs";
      inner.style.background = "#fff";
      inner.style.border = "1px solid #aaa";
      const tabs = document.createElement("div");
      tabs.style.display = "flex";
      tabs.style.borderBottom = "1px solid #ccc";
      tabs.style.background = "#f0f0f0";
      tabs.style.fontSize = "11px";
      ["Tab 1", "Tab 2", "Tab 3"].forEach((t, i) => {
        const ti = document.createElement("div");
        ti.style.padding = "4px 12px";
        ti.style.borderRight = "1px solid #ccc";
        if (i === 0) { ti.style.background = "#fff"; ti.style.fontWeight = "600"; }
        ti.textContent = t;
        tabs.appendChild(ti);
      });
      inner.appendChild(tabs);
      break;
    }
    case "RScrollBox": {
      inner = document.createElement("div");
      inner.style.background = "#fff";
      inner.style.border = "1px solid #aaa";
      inner.style.overflow = "auto";
      break;
    }
    case "RSplitter": {
      inner = document.createElement("div");
      inner.style.background = "#888";
      inner.style.cursor = "col-resize";
      break;
    }
    case "RCodeEditor": {
      inner = document.createElement("pre");
      inner.style.background = "#1a1a2e";
      inner.style.color = "#4ade80";
      inner.style.padding = "4px 8px";
      inner.style.margin = "0";
      inner.style.fontSize = "11px";
      inner.style.fontFamily = "monospace";
      inner.style.overflow = "hidden";
      inner.textContent = String(w.props.text || "// code editor");
      break;
    }
    case "RCoolBtn":
    case "ROvalBtn": {
      inner = document.createElement("button");
      inner.type = "button";
      inner.textContent = txt;
      if (w.type === "ROvalBtn") inner.style.borderRadius = "999px";
      break;
    }
    case "RLine": {
      inner = document.createElement("div");
      inner.style.background = String(w.props.color || "#888");
      inner.style.width = "100%";
      inner.style.height = "100%";
      break;
    }
    case "RMainMenu": {
      inner = document.createElement("div");
      inner.style.background = "#f0f0f0";
      inner.style.borderBottom = "1px solid #ccc";
      inner.style.fontSize = "12px";
      inner.style.padding = "4px 8px";
      inner.textContent = String(w.props.caption || "File   Edit   View   Help");
      break;
    }
    case "RToolBar": {
      inner = document.createElement("div");
      inner.style.background = "#f0f0f0";
      inner.style.borderBottom = "1px solid #ccc";
      inner.style.padding = "4px";
      inner.style.display = "flex";
      inner.style.gap = "4px";
      ["▭", "▭", "▭"].forEach(x => {
        const b = document.createElement("span");
        b.style.padding = "2px 6px";
        b.style.border = "1px solid #ccc";
        b.style.background = "#fff";
        b.textContent = x;
        inner.appendChild(b);
      });
      break;
    }
    case "RStatusBar": {
      inner = document.createElement("div");
      inner.style.background = "#e8e8e8";
      inner.style.borderTop = "1px solid #ccc";
      inner.style.padding = "2px 8px";
      inner.style.fontSize = "11px";
      inner.textContent = String(w.props.caption || "Ready");
      break;
    }
    case "RWebView": {
      inner = document.createElement("div");
      inner.style.background = "#fff";
      inner.style.border = "1px solid #aaa";
      inner.style.display = "flex";
      inner.style.alignItems = "center";
      inner.style.justifyContent = "center";
      inner.style.color = "#888";
      inner.textContent = "🌍 " + String(w.props.url || "WebView");
      break;
    }
    case "RDOM": {
      inner = document.createElement("div");
      inner.style.background = "#fff";
      inner.style.border = "1px dashed #888";
      inner.style.padding = "4px 8px";
      inner.style.fontFamily = "monospace";
      inner.style.fontSize = "11px";
      inner.textContent = `<${w.props.tagname || "div"}>${w.props.innerhtml || ""}`;
      break;
    }
    case "RWebVideo": {
      inner = document.createElement("div");
      inner.style.background = "#000";
      inner.style.color = "#fff";
      inner.style.display = "flex";
      inner.style.alignItems = "center";
      inner.style.justifyContent = "center";
      inner.textContent = "🎬 " + (w.props.src || "Video");
      break;
    }
    case "RPlot": {
      inner = document.createElement("div");
      inner.style.background = "#fff";
      inner.style.border = "1px solid #aaa";
      inner.style.display = "flex";
      inner.style.alignItems = "center";
      inner.style.justifyContent = "center";
      inner.style.color = "#666";
      inner.textContent = "📈 Plot — " + w.name;
      break;
    }
    default: {
      inner = document.createElement("div");
      inner.textContent = txt;
    }
  }
  inner.classList.add("dw-inner");
  inner.style.pointerEvents = "none";
  return inner;
}

function applyVisualProps(el, props, type) {
  const fontName  = props.fontname || props.font;
  const fontSize  = props.fontsize ?? props.fontSize;
  const fontColor = props.fontcolor || props.foreground;
  // Legacy VB6/Delphi semantics: `color` is the BACKGROUND fill on every
  // visible widget (button, panel, form, etc.). `fontcolor`/`foreground`
  // is the text colour. We treat `color` as background everywhere; for
  // text colour fall back to it only on widgets that don't have a
  // dedicated text-on-background like Buttons.
  const bgColor   = props.background || props.fillcolor || props.brushcolor || props.color;
  const fgColor   = fontColor;
  if (fontName)         el.style.fontFamily = fontName;
  if (fontSize !== undefined && fontSize !== "") el.style.fontSize = fontSize + "px";
  if (fgColor)          el.style.color = fgColor;
  if (bgColor)          el.style.background = bgColor;
  if (props.alignment)  el.style.textAlign = String(props.alignment).toLowerCase();
  if (props.bordercolor) el.style.borderColor = props.bordercolor;
  if (props.visible !== undefined && !Number(props.visible)) el.style.opacity = "0.35";
}

function onDesignerMouseDown(e) {
  if (e.button !== 0) return;
  const form = state.project.forms.find(f => f.id === state.activeFormId);
  if (!form) return;
  const formEl = e.currentTarget;
  if (e.target.closest(".form-titlebar")) return;     // ignore titlebar drags

  const rect = formEl.getBoundingClientRect();
  const startX = e.clientX - rect.left;
  const startY = e.clientY - rect.top;

  // Armed tool → draw a new widget by dragging.
  if (state.selectedTool !== "pointer") {
    return beginDraw(form, formEl, startX, startY, e);
  }

  // Pointer mode.
  const handle = e.target.closest(".handle");
  if (handle) {
    return beginResize(form, formEl, handle.dataset.handle, e);
  }

  const wEl = e.target.closest(".dwidget");
  if (wEl) {
    const name = wEl.dataset.name;
    // Manual double-click detection (real dblclick can be suppressed by drag listeners).
    const now = Date.now();
    if (_lastClick && _lastClick.name === name && (now - _lastClick.t) < 350) {
      _lastClick = null;
      onDesignerDoubleClick({ target: wEl });
      return;
    }
    _lastClick = { name, t: now };
    if (e.shiftKey) {
      const idx = state.selection.indexOf(name);
      if (idx >= 0) state.selection.splice(idx, 1);
      else state.selection.push(name);
    } else if (!state.selection.includes(name)) {
      state.selection = [name];
    }
    renderActiveDesigner();
    renderProperties();
    // After re-render the formEl in scope is detached. Look up the live one.
    const liveFormEl = document.querySelector(`.mdi-pane[data-form="${form.id}"] .design-form`);
    return beginMove(form, liveFormEl || formEl, e);
  }

  // Empty form area: start rubber-band select (or clear selection).
  state.selection = [];
  renderActiveDesigner();
  renderProperties();
  beginRubberBand(form, formEl, startX, startY);
}

let _lastClick = null;

function onDesignerDoubleClick(e) {
  const wEl = e.target.closest(".dwidget");
  if (!wEl) return;
  const form = state.project.forms.find(f => f.id === state.activeFormId);
  const w = findWidget(form, wEl.dataset.name);
  if (!w) return;
  state.selection = [w.name];
  switchView("code");
  const tries = setInterval(() => {
    const ed = _editors.get(form.id);
    if (!ed) return;
    clearInterval(tries);
    const pane = document.querySelector(`.mdi-pane[data-form="${form.id}"]`);
    populateObjEvtDropdowns(form, pane, ed);
    const objSel = pane.querySelector(".obj-dropdown");
    const evtSel = pane.querySelector(".evt-dropdown");
    objSel.value = w.name;
    // Manually trigger the dropdown's onchange to refresh the event list
    // for the now-selected object (.value = ... does NOT fire change).
    objSel.dispatchEvent(new Event("change"));
    evtSel.value = eventsFor(w.type)[0] || "OnClick";
    insertHandlerStub(form, w.name, evtSel.value, ed);
  }, 50);
}

// ─── Drag operations ──────────────────────────────────────────

function beginDraw(form, formEl, startX, startY, ev) {
  const rubber = document.createElement("div");
  rubber.className = "rubber";
  rubber.style.left = startX + "px";
  rubber.style.top  = startY + "px";
  formEl.appendChild(rubber);

  const onMove = (e) => {
    const rect = formEl.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const y = e.clientY - rect.top;
    const l = Math.min(startX, x), t = Math.min(startY, y);
    rubber.style.left   = l + "px";
    rubber.style.top    = t + "px";
    rubber.style.width  = Math.abs(x - startX) + "px";
    rubber.style.height = Math.abs(y - startY) + "px";
  };
  const onUp = (e) => {
    document.removeEventListener("mousemove", onMove);
    document.removeEventListener("mouseup",   onUp);
    const rect = formEl.getBoundingClientRect();
    const x = e.clientX - rect.left, y = e.clientY - rect.top;
    let l = Math.min(startX, x), t = Math.min(startY, y);
    let wd = Math.abs(x - startX), ht = Math.abs(y - startY);
    if (wd < 4 && ht < 4) {
      // Click without drag → use widget defaults.
      l = snap(startX); t = snap(startY); wd = undefined; ht = undefined;
    } else {
      l = snap(l); t = snap(t); wd = Math.max(SNAP, snap(wd)); ht = Math.max(SNAP, snap(ht));
    }
    const geom = wd && ht ? { left: l, top: t, width: wd, height: ht } : { left: l, top: t };
    const w = addWidget(form, state.selectedTool, geom);
    state.selection = [w.name];
    armTool("pointer");
    renderActiveDesigner();
    renderProperties();
    renderProjectTree();
  };
  document.addEventListener("mousemove", onMove);
  document.addEventListener("mouseup",   onUp);
  ev.preventDefault();
}

function beginMove(form, formEl, ev) {
  // Snapshot starting positions for every selected widget.
  const starts = state.selection.map(name => {
    const w = findWidget(form, name);
    return { w, x: w.props.left ?? 0, y: w.props.top ?? 0 };
  }).filter(s => s.w);
  const startMx = ev.clientX, startMy = ev.clientY;
  let moved = false;

  const onMove = (e) => {
    const dx = e.clientX - startMx;
    const dy = e.clientY - startMy;
    if (Math.abs(dx) + Math.abs(dy) < 3) return;
    moved = true;
    for (const s of starts) {
      const nx = snap(s.x + dx), ny = snap(s.y + dy);
      s.w.props.left = nx;
      s.w.props.top  = ny;
      const el = formEl.querySelector(`.dwidget[data-name="${s.w.name}"]`);
      if (el) { el.style.left = nx + "px"; el.style.top = ny + "px"; }
    }
  };
  const onUp = () => {
    document.removeEventListener("mousemove", onMove);
    document.removeEventListener("mouseup",   onUp);
    if (moved) renderProperties();
  };
  document.addEventListener("mousemove", onMove);
  document.addEventListener("mouseup",   onUp);
}

function beginResize(form, formEl, dir, ev) {
  if (state.selection.length !== 1) return;
  const w = findWidget(form, state.selection[0]);
  if (!w) return;
  const start = { x: w.props.left, y: w.props.top, w: w.props.width, h: w.props.height };
  const startMx = ev.clientX, startMy = ev.clientY;
  const el = formEl.querySelector(`.dwidget[data-name="${w.name}"]`);

  const onMove = (e) => {
    const dx = e.clientX - startMx;
    const dy = e.clientY - startMy;
    let { x, y, w: wd, h: ht } = start;
    if (dir.includes("e")) wd = Math.max(SNAP, start.w + dx);
    if (dir.includes("s")) ht = Math.max(SNAP, start.h + dy);
    if (dir.includes("w")) { x = start.x + dx; wd = Math.max(SNAP, start.w - dx); }
    if (dir.includes("n")) { y = start.y + dy; ht = Math.max(SNAP, start.h - dy); }
    x = snap(x); y = snap(y); wd = snap(wd); ht = snap(ht);
    w.props.left = x; w.props.top = y; w.props.width = wd; w.props.height = ht;
    if (el) {
      el.style.left = x + "px"; el.style.top = y + "px";
      el.style.width = wd + "px"; el.style.height = ht + "px";
    }
  };
  const onUp = () => {
    document.removeEventListener("mousemove", onMove);
    document.removeEventListener("mouseup",   onUp);
    renderProperties();
  };
  document.addEventListener("mousemove", onMove);
  document.addEventListener("mouseup",   onUp);
  ev.preventDefault();
  ev.stopPropagation();
}

function beginRubberBand(form, formEl, startX, startY) {
  const rb = document.createElement("div");
  rb.className = "rubber";
  rb.style.left = startX + "px";
  rb.style.top  = startY + "px";
  formEl.appendChild(rb);

  const onMove = (e) => {
    const rect = formEl.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const y = e.clientY - rect.top;
    const l = Math.min(startX, x), t = Math.min(startY, y);
    const r = Math.max(startX, x), b = Math.max(startY, y);
    rb.style.left   = l + "px";
    rb.style.top    = t + "px";
    rb.style.width  = (r - l) + "px";
    rb.style.height = (b - t) + "px";
    state.selection = form.children.filter(w => {
      const wl = w.props.left, wt = w.props.top;
      const wr = wl + (w.props.width  || 0), wb = wt + (w.props.height || 0);
      return wl < r && wr > l && wt < b && wb > t;
    }).map(w => w.name);
    // Re-render selection markers.
    formEl.querySelectorAll(".dwidget").forEach(el => {
      el.classList.toggle("selected", state.selection.includes(el.dataset.name));
    });
  };
  const onUp = () => {
    document.removeEventListener("mousemove", onMove);
    document.removeEventListener("mouseup",   onUp);
    rb.remove();
    renderActiveDesigner();
    renderProperties();
  };
  document.addEventListener("mousemove", onMove);
  document.addEventListener("mouseup",   onUp);
}

// ─── Properties window (Phase A: flat key/value; Phase D will replace) ──

// ─── Property metadata (Phase D: typed property grid) ──────────

const COLOR_PROPS = new Set([
  "background", "foreground", "color", "bordercolor", "fillcolor",
  "fontcolor", "pencolor", "brushcolor", "colorhighlight", "colorshadow",
]);
const FONT_NAME_PROPS = new Set(["fontname", "font"]);
const NUM_PROPS = new Set([
  "left","top","width","height","fontsize","interval","startX","startY","value","min","max","step",
  "position","smallchange","largechange","tickfrequency","alphablendvalue","taborder",
  "cols","rows","fixedcols","fixedrows","colcount","rowcount","colwidth","rowheight",
  "defaultcolwidth","defaultrowheight","gridlinewidth","itemcount","itemindex",
  "linecount","selstart","sellength","caretx","carety","line","col","row",
  "port","timeout","statuscode","groupindex","numbmps","spacing","x1","y1","x2","y2",
  "bmpwidth","bmpheight","helpcontext","sortcolumn","filterindex",
  "volume","currenttime","duration","latitude","longitude","accuracy",
  "size","length","len","ndim","count","dpi",
]);
const BOOL_PROPS = new Set([
  "visible","enabled","checked","modal","resizable","wrap","readonly","autosize",
  "transparent","wordwrap","sorted","multiselect","flat","down","allowallup",
  "linenumbers","editorenabled","showhint","center","connected","usessl",
  "loop","autoplay","controls","playing","paused","simplepanel","grid",
  "gridlines","checkboxes","rowselect","empty","alphablend",
]);

const ENUMS = {
  borderstyle: ["0", "1", "2", "3"],          // none / single / sunken / fixed-dialog
  alignment:   ["left", "center", "right"],
  cursor:      ["default","pointer","crosshair","text","move","wait"],
  windowstate: ["0", "1", "2"],               // normal / minimized / maximized
  formstyle:   ["0", "1", "2", "3"],          // normal / mdichild / mdiparent / staysontop
  orientation: ["horizontal", "vertical"],
  scrollbars:  ["none","horizontal","vertical","both"],
  viewstyle:   ["icon","smallicon","list","report"],
  storagetype: ["local","session"],
  kind:        ["horizontal","vertical"],
  sorttype:    ["none","ascending","descending"],
  layout:      ["left","right","top","bottom"],
  align:       ["none","top","bottom","left","right","client"],
  style:       ["0","1","2"],
};

const CATEGORIES = {
  caption: "Appearance", text: "Appearance", picture: "Appearance",
  background: "Appearance", foreground: "Appearance", borderstyle: "Appearance",
  alignment: "Appearance", color: "Appearance", bordercolor: "Appearance",
  fillcolor: "Appearance", visible: "Appearance",

  fontname: "Font", fontsize: "Font", fontcolor: "Font", font: "Font",

  left: "Layout", top: "Layout", width: "Layout", height: "Layout",
  startX: "Layout", startY: "Layout",

  enabled: "Behavior", checked: "Behavior", modal: "Behavior",
  resizable: "Behavior", interval: "Behavior", readonly: "Behavior",
  autosize: "Behavior", tooltip: "Behavior", hint: "Behavior",
  url: "Behavior", items: "Behavior", value: "Behavior",
  min: "Behavior", max: "Behavior", step: "Behavior", cursor: "Behavior",
};

function propType(key) {
  if (ENUMS[key])             return "enum";
  if (ASSET_PROPS.has(key))   return "asset";
  if (COLOR_PROPS.has(key))   return "color";
  if (FONT_NAME_PROPS.has(key)) return "font";
  if (BOOL_PROPS.has(key))    return "bool";
  if (NUM_PROPS.has(key))     return "number";
  return "string";
}
function propCategory(key) { return CATEGORIES[key] || "Misc"; }

// Properties that point to a project-bundled asset OR an external URL.
// Examples: RImage.picture, RImage.src, RDataFrame.source, RPlot.dataset.
const ASSET_PROPS = new Set([
  "picture", "src", "source", "dataset", "csv", "csvfile", "image", "imageurl",
]);

// Boolean props that should default to TRUE (1) when not explicitly set.
const BOOL_DEFAULT_TRUE = new Set([
  "enabled", "visible", "controlbox", "maxbutton", "minbutton", "showhint",
  "tabstop", "autosize", "wordwrap", "sorted",
]);
// Numeric props with non-zero sensible defaults.
const NUM_DEFAULTS = {
  fontsize: 9, alphablendvalue: 255, taborder: 0, max: 100, min: 0,
  smallchange: 1, largechange: 10, tickfrequency: 1,
};
function defaultPropValue(k) {
  const t = propType(k);
  if (t === "bool")   return BOOL_DEFAULT_TRUE.has(k) ? 1 : 0;
  if (t === "number") return NUM_DEFAULTS[k] ?? 0;
  if (t === "color")  return "";
  if (t === "font")   return "Tahoma";
  return "";
}

const FONT_FAMILIES = ["Tahoma","Arial","Verdana","Times New Roman","Courier New","Segoe UI","MS Sans Serif"];

function renderProperties() {
  const body = $("#props-body");
  body.innerHTML = "";

  const form = state.project.forms.find(f => f.id === state.activeFormId);
  if (!form) {
    $("#props-target").textContent = "(none)";
    return;
  }

  let target = null;
  if (state.selection.length === 1) {
    target = findWidget(form, state.selection[0]);
  } else if (state.selection.length === 0) {
    target = { name: form.name, type: "RForm", props: form.props };
  }
  if (!target) {
    $("#props-target").textContent = `${state.selection.length} items selected`;
    return;
  }

  $("#props-target").textContent = `${target.name}  ${target.type}`;

  const filter = ($("#props-search")?.value || "").trim().toLowerCase();
  const mode = state.propsMode || "cat";   // "cat" or "abc"

  // Union: stored props + every prop the registry knows for this type.
  const stored = target.props || {};
  const reg = COMPONENT_REGISTRY[(target.type || "").toUpperCase()];
  const allKeys = new Set(Object.keys(stored));
  if (reg) for (const p of reg.props) allKeys.add(p);
  // Dedupe legacy aliases: prefer `fontname` over the `font` alias when both
  // would appear (registry exposes `font` for backwards compatibility but
  // the property grid + serializer both speak `fontname`).
  if (allKeys.has("fontname")) allKeys.delete("font");
  let entries = [...allKeys].map(k => [k, stored[k] ?? defaultPropValue(k)]);
  if (filter) entries = entries.filter(([k]) => k.toLowerCase().includes(filter));

  if (mode === "abc") {
    entries.sort(([a],[b]) => a.localeCompare(b));
    for (const [k, v] of entries) body.appendChild(buildPropRow(form, target, k, v));
    return;
  }

  // Categorized: group + sort categories.
  const groups = {};
  for (const [k, v] of entries) {
    const cat = propCategory(k);
    (groups[cat] ||= []).push([k, v]);
  }
  const catOrder = ["Appearance","Font","Layout","Behavior","Misc"];
  const cats = Object.keys(groups).sort((a,b) => {
    const ai = catOrder.indexOf(a), bi = catOrder.indexOf(b);
    return (ai < 0 ? 99 : ai) - (bi < 0 ? 99 : bi);
  });
  for (const cat of cats) {
    const hdr = document.createElement("div");
    hdr.className = "prop-cat";
    hdr.textContent = cat;
    body.appendChild(hdr);
    groups[cat].sort(([a],[b]) => a.localeCompare(b));
    for (const [k, v] of groups[cat]) body.appendChild(buildPropRow(form, target, k, v));
  }
}

function buildPropRow(form, target, k, v) {
  const row = document.createElement("div");
  row.className = "prop-row";
  row.dataset.key = k;

  const nameDiv = document.createElement("div");
  nameDiv.className = "prop-name";
  nameDiv.title = k;
  nameDiv.textContent = k;
  row.appendChild(nameDiv);

  const valDiv = document.createElement("div");
  valDiv.className = "prop-val";
  const t = propType(k);
  let editor;

  const commit = (raw) => {
    let coerced = raw;
    if (t === "number") coerced = Number(raw);
    else if (t === "bool") coerced = raw ? 1 : 0;
    if (target.type === "RForm") form.props[k] = coerced;
    else setProp(form, target.name, k, coerced);
    renderActiveDesigner();
    updateLayoutDock();
  };

  if (t === "bool") {
    editor = document.createElement("input");
    editor.type = "checkbox";
    editor.checked = !!Number(v);
    editor.addEventListener("change", () => commit(editor.checked));
  } else if (t === "color") {
    const wrap = document.createElement("div");
    wrap.className = "prop-color";
    const swatch = document.createElement("input");
    swatch.type = "color";
    swatch.value = normalizeColor(v) || "#ECE9D8";
    const txt = document.createElement("input");
    txt.type = "text";
    txt.value = v ?? "";
    // Realtime preview: `input` fires while the picker is open / user drags
    // the slider; `change` fires only on commit. Update both the text input
    // and the live designer for every input event so the user sees the
    // colour change immediately.
    swatch.addEventListener("input",  () => { txt.value = swatch.value; commit(swatch.value); });
    swatch.addEventListener("change", () => { txt.value = swatch.value; commit(swatch.value); });
    txt.addEventListener("input",  () => { const c = normalizeColor(txt.value); if (c) { swatch.value = c; commit(txt.value); } });
    txt.addEventListener("change", () => { const c = normalizeColor(txt.value); if (c) swatch.value = c; commit(txt.value); });
    // Explicit "OK" / dismiss button — some platforms (notably macOS) show
    // the system colour picker without an OK button, so we provide our own
    // way to blur the swatch and close any open popover.
    const ok = document.createElement("button");
    ok.type = "button";
    ok.className = "prop-color-ok";
    ok.title = "Apply and close picker";
    ok.textContent = "OK";
    ok.addEventListener("click", () => {
      txt.value = swatch.value;
      commit(swatch.value);
      try { swatch.blur(); txt.blur(); } catch (_) {}
    });
    wrap.appendChild(swatch); wrap.appendChild(txt); wrap.appendChild(ok);
    valDiv.appendChild(wrap);
  } else if (t === "enum") {
    editor = document.createElement("select");
    for (const opt of ENUMS[k]) {
      const o = document.createElement("option");
      o.value = opt; o.textContent = opt;
      editor.appendChild(o);
    }
    editor.value = String(v);
    editor.addEventListener("change", () => commit(editor.value));
  } else if (t === "font") {
    editor = document.createElement("select");
    for (const f of FONT_FAMILIES) {
      const o = document.createElement("option");
      o.value = f; o.textContent = f;
      editor.appendChild(o);
    }
    if (!FONT_FAMILIES.includes(v)) {
      const o = document.createElement("option");
      o.value = v; o.textContent = v;
      editor.appendChild(o);
    }
    editor.value = String(v);
    // `change` already fires immediately for selects, but commit twice is safe
    // and ensures realtime preview if the browser also emits `input`.
    editor.addEventListener("input",  () => commit(editor.value));
    editor.addEventListener("change", () => commit(editor.value));
  } else if (t === "asset") {
    // Asset chooser: text input (free-form URL or "assets/foo.png") with a
    // companion <select> of project-bundled assets and an Upload button.
    const wrap = document.createElement("div");
    wrap.className = "prop-asset";
    const txt = document.createElement("input");
    txt.type = "text";
    txt.value = v ?? "";
    txt.placeholder = "URL or assets/<name>";
    const sel = document.createElement("select");
    sel.title = "Choose project asset";
    sel.innerHTML = `<option value="">(asset…)</option>` +
      (state.project.assets || []).map(a =>
        `<option value="assets/${escapeHtml(a.name)}">${escapeHtml(a.name)}</option>`).join("");
    sel.addEventListener("change", () => {
      if (sel.value) { txt.value = sel.value; commit(sel.value); sel.value = ""; }
    });
    txt.addEventListener("input",  () => commit(txt.value));
    txt.addEventListener("change", () => commit(txt.value));
    const up = document.createElement("button");
    up.type = "button"; up.textContent = "+";
    up.title = "Upload new asset";
    up.className = "prop-asset-up";
    up.addEventListener("click", () => doAssetUpload());
    wrap.appendChild(txt); wrap.appendChild(sel); wrap.appendChild(up);
    valDiv.appendChild(wrap);
  } else if (t === "number") {
    editor = document.createElement("input");
    editor.type = "number";
    editor.value = v ?? "";
    editor.addEventListener("change", () => commit(editor.value));
  } else {
    editor = document.createElement("input");
    editor.type = "text";
    editor.value = v ?? "";
    editor.addEventListener("change", () => commit(editor.value));
  }
  if (editor) valDiv.appendChild(editor);
  row.appendChild(valDiv);
  return row;
}

function normalizeColor(s) {
  if (!s) return null;
  s = String(s).trim();
  if (/^#[0-9a-f]{6}$/i.test(s)) return s.toLowerCase();
  if (/^#[0-9a-f]{3}$/i.test(s)) {
    return "#" + s.slice(1).split("").map(c => c+c).join("").toLowerCase();
  }
  return null;
}

function setupPropsToolbar() {
  document.querySelectorAll("#props-toolbar .ptab").forEach(btn => {
    btn.addEventListener("click", () => {
      document.querySelectorAll("#props-toolbar .ptab").forEach(b => b.classList.remove("active"));
      btn.classList.add("active");
      state.propsMode = btn.dataset.mode;
      renderProperties();
    });
  });
  const search = $("#props-search");
  if (search) search.addEventListener("input", () => renderProperties());
}

// ─── Form Layout dock ──────────────────────────────────────────

function updateLayoutDock() {
  const form = state.project.forms.find(f => f.id === state.activeFormId);
  const layoutForm = $("#layout-form");
  if (!form || !layoutForm) return;
  // Map a 1024x640 virtual screen onto the 160x90 mini-screen.
  const sw = 1024, sh = 640;
  const w = Math.max(8, Math.min(160, ((form.props.width  ?? 480) / sw) * 160));
  const h = Math.max(6, Math.min(90,  ((form.props.height ?? 320) / sh) * 90));
  const x = (form.props.startX ?? 100) / sw * 160;
  const y = (form.props.startY ?? 100) / sh * 90;
  layoutForm.style.width  = w + "px";
  layoutForm.style.height = h + "px";
  layoutForm.style.left   = x + "px";
  layoutForm.style.top    = y + "px";
}

// ─── Commands ─────────────────────────────────────────────────

async function runCommand(cmd) {
  switch (cmd) {
    case "project.new": {
      state.project = newProject("untitled");
      state.activeFormId = state.project.forms[0].id;
      state.selection = [];
      // Wipe existing tabs / panes.
      $("#mdi-tabs").innerHTML = "";
      $("#mdi-area").innerHTML = "";
      ensureFormPane(state.project.forms[0]);
      switchToForm(state.project.forms[0].id);
      renderProjectTree();
      renderProperties();
      setStatus("new project", "ok");
      return;
    }
    case "form.new": {
      const f = addForm(state.project);
      ensureFormPane(f);
      switchToForm(f.id);
      renderProjectTree();
      return;
    }
    case "form.delete": {
      if (state.activeFormId) closeForm(state.activeFormId);
      return;
    }
    case "view.code":     return switchView("code");
    case "view.designer": return switchView("designer");

    case "view.toggle.toolbox": return toggleDock("#toolbox", "84px");
    case "view.toggle.project": return toggleDock("#proj-dock");
    case "view.toggle.props":   return toggleDock("#props-dock");
    case "view.toggle.layout":  return toggleDock("#layout-dock");
    case "view.toggle.output":  return toggleDock("#output-dock", "180px");

    case "edit.delete": {
      const form = state.project.forms.find(f => f.id === state.activeFormId);
      if (!form || !state.selection.length) return;
      for (const n of state.selection) removeWidget(form, n);
      state.selection = [];
      renderActiveDesigner();
      renderProperties();
      return;
    }

    case "run.start":  return doRun();
    case "run.stop":   return doStop();
    case "run.build":  return doBuild();

    case "example.load": return $("#file-open-example").click();
    case "project.open": return $("#file-open-project").click();
    case "project.save": return doSaveProject();
    case "project.saveas": return doSaveProject();

    case "edit.cut":   return doClipboard("cut");
    case "edit.copy":  return doClipboard("copy");
    case "edit.paste": return doClipboard("paste");
    case "edit.selectall": {
      const form = state.project.forms.find(f => f.id === state.activeFormId);
      if (form) {
        state.selection = form.children.map(w => w.name);
        renderActiveDesigner(); renderProperties();
      }
      return;
    }
    case "edit.undo": case "edit.redo":
      logImmediate(`(undo/redo not yet implemented)`); return;

    case "module.new":   return doAddModule();
    case "project.props":
      logImmediate(`Project: ${state.project.name}, ${state.project.forms.length} forms, ${(state.project.modules||[]).length} modules`);
      return;
    case "asset.upload": return doAssetUpload();
    case "asset.manage": return showAssetManager();

    case "view.source":  return showFullSource();
    case "help.about":   return showAboutDialog();
    case "help.license": return showLicenseDialog();

    case "view.theme.light": return setTheme("light");
    case "view.theme.dark":  return setTheme("dark");
    case "view.theme.auto":  return setTheme(null);

    case "fmt.alignleft":  return alignSelection("left");
    case "fmt.alignright": return alignSelection("right");
    case "fmt.aligntop":   return alignSelection("top");
    case "fmt.alignbottom":return alignSelection("bottom");
    case "fmt.samewidth":  return sizeSelection("width");
    case "fmt.sameheight": return sizeSelection("height");
    case "fmt.center.h":   return centerSelection("h");
    case "fmt.center.v":   return centerSelection("v");
    case "fmt.tofront":    return zorderSelection("front");
    case "fmt.toback":     return zorderSelection("back");

    case "help.about":
      return showAboutDialog();

    default:
      logImmediate(`(stub) ${cmd}`);
  }
}

function toggleDock(sel, restore) {
  const el = $(sel);
  if (!el) return;
  el.style.display = el.style.display === "none" ? "" : "none";
}

// ─── Run / Build ───────────────────────────────────────────────

async function doRun() {
  if (!state.wasmReady) { setStatus("wasm not ready", "error"); return; }
  setStatus("compiling…");
  clearErrorsPanel();
  try {
    const src = serializeProject(state.project);
    logOutput("------ source ------\n" + src);
    const bc = compile(src, state.project.name);
    setStatus("running");
    $("#preview-window").hidden = false;
    $("#preview-title").textContent = `${state.project.name} — RapidR Runtime`;
    const iframe = $("#preview");
    // Wait for the preview iframe to announce __rapidr_preview_ready
    // (i.e. its top-level `await init()` has resolved); then ship the
    // bytecode. Avoids the load-vs-init race.
    const onReady = (e) => {
      if (e.source !== iframe.contentWindow) return;
      if (!e.data?.__rapidr_preview_ready) return;
      window.removeEventListener("message", onReady);
      hookPreviewConsole(iframe);
      iframe.contentWindow.postMessage({ __rapidr_run: bc }, "*");
    };
    window.addEventListener("message", onReady);
    iframe.src = "./preview.html?role=run";
  } catch (err) {
    setStatus("compile failed", "error");
    logOutput(String(err));
    $('.otab[data-tab="errors"]').click();
  }
}

function doStop() {
  const iframe = $("#preview");
  iframe.src = "about:blank";
  $("#preview-window").hidden = true;
  setStatus("stopped");
}

async function doBuild() {
  if (!state.wasmReady) { setStatus("wasm not ready", "error"); return; }
  try {
    const { buildBundleZip } = await import("./zip.js");
    const src = serializeProject(state.project);
    const rrbc = compile(src, state.project.name);
    const [jsText, wasmRes] = await Promise.all([
      fetch("./runtime/rapidrintr.js").then(r => r.text()),
      fetch("./runtime/rapidrintr_bg.wasm").then(r => r.arrayBuffer()),
    ]);
    const { bytes } = buildBundleZip({
      projectName: state.project.name,
      rrbc,
      rapidrintrJs: jsText,
      rapidrintrWasm: new Uint8Array(wasmRes),
      title: state.project.name,
      version: RAPIDR_IDE_VERSION,
      assets: (state.project.assets || []).map(a => ({ name: a.name, dataUrl: a.dataUrl })),
    });
    const url = URL.createObjectURL(new Blob([bytes], { type: "application/zip" }));
    const a = document.createElement("a");
    a.href = url; a.download = `${state.project.name}-web.zip`;
    a.click();
    URL.revokeObjectURL(url);
    setStatus("built " + a.download, "ok");
  } catch (err) {
    setStatus("build failed", "error");
    logOutput(String(err));
  }
}

async function doSaveProject() {
  const json = JSON.stringify(serializeProjectModel(state.project), null, 2);
  const url = URL.createObjectURL(new Blob([json], { type: "application/json" }));
  const a = document.createElement("a");
  a.href = url; a.download = `${state.project.name}.rrproj`;
  a.click();
  URL.revokeObjectURL(url);
  setStatus(`saved ${a.download}`, "ok");
}

// Capture only persistable parts of the in-memory model.
function serializeProjectModel(project) {
  return {
    rapidr_project: 1,
    name: project.name,
    modules: (project.modules || []).map(m => ({ id: m.id, name: m.name, source: m.source || "" })),
    assets: (project.assets || []).map(a => ({ name: a.name, mime: a.mime || "", dataUrl: a.dataUrl })),
    forms: project.forms.map(f => ({
      id: f.id,
      name: f.name,
      props: f.props,
      children: f.children.map(w => ({
        name: w.name, type: w.type, props: w.props,
        code: w.code || { handlers: {} },
      })),
      code: f.code || { handlers: {}, source: "" },
    })),
  };
}

function loadProjectModel(model) {
  if (!model || model.rapidr_project !== 1) throw new Error("not a .rrproj file");
  state.project = {
    name: model.name || "untitled",
    modules: (model.modules || []).map(m => ({ id: m.id || "m"+Date.now()+Math.random(), name: m.name, source: m.source || "" })),
    assets:  (model.assets  || []).map(a => ({ name: a.name, mime: a.mime || "", dataUrl: a.dataUrl })),
    forms: model.forms.map(f => ({
      id: f.id || crypto.randomUUID?.() || (Math.random()+"."+Date.now()),
      name: f.name,
      props: { ...f.props },
      children: (f.children || []).map(w => ({
        name: w.name, type: w.type,
        props: { ...w.props },
        code: w.code || { handlers: {} },
      })),
      code: f.code || { handlers: {}, source: "" },
    })),
  };
  state.selection = [];
  state.activeFormId = state.project.forms[0]?.id;
  $("#mdi-tabs").innerHTML = "";
  $("#mdi-area").innerHTML = "";
  for (const f of state.project.forms) ensureFormPane(f);
  for (const m of state.project.modules) ensureModulePane(m);
  if (state.activeFormId) switchToForm(state.activeFormId);
  renderProjectTree();
  renderProperties();
}

// ─── File loaders ──────────────────────────────────────────────

function setupFileLoaders() {
  const examplesSel = $("#examples");
  if (examplesSel) {
    examplesSel.addEventListener("change", async () => {
      const val = examplesSel.value;
      if (!val) return;
      try {
        const resp = await fetch(val);
        if (!resp.ok) throw new Error(`HTTP ${resp.status}`);
        const text = await resp.text();
        const name = val.split("/").pop().replace(/\.rr$/i, "");

        state.project = newProject(name);
        state.project.rawSource = text;
        $("#mdi-tabs").innerHTML = "";
        $("#mdi-area").innerHTML = "";
        ensureFormPane(state.project.forms[0]);
        switchToForm(state.project.forms[0].id);
        renderProjectTree();
        setStatus(`loaded ${name} (raw source mode)`, "ok");
      } catch (err) {
        setStatus("load example failed: " + err.message, "error");
        logOutput(String(err));
      }
    });
  }

  $("#file-open-example").addEventListener("change", async (e) => {
    const f = e.target.files?.[0];
    if (!f) return;
    const text = await f.text();
    // Phase A: dump the loaded .rr into a single-form project as raw source
    // (no parse-back into the model yet — that's Phase F).
    state.project = newProject(f.name.replace(/\.rr$/i, ""));
    state.project.rawSource = text;       // bypass serializer
    $("#mdi-tabs").innerHTML = "";
    $("#mdi-area").innerHTML = "";
    ensureFormPane(state.project.forms[0]);
    switchToForm(state.project.forms[0].id);
    renderProjectTree();
    setStatus(`loaded ${f.name} (raw source mode)`, "ok");
  });

  $("#file-open-project").addEventListener("change", async (e) => {
    const f = e.target.files?.[0];
    if (!f) return;
    try {
      const model = JSON.parse(await f.text());
      loadProjectModel(model);
      setStatus(`loaded ${f.name}`, "ok");
    } catch (err) {
      setStatus("load failed: " + err.message, "error");
      logOutput(String(err));
    }
  });

  $("#file-upload-asset").addEventListener("change", async (e) => {
    const files = Array.from(e.target.files || []);
    if (!files.length) return;
    state.project.assets = state.project.assets || [];
    let added = 0;
    for (const f of files) {
      const dataUrl = await new Promise((res, rej) => {
        const r = new FileReader();
        r.onload  = () => res(r.result);
        r.onerror = () => rej(r.error);
        r.readAsDataURL(f);
      });
      // Sanitise file name: strip path components, allow only [A-Za-z0-9._-].
      const safe = String(f.name).split(/[\\/]/).pop().replace(/[^A-Za-z0-9._-]/g, "_");
      // Ensure unique
      const taken = new Set(state.project.assets.map(a => a.name.toLowerCase()));
      let name = safe; let i = 1;
      while (taken.has(name.toLowerCase())) {
        const dot = safe.lastIndexOf(".");
        name = dot > 0 ? `${safe.slice(0, dot)}_${i}${safe.slice(dot)}` : `${safe}_${i}`;
        i++;
      }
      state.project.assets.push({ name, mime: f.type || "application/octet-stream", dataUrl });
      added++;
    }
    e.target.value = "";   // reset for next upload
    setStatus(`uploaded ${added} asset(s)`, "ok");
    renderProperties();   // refresh asset dropdowns
  });
}

// ─── Asset upload + manager ────────────────────────────────────

function doAssetUpload() {
  const inp = $("#file-upload-asset");
  if (inp) inp.click();
}

function showAssetManager() {
  const assets = state.project.assets || [];
  const rows = assets.length
    ? assets.map((a, i) => {
        const mime = a.mime || "";
        const isImg = /^image\//i.test(mime);
        const thumb = isImg
          ? `<img class="asset-thumb" src="${a.dataUrl}" alt="" />`
          : `<span class="asset-thumb asset-thumb-icon">${
              /^text\//i.test(mime) || /\.csv$|\.json$|\.txt$/i.test(a.name) ? "📄"
              : /^audio\//i.test(mime) ? "🎵"
              : /^video\//i.test(mime) ? "🎞"
              : "📦"}</span>`;
        return `<div class="asset-row" data-i="${i}">
           ${thumb}
           <span class="asset-name" title="${escapeHtml(mime)}">${escapeHtml(a.name)}</span>
           <span class="asset-mime">${escapeHtml(mime || "?")}</span>
           <span class="asset-size">${humanSize(a.dataUrl)}</span>
           <button data-act="del" data-i="${i}">Remove</button>
         </div>`;
      }).join("")
    : `<div class="asset-empty">No assets uploaded yet.</div>`;
  const html =
    `<div class="asset-list" style="max-height:50vh;overflow:auto">${rows}</div>
     <div style="margin-top:8px"><button id="asset-add-btn">Upload Asset…</button></div>`;
  showDialog("Project Assets", html, [
    { label: "Close", primary: true },
  ]);
  // Wire dialog buttons after insert.
  setTimeout(() => {
    const ov = document.querySelector(".ide-modal-overlay:last-of-type");
    if (!ov) return;
    ov.querySelector("#asset-add-btn")?.addEventListener("click", () => {
      ov.remove();
      doAssetUpload();
    });
    ov.querySelectorAll('button[data-act="del"]').forEach(b => {
      b.addEventListener("click", () => {
        const i = Number(b.dataset.i);
        state.project.assets.splice(i, 1);
        ov.remove();
        renderProperties();
        showAssetManager();
      });
    });
  }, 0);
}

function humanSize(dataUrl) {
  if (typeof dataUrl !== "string") return "?";
  const m = /^data:[^;,]*;base64,(.*)$/.exec(dataUrl);
  const bytes = m ? Math.floor(m[1].length * 3 / 4) : dataUrl.length;
  if (bytes < 1024) return bytes + " B";
  if (bytes < 1024 * 1024) return (bytes / 1024).toFixed(1) + " KB";
  return (bytes / (1024 * 1024)).toFixed(2) + " MB";
}

// ─── Clipboard / formatting / modules / dialogs ───────────────

let _clipboard = null;   // Array of widget snapshots {type, props}

function doClipboard(action) {
  const form = state.project.forms.find(f => f.id === state.activeFormId);
  if (!form) return;
  if (action === "copy" || action === "cut") {
    if (!state.selection.length) return;
    _clipboard = state.selection
      .map(n => findWidget(form, n))
      .filter(Boolean)
      .map(w => ({ type: w.type, props: { ...w.props } }));
    if (action === "cut") {
      for (const n of state.selection) removeWidget(form, n);
      state.selection = [];
      renderActiveDesigner();
      renderProperties();
    }
    setStatus(`${action} ${_clipboard.length} widget(s)`, "ok");
    return;
  }
  if (action === "paste") {
    if (!_clipboard?.length) { setStatus("clipboard empty"); return; }
    const newNames = [];
    for (const snap of _clipboard) {
      const geom = {
        left:   (snap.props.left   ?? 0) + 16,
        top:    (snap.props.top    ?? 0) + 16,
        width:  snap.props.width,
        height: snap.props.height,
      };
      const w = addWidget(form, snap.type, geom);
      // Copy other props too (caption, font, color…) without overwriting name/geom.
      for (const [k, v] of Object.entries(snap.props)) {
        if (["left","top","width","height"].includes(k)) continue;
        if (k === "caption" || k === "text") continue;   // keep auto-generated name as caption
        w.props[k] = v;
      }
      newNames.push(w.name);
    }
    state.selection = newNames;
    renderActiveDesigner();
    renderProperties();
    setStatus(`pasted ${newNames.length} widget(s)`, "ok");
  }
}

function alignSelection(edge) {
  const form = state.project.forms.find(f => f.id === state.activeFormId);
  if (!form || state.selection.length < 2) return;
  const ws = state.selection.map(n => findWidget(form, n)).filter(Boolean);
  const anchor = ws[0];
  for (let i = 1; i < ws.length; i++) {
    const w = ws[i];
    if (edge === "left")   w.props.left = anchor.props.left;
    if (edge === "right")  w.props.left = (anchor.props.left + (anchor.props.width||0)) - (w.props.width||0);
    if (edge === "top")    w.props.top  = anchor.props.top;
    if (edge === "bottom") w.props.top  = (anchor.props.top  + (anchor.props.height||0)) - (w.props.height||0);
  }
  renderActiveDesigner(); renderProperties();
}
function sizeSelection(dim) {
  const form = state.project.forms.find(f => f.id === state.activeFormId);
  if (!form || state.selection.length < 2) return;
  const ws = state.selection.map(n => findWidget(form, n)).filter(Boolean);
  const v = ws[0].props[dim];
  for (let i = 1; i < ws.length; i++) ws[i].props[dim] = v;
  renderActiveDesigner(); renderProperties();
}
function centerSelection(axis) {
  const form = state.project.forms.find(f => f.id === state.activeFormId);
  if (!form) return;
  const fw = form.props.width || 480, fh = form.props.height || 320;
  for (const n of state.selection) {
    const w = findWidget(form, n); if (!w) continue;
    if (axis === "h") w.props.left = Math.max(0, Math.round((fw - (w.props.width||0)) / 2));
    if (axis === "v") w.props.top  = Math.max(0, Math.round((fh - (w.props.height||0)) / 2));
  }
  renderActiveDesigner(); renderProperties();
}
function zorderSelection(dir) {
  const form = state.project.forms.find(f => f.id === state.activeFormId);
  if (!form) return;
  const sel = new Set(state.selection);
  const stay = form.children.filter(w => !sel.has(w.name));
  const moving = form.children.filter(w => sel.has(w.name));
  form.children = (dir === "front") ? [...stay, ...moving] : [...moving, ...stay];
  renderActiveDesigner();
}

function doAddModule() {
  const def = `Module${(state.project.modules?.length || 0) + 1}`;
  promptDialog("New module", "Module name:", def, (nm) => {
    if (!nm) return;
    if (!/^[A-Za-z_][A-Za-z0-9_]*$/.test(nm)) { showDialog("New module", "<p>Invalid identifier.</p>", [{label:"OK",primary:true}]); return; }
    state.project.modules ||= [];
    if (state.project.modules.some(m => m.name === nm)) { showDialog("New module", "<p>Name already in use.</p>", [{label:"OK",primary:true}]); return; }
    const mod = {
      id: "m" + Date.now(),
      name: nm,
      source: `' Module ${nm}\n' Declare GLOBAL variables and shared SUBs / FUNCTIONs here.\n\nGLOBAL g_${nm} AS Integer\n\n`,
    };
    state.project.modules.push(mod);
    ensureModulePane(mod);
    switchToModule(mod.id);
    renderProjectTree();
  });
}

function ensureModulePane(mod) {
  let tab = document.querySelector(`#mdi-tabs .mtab[data-mod="${mod.id}"]`);
  if (!tab) {
    tab = document.createElement("div");
    tab.className = "mtab";
    tab.dataset.mod = mod.id;
    tab.innerHTML = `<span class="mtab-name">${escapeHtml(mod.name)} [Module]</span><span class="x" title="Close">×</span>`;
    tab.addEventListener("click", (e) => {
      if (e.target.classList.contains("x")) closeModule(mod.id);
      else switchToModule(mod.id);
    });
    $("#mdi-tabs").appendChild(tab);
  }
  let pane = document.querySelector(`.mdi-pane[data-mod="${mod.id}"]`);
  if (!pane) {
    pane = document.createElement("div");
    pane.className = "mdi-pane";
    pane.dataset.mod = mod.id;
    pane.innerHTML = `<div class="code-pane" data-role="code"><div class="monaco-host"></div></div>`;
    $("#mdi-area").appendChild(pane);
  }
}

async function switchToModule(modId) {
  const mod = state.project.modules.find(m => m.id === modId);
  if (!mod) return;
  // Re-create the tab/pane if it was closed previously.
  ensureModulePane(mod);
  state.activeFormId = null;
  $$(".mtab").forEach(t => t.classList.toggle("active", t.dataset.mod === modId));
  $$(".mdi-pane").forEach(p => p.classList.toggle("active", p.dataset.mod === modId));
  const pane = document.querySelector(`.mdi-pane[data-mod="${modId}"]`);
  const host = pane.querySelector(".monaco-host");
  if (!_editors.has(modId)) {
    const ed = await createRapidrEditor(host, {
      value: mod.source,
      onChange: (txt) => { mod.source = txt; },
    });
    _editors.set(modId, ed);
  } else {
    _editors.get(modId).layout();
  }
}
// Close a module TAB only (keeps the module in the project).
function closeModule(modId) {
  document.querySelector(`#mdi-tabs .mtab[data-mod="${modId}"]`)?.remove();
  document.querySelector(`.mdi-pane[data-mod="${modId}"]`)?.remove();
  if (_editors.has(modId)) { _editors.get(modId).dispose(); _editors.delete(modId); }
  if (state.project.forms.length) switchToForm(state.project.forms[0].id);
  renderProjectTree();
}

// Permanently delete a module from the project (called from tree row ×).
function deleteModulePermanent(modId) {
  state.project.modules = (state.project.modules || []).filter(m => m.id !== modId);
  closeModule(modId);
}

function showFullSource() {
  const src = serializeProject(state.project);
  showDialog("Full source — " + state.project.name,
    `<pre style="margin:0;font-family:var(--font-mono);font-size:12px;white-space:pre;overflow:auto;max-height:60vh">${escapeHtml(src)}</pre>`,
    [{ label: "Copy", onClick: () => navigator.clipboard?.writeText(src).then(()=>setStatus("copied source","ok")) },
     { label: "Close", primary: true }]);
}

function showAboutDialog() {
  const html = `
    <div style="display:flex;gap:14px;align-items:flex-start">
      <div style="font-size:48px;line-height:1">🚀</div>
      <div>
        <div style="font-size:16px;font-weight:600">RapidR IDE <span style="color:var(--c-text-mute);font-weight:400;font-size:12px">v${escapeHtml(RAPIDR_IDE_VERSION)}</span></div>
        <div style="margin-top:2px">Self-hosted, zero-backend, in-browser BASIC IDE</div>
        <div style="margin-top:8px"><b>Author:</b> Roberto Berrospe</div>
        <div><b>Assisted by:</b> Claude (Anthropic)</div>
        <div style="margin-top:8px"><b>License:</b> see LICENSE (MIT)</div>
        <div style="margin-top:6px;color:var(--c-text-mute);font-size:11px">
          Compiles in WebAssembly via <code>rapidrintr.wasm</code>.
        </div>
      </div>
    </div>`;
  showDialog("About RapidR IDE", html, [
    { label: "View License", onClick: showLicenseDialog },
    { label: "OK", primary: true },
  ]);
}

async function showLicenseDialog() {
  let txt = "(LICENSE not found)";
  try {
    const r = await fetch("../LICENSE");
    if (r.ok) txt = await r.text();
  } catch (_) {}
  showDialog("License",
    `<pre style="margin:0;font-family:var(--font-mono);font-size:11px;white-space:pre-wrap;max-height:60vh;overflow:auto">${escapeHtml(txt)}</pre>`,
    [{ label: "Close", primary: true }]);
}

function escapeHtml(s) {
  return String(s).replace(/[&<>]/g, c => ({"&":"&amp;","<":"&lt;",">":"&gt;"}[c]));
}

// Lightweight in-IDE prompt (replacement for window.prompt which is
// silently blocked in some embedding contexts).
function promptDialog(title, label, defaultValue, onOk) {
  const id = "prompt-" + Math.random().toString(36).slice(2);
  const html = `
    <div style="display:flex;flex-direction:column;gap:6px;min-width:280px">
      <label for="${id}" style="font-size:11px;color:var(--c-text-mute,#666)">${escapeHtml(label)}</label>
      <input id="${id}" type="text" value="${escapeHtml(defaultValue ?? "")}"
             style="padding:6px 8px;font:inherit;border:1px solid var(--c-border,#999);border-radius:3px">
    </div>`;
  showDialog(title, html, [
    { label: "Cancel" },
    { label: "OK", primary: true, onClick: () => {
        const el = document.getElementById(id);
        const v = el ? el.value.trim() : "";
        try { onOk(v); } catch (e) { console.error(e); }
      } },
  ]);
  // Focus + Enter-to-confirm.
  setTimeout(() => {
    const el = document.getElementById(id);
    if (!el) return;
    el.focus(); el.select();
    el.addEventListener("keydown", (e) => {
      if (e.key === "Enter") {
        e.preventDefault();
        const ov = el.closest(".ide-modal-overlay");
        const okBtn = ov?.querySelector(".ide-modal-btns button.primary");
        okBtn?.click();
      }
    });
  }, 0);
}

// Lightweight modal dialog (no third-party deps).
function showDialog(title, bodyHtml, buttons) {
  const ov = document.createElement("div"); ov.className = "ide-modal-overlay";
  const dlg = document.createElement("div"); dlg.className = "ide-modal";
  dlg.innerHTML = `
    <div class="ide-modal-tb"><span>${escapeHtml(title)}</span><span class="x" title="Close">×</span></div>
    <div class="ide-modal-body"></div>
    <div class="ide-modal-btns"></div>`;
  dlg.querySelector(".ide-modal-body").innerHTML = bodyHtml;
  const btnRow = dlg.querySelector(".ide-modal-btns");
  for (const b of (buttons || [{label:"OK",primary:true}])) {
    const el = document.createElement("button");
    el.textContent = b.label;
    if (b.primary) el.className = "primary";
    el.addEventListener("click", () => {
      if (b.onClick) b.onClick();
      if (b.keepOpen) return;
      ov.remove();
    });
    btnRow.appendChild(el);
  }
  const close = () => ov.remove();
  dlg.querySelector(".x").addEventListener("click", close);
  ov.addEventListener("click", (e) => { if (e.target === ov) close(); });
  ov.appendChild(dlg);
  document.body.appendChild(ov);
}

// Form Layout dock — drag the mini form to set startup X/Y.
function setupLayoutDock() {
  const screen = $("#layout-screen");
  const layoutForm = $("#layout-form");
  if (!screen || !layoutForm) return;
  let drag = null;
  layoutForm.addEventListener("mousedown", (e) => {
    const sr = screen.getBoundingClientRect();
    const fr = layoutForm.getBoundingClientRect();
    drag = {
      ox: e.clientX - fr.left,
      oy: e.clientY - fr.top,
      sr,
    };
    e.preventDefault();
  });
  window.addEventListener("mousemove", (e) => {
    if (!drag) return;
    const x = Math.max(0, Math.min(drag.sr.width  - layoutForm.offsetWidth,  e.clientX - drag.sr.left - drag.ox));
    const y = Math.max(0, Math.min(drag.sr.height - layoutForm.offsetHeight, e.clientY - drag.sr.top  - drag.oy));
    layoutForm.style.left = x + "px";
    layoutForm.style.top  = y + "px";
    // Map back to virtual 1024x640 screen.
    const form = state.project.forms.find(f => f.id === state.activeFormId);
    if (form) {
      form.props.startX = Math.round(x / 160 * 1024);
      form.props.startY = Math.round(y /  90 *  640);
      renderProperties();
    }
  });
  window.addEventListener("mouseup", () => { drag = null; });
}

// ─── Boot ──────────────────────────────────────────────────────
function setupPreviewWindow() {
  $("#preview-close").addEventListener("click", doStop);
  const tb = $("#preview-titlebar");
  const win = $("#preview-window");
  let drag = null;
  tb.addEventListener("mousedown", (e) => {
    if (e.target.id === "preview-close") return;
    const rect = win.getBoundingClientRect();
    drag = { ox: e.clientX - rect.left, oy: e.clientY - rect.top };
    e.preventDefault();
  });
  window.addEventListener("mousemove", (e) => {
    if (!drag) return;
    win.style.left = (e.clientX - drag.ox) + "px";
    win.style.top  = (e.clientY - drag.oy) + "px";
  });
  window.addEventListener("mouseup", () => { drag = null; });
}

function setupKeyboard() {
  window.addEventListener("keydown", (e) => {
    // Don't hijack typing in inputs or the Monaco editor.
    const t = e.target;
    if (t.matches?.("input, textarea, select")) return;
    if (t.closest?.(".monaco-editor")) return;

    if ((e.key === "Delete" || e.key === "Backspace") && state.activeView === "designer" && state.selection.length) {
      e.preventDefault();
      runCommand("edit.delete");
      return;
    }
    if (e.key === "F5") { e.preventDefault(); runCommand(e.shiftKey ? "run.stop" : "run.start"); return; }
    if (e.key === "F7") { e.preventDefault(); runCommand("view.code"); return; }
    if ((e.key === "F7") && e.shiftKey) { e.preventDefault(); runCommand("view.designer"); return; }
    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "s") {
      e.preventDefault(); runCommand("project.save"); return;
    }
    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "n") {
      e.preventDefault(); runCommand("project.new"); return;
    }
    if (e.key === "Escape" && state.selectedTool !== "pointer") {
      e.preventDefault(); armTool("pointer"); renderActiveDesigner();
    }
  });
}
async function main() {
  // Test hook: expose key state + commands so headless tests can drive the IDE.
  window.RapidR = {
    version: RAPIDR_IDE_VERSION,
    state,
    runCommand,
    renderActiveDesigner,
    renderProperties,
    renderProjectTree,
    switchView,
    switchToForm,
    serializeProject,
  };
  // Show version in the status bar + window title.
  const ver = $("#ide-version");
  if (ver) ver.textContent = "v" + RAPIDR_IDE_VERSION;
  document.title = `RapidR IDE v${RAPIDR_IDE_VERSION}`;
  setupMenus();
  setupOutputTabs();
  setupToolbox();
  setupFileLoaders();
  setupPreviewWindow();
  setupKeyboard();
  setupPropsToolbar();
  setupLayoutDock();

  // Initialize the wasm runtime (compile + interpreter).
  setStatus("loading runtime…");
  try {
    await init();
    state.wasmReady = true;
    setStatus("ready", "ok");
  } catch (err) {
    setStatus("runtime failed: " + err, "error");
    return;
  }

  // Boot a default project.
  state.activeFormId = state.project.forms[0].id;
  ensureFormPane(state.project.forms[0]);
  switchToForm(state.project.forms[0].id);
  renderProjectTree();
  renderProperties();

  // Receive log/status messages from preview iframe.
  window.addEventListener("message", (e) => {
    const d = e.data || {};
    if (d.__rapidr_log)    logOutput(d.__rapidr_log);
    if (d.__rapidr_status) setStatus(d.__rapidr_status);
  });
}

main();
