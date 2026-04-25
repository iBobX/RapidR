// Lazy-loaded Monaco editor wrapper, vendored under ./vendor/monaco/vs.
// Exposes:
//   loadMonaco()                  — Promise<monaco>, idempotent
//   createRapidrEditor(host, opts) — Promise<editor>
//
// Custom "rapidr" language: BASIC-flavoured tokens + full IntelliSense
// powered by ./lang-data.js (mirrors the VSCode extension data).

import {
  COMPONENT_REGISTRY,
  BUILTIN_FUNCTIONS,
  KEYWORDS,
  TYPE_KEYWORDS,
  DIRECTIVES,
  prettyComponentName,
  resolveVariableType,
} from "./lang-data.js";

let _monacoPromise = null;

export function loadMonaco() {
  if (_monacoPromise) return _monacoPromise;
  _monacoPromise = new Promise((resolve, reject) => {
    const s = document.createElement("script");
    s.src = "./vendor/monaco/vs/loader.js";
    s.onload = () => {
      const vsBase = new URL("./vendor/monaco/vs/", document.baseURI).toString();
      window.MonacoEnvironment = {
        getWorkerUrl: function () {
          return URL.createObjectURL(new Blob([
            `self.MonacoEnvironment = { baseUrl: ${JSON.stringify(vsBase)} };` +
            `importScripts(${JSON.stringify(vsBase + "base/worker/workerMain.js")});`,
          ], { type: "text/javascript" }));
        },
      };
      const req = window.require;
      req.config({ paths: { vs: vsBase.replace(/\/$/, "") } });
      req(["vs/editor/editor.main"], () => {
        const monaco = window.monaco;
        registerRapidrLanguage(monaco);
        resolve(monaco);
      });
    };
    s.onerror = (e) => reject(new Error("monaco loader failed: " + e?.message));
    document.head.appendChild(s);
  });
  return _monacoPromise;
}

// Combined text from all forms+modules in the project (so cross-tab
// type resolution can work even when editing a different file).
function gatherProjectText(currentText) {
  const parts = [currentText || ""];
  const proj = window.RapidR?.state?.project;
  if (proj) {
    for (const f of (proj.forms || [])) {
      for (const w of (f.children || [])) {
        // Synthesize a CREATE block so resolveVariableType picks up the type.
        parts.push(`CREATE ${w.name} AS ${w.type}`);
      }
      if (f.code?.source) parts.push(f.code.source);
      for (const ev of Object.values(f.code?.handlers || {})) {
        if (ev?.body) parts.push(ev.body);
      }
    }
    for (const m of (proj.modules || [])) {
      if (m.source) parts.push(m.source);
    }
  }
  return parts.join("\n");
}

function registerRapidrLanguage(monaco) {
  if (monaco.languages.getLanguages().some(l => l.id === "rapidr")) return;
  monaco.languages.register({ id: "rapidr", extensions: [".rr"], aliases: ["RapidR", "rapidr"] });

  const monarchKeywords = KEYWORDS.flatMap(k => k.toLowerCase().split(/\s+/));
  const monarchTypes = [
    ...TYPE_KEYWORDS.map(t => t.name.toLowerCase()),
    ...Object.keys(COMPONENT_REGISTRY).map(c => c.toLowerCase()),
  ];

  monaco.languages.setMonarchTokensProvider("rapidr", {
    defaultToken: "",
    ignoreCase: true,
    keywords: monarchKeywords,
    typeKeywords: monarchTypes,
    operators: ["=","<",">","<=",">=","<>","+","-","*","/","\\","^","&"],
    symbols: /[=<>!~?:&|+\-*\/\^%]+/,
    tokenizer: {
      root: [
        [/'.*$/, "comment"],
        [/REM\b.*$/i, "comment"],
        [/"([^"]|"")*"/, "string"],
        [/\b\d+\.\d+\b/, "number.float"],
        [/\b\d+\b/, "number"],
        [/\$[A-Za-z_]\w*/, "keyword.directive"],
        [/[A-Za-z_]\w*/, {
          cases: {
            "@keywords":     "keyword",
            "@typeKeywords": "type",
            "@default":      "identifier",
          },
        }],
        [/@symbols/, {
          cases: { "@operators": "operator", "@default": "" },
        }],
      ],
    },
  });

  monaco.languages.setLanguageConfiguration("rapidr", {
    comments: { lineComment: "'" },
    brackets: [["(", ")"]],
    autoClosingPairs: [
      { open: "(", close: ")" },
      { open: '"', close: '"' },
    ],
  });

  // Themes — VB6-ish chrome (light) + dark.
  monaco.editor.defineTheme("rapidr-vb6", {
    base: "vs",
    inherit: true,
    rules: [
      { token: "comment",            foreground: "008000", fontStyle: "italic" },
      { token: "keyword",            foreground: "0000FF" },
      { token: "keyword.directive",  foreground: "AF00DB" },
      { token: "type",               foreground: "267F99" },
      { token: "string",             foreground: "A31515" },
      { token: "number",             foreground: "098658" },
    ],
    colors: { "editor.background": "#FFFFFF" },
  });
  monaco.editor.defineTheme("rapidr-dark", {
    base: "vs-dark",
    inherit: true,
    rules: [
      { token: "comment",            foreground: "6A9955", fontStyle: "italic" },
      { token: "keyword",            foreground: "569CD6" },
      { token: "keyword.directive",  foreground: "C586C0" },
      { token: "type",               foreground: "4EC9B0" },
      { token: "string",             foreground: "CE9178" },
      { token: "number",             foreground: "B5CEA8" },
    ],
    colors: { "editor.background": "#1E1E1E" },
  });

  // ── Completion provider ────────────────────────────────────────
  monaco.languages.registerCompletionItemProvider("rapidr", {
    triggerCharacters: [".", "$"],
    provideCompletionItems: (model, position) => {
      const lineText = model.getLineContent(position.lineNumber);
      const linePrefix = lineText.substring(0, position.column - 1);
      const word = model.getWordUntilPosition(position);
      const range = {
        startLineNumber: position.lineNumber,
        endLineNumber:   position.lineNumber,
        startColumn:     word.startColumn,
        endColumn:       word.endColumn,
      };
      const sk = monaco.languages.CompletionItemKind;
      // CompletionItemInsertTextRules.InsertAsSnippet === 4 (Monaco constant; safe to hard-code)
      const insertSnippet = (monaco.languages.CompletionItemInsertTextRules && monaco.languages.CompletionItemInsertTextRules.InsertAsSnippet) || 4;

      // — `Identifier.` member completion
      const dot = linePrefix.match(/(\w+)\.\s*$/);
      if (dot) {
        const varName = dot[1];
        const text = gatherProjectText(model.getValue());
        // Resolve to a component type. Try direct registry hit (e.g. someone typed "RButton.").
        let typeName = COMPONENT_REGISTRY[varName.toUpperCase()] ? varName.toUpperCase()
                     : resolveVariableType(text, varName);
        const reg = typeName ? COMPONENT_REGISTRY[typeName] : null;
        if (!reg) return { suggestions: [] };

        const dotRange = {
          startLineNumber: position.lineNumber,
          endLineNumber:   position.lineNumber,
          startColumn:     position.column,
          endColumn:       position.column,
        };
        const suggestions = [];
        for (const p of reg.props) {
          suggestions.push({
            label: p, kind: sk.Property,
            insertText: p, range: dotRange,
            detail: `(property) ${prettyComponentName(typeName)}.${p}`,
            sortText: "0" + p,
          });
        }
        for (const m of reg.methods) {
          const sigInfo = reg.methodSignatures?.[m];
          suggestions.push({
            label: m, kind: sk.Method,
            insertText: m + "($0)", insertTextRules: insertSnippet,
            range: dotRange,
            detail: sigInfo?.sig ? `(method) ${sigInfo.sig}` : `(method) ${prettyComponentName(typeName)}.${m}()`,
            documentation: sigInfo?.desc,
            sortText: "1" + m,
          });
        }
        for (const ev of reg.events) {
          suggestions.push({
            label: ev, kind: sk.Event,
            insertText: ev, range: dotRange,
            detail: `(event) ${prettyComponentName(typeName)}.${ev}`,
            sortText: "2" + ev,
          });
        }
        return { suggestions };
      }

      // — Directive completion (after $)
      if (linePrefix.match(/\$\w*$/)) {
        const dirRange = {
          startLineNumber: position.lineNumber,
          endLineNumber:   position.lineNumber,
          startColumn:     word.startColumn - 1,  // include the $
          endColumn:       word.endColumn,
        };
        return {
          suggestions: DIRECTIVES.map(d => ({
            label: "$" + d.name, kind: sk.Keyword,
            insertText: d.snippet, insertTextRules: insertSnippet,
            range: dirRange, detail: d.description,
          })),
        };
      }

      // — `AS <type>` completion
      if (/\bAS\s+\w*$/i.test(linePrefix)) {
        const items = [];
        for (const t of TYPE_KEYWORDS) {
          items.push({ label: t.name, kind: sk.TypeParameter, insertText: t.name, range, detail: t.description });
        }
        for (const c of Object.keys(COMPONENT_REGISTRY)) {
          const display = prettyComponentName(c);
          items.push({ label: display, kind: sk.Class, insertText: display, range, detail: "GUI component type" });
        }
        return { suggestions: items };
      }

      // — General completions (keywords, builtins, components, locals)
      const suggestions = [];
      for (const kw of KEYWORDS) {
        suggestions.push({ label: kw, kind: sk.Keyword, insertText: kw, range, sortText: "3" + kw });
      }
      for (const fn of BUILTIN_FUNCTIONS) {
        suggestions.push({
          label: fn.name, kind: sk.Function,
          insertText: fn.snippet || (fn.name + "($0)"),
          insertTextRules: insertSnippet,
          range,
          detail: fn.signature || fn.description,
          documentation: fn.description,
          sortText: "1" + fn.name,
        });
      }
      for (const c of Object.keys(COMPONENT_REGISTRY)) {
        const display = prettyComponentName(c);
        suggestions.push({
          label: display, kind: sk.Class, insertText: display, range,
          sortText: "2" + display,
          detail: COMPONENT_REGISTRY[c].description || "GUI component type",
        });
      }
      // Local symbols from the model
      const text = model.getValue();
      const seen = new Set();
      const push = (name, kind, detail) => {
        const u = name.toUpperCase();
        if (seen.has(u)) return;
        seen.add(u);
        suggestions.push({ label: name, kind, insertText: name, range, detail, sortText: "0" + name });
      };
      for (const m of text.matchAll(/\bSUB\s+(\w+)/gi))      push(m[1], sk.Function, "(subroutine)");
      for (const m of text.matchAll(/\bFUNCTION\s+(\w+)/gi)) push(m[1], sk.Function, "(function)");
      for (const m of text.matchAll(/\bDIM\s+(\w+)/gi))      push(m[1], sk.Variable, "(variable)");
      for (const m of text.matchAll(/\bCONST\s+(\w+)/gi))    push(m[1], sk.Constant, "(constant)");
      for (const m of text.matchAll(/\bGLOBAL\s+(\w+)/gi))   push(m[1], sk.Variable, "(global)");
      for (const m of text.matchAll(/\bCREATE\s+(\w+)\s+AS\s+(\w+)/gi))
        push(m[1], sk.Variable, `(${m[2]})`);
      // Designer widgets from the project
      const proj = window.RapidR?.state?.project;
      if (proj) {
        for (const f of (proj.forms || [])) {
          push(f.name, sk.Class, "(form)");
          for (const w of (f.children || [])) push(w.name, sk.Variable, `(${w.type})`);
        }
        for (const m of (proj.modules || [])) push(m.name, sk.Module, "(module)");
      }
      return { suggestions };
    },
  });

  // ── Hover provider ─────────────────────────────────────────────
  monaco.languages.registerHoverProvider("rapidr", {
    provideHover: (model, position) => {
      const word = model.getWordAtPosition(position);
      if (!word) return null;
      const name = word.word;
      const upper = name.toUpperCase();

      // Component type hover
      if (COMPONENT_REGISTRY[upper]) {
        const reg = COMPONENT_REGISTRY[upper];
        const md = [
          `**${prettyComponentName(upper)}** — GUI component`,
          reg.description ? "" : null,
          reg.description || null,
          "",
          `\`${reg.props.length}\` properties · \`${reg.methods.length}\` methods · \`${reg.events.length}\` events`,
        ].filter(x => x !== null);
        return { contents: md.map(value => ({ value })) };
      }

      // Builtin function hover
      const fn = BUILTIN_FUNCTIONS.find(f => f.name.toUpperCase() === upper);
      if (fn) {
        return { contents: [
          { value: `**${fn.name}** — ${fn.signature || ""}` },
          { value: fn.description || "" },
        ]};
      }

      // Member after `.` — look at preceding identifier.
      const lineText = model.getLineContent(position.lineNumber);
      const before = lineText.substring(0, word.startColumn - 1);
      const ownerMatch = before.match(/(\w+)\.\s*$/);
      if (ownerMatch) {
        const owner = ownerMatch[1];
        const text = gatherProjectText(model.getValue());
        const typeName = COMPONENT_REGISTRY[owner.toUpperCase()] ? owner.toUpperCase()
                       : resolveVariableType(text, owner);
        const reg = typeName ? COMPONENT_REGISTRY[typeName] : null;
        if (reg) {
          const lower = name.toLowerCase();
          if (reg.props.includes(lower))   return { contents: [{ value: `(property) **${prettyComponentName(typeName)}.${name}**` }] };
          if (reg.methods.includes(lower)) {
            const sigInfo = reg.methodSignatures?.[lower];
            return { contents: [
              { value: `(method) **${prettyComponentName(typeName)}.${sigInfo?.sig || (name + "()")}**` },
              { value: sigInfo?.desc || "" },
            ]};
          }
          if (reg.events.includes(lower))  return { contents: [{ value: `(event) **${prettyComponentName(typeName)}.${name}**` }] };
        }
      }

      // Variable/CREATE'd resolution
      const text = gatherProjectText(model.getValue());
      const t = resolveVariableType(text, name);
      if (t) return { contents: [{ value: `\`${name}\` : **${prettyComponentName(t)}**` }] };
      return null;
    },
  });

  // ── Signature help — for builtin functions ─────────────────────
  monaco.languages.registerSignatureHelpProvider("rapidr", {
    signatureHelpTriggerCharacters: ["(", ","],
    provideSignatureHelp: (model, position) => {
      const lineText = model.getLineContent(position.lineNumber);
      const before = lineText.substring(0, position.column - 1);
      // Find the open paren walking back, skipping balanced pairs.
      let depth = 0;
      let openIdx = -1;
      for (let i = before.length - 1; i >= 0; i--) {
        const ch = before[i];
        if (ch === ")") depth++;
        else if (ch === "(") {
          if (depth === 0) { openIdx = i; break; }
          depth--;
        }
      }
      if (openIdx < 0) return null;
      const callee = before.substring(0, openIdx).match(/(\w[\w$]*)\s*$/);
      if (!callee) return null;
      const name = callee[1];
      const upper = name.toUpperCase();
      // Builtin?
      const fn = BUILTIN_FUNCTIONS.find(f => f.name.toUpperCase() === upper);
      let sigLabel = null, doc = null;
      if (fn) { sigLabel = fn.signature || `${fn.name}(...)`; doc = fn.description; }
      // Component method?
      if (!sigLabel) {
        const dotMatch = before.substring(0, openIdx).match(/(\w+)\.(\w+)\s*$/);
        if (dotMatch) {
          const typeName = COMPONENT_REGISTRY[dotMatch[1].toUpperCase()]
            ? dotMatch[1].toUpperCase()
            : resolveVariableType(gatherProjectText(model.getValue()), dotMatch[1]);
          const reg = typeName ? COMPONENT_REGISTRY[typeName] : null;
          const sigInfo = reg?.methodSignatures?.[dotMatch[2].toLowerCase()];
          if (sigInfo) { sigLabel = `${prettyComponentName(typeName)}.${sigInfo.sig}`; doc = sigInfo.desc; }
        }
      }
      if (!sigLabel) return null;

      // Count commas at depth 0 between openIdx and cursor → active param.
      const argsText = before.substring(openIdx + 1);
      let active = 0, d = 0;
      for (const ch of argsText) {
        if (ch === "(") d++;
        else if (ch === ")") d--;
        else if (ch === "," && d === 0) active++;
      }

      // Best-effort parameter parsing from the signature label.
      const paramText = sigLabel.match(/\(([^)]*)\)/)?.[1] || "";
      const params = paramText
        ? paramText.split(",").map(s => ({ label: s.trim() }))
        : [];

      return {
        value: {
          signatures: [{ label: sigLabel, documentation: doc || "", parameters: params }],
          activeSignature: 0,
          activeParameter: Math.min(active, Math.max(0, params.length - 1)),
        },
        dispose: () => {},
      };
    },
  });
}

export async function createRapidrEditor(host, { value = "", onChange } = {}) {
  const monaco = await loadMonaco();
  const isDark = document.documentElement.dataset.theme === "dark";
  const editor = monaco.editor.create(host, {
    value,
    language: "rapidr",
    theme: isDark ? "rapidr-dark" : "rapidr-vb6",
    automaticLayout: true,
    fontFamily: "Consolas, 'SF Mono', Menlo, monospace",
    fontSize: 13,
    minimap: { enabled: false },
    scrollBeyondLastLine: false,
    renderWhitespace: "selection",
    tabSize: 2,
    wordWrap: "off",
    suggestOnTriggerCharacters: true,
    quickSuggestions: { other: true, comments: false, strings: false },
    parameterHints: { enabled: true },
  });
  if (onChange) {
    editor.onDidChangeModelContent(() => onChange(editor.getValue()));
  }
  return editor;
}
