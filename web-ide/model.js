// Project / form / widget model + .rr serializer.
//
// Project shape:
//   { name, forms: [Form], modules: [Module], startupForm: id, rawSource?: string }
// Form shape:
//   { id, name, props: {...form props}, children: [Widget], code: { handlers: {key→source} } }
// Widget shape:
//   { name, type, props: {...widget props}, code: { handlers: {key→source} } }

import { defaultsFor, normalizeTypeCasing } from "./toolbox.js";

let _idCounter = 1;
const newId = () => `f${_idCounter++}`;

// ─── Constructors ───────────────────────────────────────────────

export function newProject(name = "untitled") {
  const f = newForm("Form1", { isStartup: true });
  return {
    name,
    forms: [f],
    modules: [],
    assets: [],   // [{ name, mime, dataUrl }]
    startupForm: f.id,
  };
}

export function newForm(name, opts = {}) {
  return {
    id: newId(),
    name,
    props: {
      caption: name,
      width: 480,
      height: 320,
      startX: 100,
      startY: 100,
      borderstyle: 2,        // sizable
      ...opts.props,
    },
    children: [],
    code: { handlers: {} },
  };
}

export function addForm(project, baseName = "Form") {
  const n = nextUnique(project.forms.map(f => f.name), baseName);
  const f = newForm(n);
  project.forms.push(f);
  return f;
}

export function addWidget(form, type, geom = {}) {
  const baseName = nameFor(type);
  const n = nextUnique(allNames(form), baseName);
  const props = { ...defaultsFor(type, n), ...geom };
  const w = { name: n, type, props, code: { handlers: {} } };
  form.children.push(w);
  return w;
}

export function removeWidget(form, name) {
  form.children = form.children.filter(w => w.name !== name);
}

export function findWidget(form, name) {
  return form.children.find(w => w.name === name) || null;
}

export function allWidgets(form) { return form.children.slice(); }

export function setProp(form, widgetName, key, value) {
  const w = findWidget(form, widgetName);
  if (!w) return;
  w.props[key] = value;
}

// ─── Naming helpers ─────────────────────────────────────────────

function nameFor(type) {
  // RButton → Button1, RLabel → Label1, etc.
  const base = type.startsWith("R") ? type.slice(1) : type;
  return base + "1";
}

function nextUnique(existing, baseName) {
  // Strip trailing digits from baseName, then find next free index ≥ 1.
  const root = baseName.replace(/\d+$/, "") || "Item";
  const taken = new Set(existing.map(s => s.toLowerCase()));
  for (let i = 1; i < 10000; i++) {
    const candidate = `${root}${i}`;
    if (!taken.has(candidate.toLowerCase())) return candidate;
  }
  return `${root}${Date.now()}`;
}

function allNames(form) {
  return [form.name, ...form.children.map(w => w.name)];
}

// ─── Serializer ────────────────────────────────────────────────

const STR_PROPS = new Set([
  "caption", "text", "items", "picture", "fontname", "tooltip", "hint",
  "url", "alignment",
]);

function emitVal(key, v) {
  if (typeof v === "number") return String(v);
  if (typeof v === "boolean") return v ? "1" : "0";
  if (STR_PROPS.has(key.toLowerCase())) {
    return `"${String(v).replace(/"/g, '""')}"`;
  }
  if (typeof v === "string" && /^-?\d+(\.\d+)?$/.test(v)) return v;
  return `"${String(v).replace(/"/g, '""')}"`;
}

function emitProps(props, indent) {
  const out = [];
  for (const [k, v] of Object.entries(props)) {
    if (v === null || v === undefined) continue;
    if (k === "isStartup") continue;
    out.push(`${indent}${capitalize(k)} = ${emitVal(k, v)}`);
  }
  return out.join("\n");
}

function emitWidget(w, indent) {
  const inner = emitProps(w.props, indent + "    ");
  return `${indent}CREATE ${w.name} AS ${w.type}\n${inner}\n${indent}END CREATE`;
}

export function serializeForm(form) {
  const inner = emitProps(form.props, "    ");
  const kids  = form.children.map(w => emitWidget(w, "    ")).join("\n");
  return `CREATE ${form.name} AS RForm\n${inner}${kids ? "\n" + kids : ""}\nEND CREATE`;
}

export function serializeProject(project) {
  if (project.rawSource) return project.rawSource;

  const lines = ["$APPTYPE WEB", ""];

  // Module sources first (globals + shared SUBs/FUNCTIONs need to be visible).
  for (const m of (project.modules || [])) {
    const src = (m.source || "").trim();
    if (src) {
      lines.push(`' --- module ${m.name} ---`);
      lines.push(src);
      lines.push("");
    }
  }

  // Per-form CREATE blocks.
  for (const f of project.forms) {
    lines.push(serializeForm(f));
    lines.push("");
  }

  // Event-handler bindings — for both form-level and widget-level handlers.
  // form.code.handlers / widget.code.handlers shape: { OnClick: "Button1_Click", ... }
  const bindings = [];
  for (const f of project.forms) {
    for (const [evt, sub] of Object.entries(f.code?.handlers || {})) {
      bindings.push(`${f.name}.${evt} = ${sub}`);
    }
    for (const w of f.children) {
      for (const [evt, sub] of Object.entries(w.code?.handlers || {})) {
        bindings.push(`${w.name}.${evt} = ${sub}`);
      }
    }
  }
  if (bindings.length) { lines.push(...bindings, ""); }

  // Show the startup form.
  const start = project.forms.find(f => f.id === project.startupForm) || project.forms[0];
  if (start) lines.push(`${start.name}.ShowModal`, "");

  // Append the user-authored code-behind source for each form.
  for (const f of project.forms) {
    const src = (f.code?.source || "").trim();
    if (src) lines.push(src, "");
  }

  return lines.join("\n") + "\n";
}

function capitalize(s) { return s ? s[0].toUpperCase() + s.slice(1) : s; }

export function deserializeProject(text, projectName = "untitled") {
  const lines = text.split(/\r?\n/);
  const project = {
    rapidr_project: 1,
    name: projectName,
    forms: [],
    modules: [],
    assets: [],
    startupForm: null,
  };

  let currentForm = null;
  let currentWidget = null;
  let createStack = []; // Stack of { name, type, obj, isForm }
  let freeCodeLines = [];
  let inSub = false;
  let currentSubLines = [];

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    const trimmed = line.trim();
    const upper = trimmed.toUpperCase();

    // Handle Sub blocks
    if (upper.startsWith("SUB ") || upper.startsWith("FUNCTION ")) {
      inSub = true;
      currentSubLines.push(line);
      continue;
    }
    if (inSub) {
      currentSubLines.push(line);
      if (upper === "END SUB" || upper === "END FUNCTION") {
        inSub = false;
        freeCodeLines.push(...currentSubLines);
        currentSubLines = [];
      }
      continue;
    }

    // Parse CREATE statements
    if (upper.startsWith("CREATE ")) {
      const match = trimmed.match(/^CREATE\s+(\w+)\s+AS\s+(\w+)/i);
      if (match) {
        const cName = match[1];
        const cType = normalizeTypeCasing(match[2]);
        const isForm = cType.toUpperCase() === "RFORM";

        if (isForm) {
          const form = {
            id: `f_${cName.toLowerCase()}`,
            name: cName,
            props: {},
            children: [],
            code: { handlers: {}, source: "" }
          };
          project.forms.push(form);
          currentForm = form;
          createStack.push({ name: cName, type: cType, obj: form, isForm: true });
        } else {
          // Widget
          const parentObj = createStack[createStack.length - 1];
          const widget = {
            name: cName,
            type: cType,
            props: {},
            code: { handlers: {} }
          };
          if (parentObj && !parentObj.isForm) {
            widget.props.parent = parentObj.name;
          }
          if (currentForm) {
            currentForm.children.push(widget);
          }
          currentWidget = widget;
          createStack.push({ name: cName, type: cType, obj: widget, isForm: false });
        }
      }
      continue;
    }

    // Parse END CREATE
    if (upper === "END CREATE") {
      createStack.pop();
      const top = createStack[createStack.length - 1];
      if (top) {
        if (top.isForm) {
          currentForm = top.obj;
          currentWidget = null;
        } else {
          currentWidget = top.obj;
        }
      } else {
        currentForm = null;
        currentWidget = null;
      }
      continue;
    }

    // Parse properties inside CREATE block
    if (createStack.length > 0) {
      const match = trimmed.match(/^(\w+)\s*=\s*(.*)/);
      if (match) {
        const propName = match[1].toLowerCase();
        let propValRaw = match[2].trim();
        
        let propVal = propValRaw;
        if (propValRaw.startsWith('"') && propValRaw.endsWith('"')) {
          propVal = propValRaw.slice(1, -1).replace(/""/g, '"');
        } else if (propValRaw === "1") {
          propVal = 1;
        } else if (propValRaw === "0") {
          propVal = 0;
        } else if (/^-?\d+$/.test(propValRaw)) {
          propVal = parseInt(propValRaw, 10);
        } else if (/^-?\d+\.\d+$/.test(propValRaw)) {
          propVal = parseFloat(propValRaw);
        }

        const top = createStack[createStack.length - 1];
        if (propName.startsWith("on")) {
          top.obj.code = top.obj.code || {};
          top.obj.code.handlers = top.obj.code.handlers || {};
          top.obj.code.handlers[capitalize(propName)] = String(propVal);
        } else {
          top.obj.props[propName] = propVal;
        }
      } else if (trimmed.toUpperCase() === "CENTER") {
        const top = createStack[createStack.length - 1];
        top.obj.props.center = true;
      }
      continue;
    }

    // Outside CREATE: event bindings like Widget.OnClick = Handler
    const matchEvent = trimmed.match(/^(\w+)\.(\w+)\s*=\s*(\w+)/);
    if (matchEvent) {
      const compName = matchEvent[1];
      const eventName = capitalize(matchEvent[2].toLowerCase());
      const handlerName = matchEvent[3];
      
      let found = false;
      for (const f of project.forms) {
        if (f.name.toLowerCase() === compName.toLowerCase()) {
          f.code.handlers[eventName] = handlerName;
          found = true;
          break;
        }
        const w = f.children.find(c => c.name.toLowerCase() === compName.toLowerCase());
        if (w) {
          w.code.handlers[eventName] = handlerName;
          found = true;
          break;
        }
      }
      if (found) continue;
    }

    // Capture freestanding code lines
    if (!upper.startsWith("$APPTYPE")) {
      freeCodeLines.push(line);
    }
  }

  if (project.forms.length > 0) {
    project.startupForm = project.forms[0].id;
    project.forms[0].code.source = freeCodeLines.join("\n");
  }

  return project;
}
