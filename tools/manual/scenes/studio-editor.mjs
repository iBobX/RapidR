// The manual's "The code editor" page (docs/manual/studio-editor.md): RapidR
// Studio's completion on examples/gui/hello_form.rr, typed through the
// keyboard the way a user types (S-EDITOR). The RAPIDQ.INC shots are of a
// RapidQ program: the same code as hello_form.bas (a .rr program has
// RAPIDQ.INC's constants built in).
import { cpSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

export const topic = "studio-editor";

const OPEN = "examples/gui/hello_form.rr";
// (the example as a RapidQ program, hello_form.bas, in the scene's folder:
// Studio started there, so the file's name is all the screen shows)
const asBas = (work, root, steps) => {
  mkdirSync(join(work, "rapidq"), { recursive: true });
  writeFileSync(join(work, "rapidq", "hello_form.bas"), readFileSync(join(root, OPEN), "utf8").replaceAll("hello_form.rr", "hello_form.bas"));
  return { open: "hello_form.bas", cwd: join(work, "rapidq"), do: steps };
};
// (the code shown, the caret on a new last line)
const START = "focus:codedoc(0),key:Ctrl+End,key:Enter";
// (the caret on line 10, the blank line after the comments: room for the
// popups inside the editor)
const MID = ["focus:codedoc(0)", "key:Ctrl+Home", ...Array(9).fill("key:Down")].join(",");
// (the editor from line 1 to line 22)
const EDITOR = [244, 90, 792, 440];
// (S3) a RapidQ program in several files: its library and its window
// brought in with $INCLUDE
const SHAPES = "examples/rapidq/shapes/shapes.bas";
// (S5) Studio's Search and Problems test project (tests/fixtures/studio_search,
// RapidR's own): copied to the scene's folder, Studio started there — a short
// path on screen
const counter = (work, root, steps) => {
  cpSync(join(root, "tests/fixtures/studio_search"), join(work, "counter"), { recursive: true });
  return { open: "counter/Counter.rrproj", cwd: work, do: steps };
};
// (E0–E2) the helpers' example, the helpers come when typing pauses
const HELPERS = "examples/studio/helpers.rr";
const HELPERS_READY = "focus:codedoc(0),wait,wait,wait,wait";

export const scenes = [
  // `form.` then `c`: the members of the form that start so, with the docs
  { name: "completion", open: OPEN, do: `${MID},type:form.c`, delay: 6, crop: EDITOR },
  // Ctrl+Space on an empty line: the program's own names first
  { name: "ctrl-space", open: OPEN, do: `${MID},key:Ctrl+Space,wait`, delay: 6, crop: EDITOR },
  // a RapidR program: RAPIDQ.INC's constants built in, offered as they are
  { name: "completion-builtin", open: OPEN, do: `${MID},type:x = mby,wait`, delay: 6, crop: EDITOR },
  // a RapidQ program: a RAPIDQ.INC constant offered before the program includes it
  { name: "completion-include", setup: (work, root) => asBas(work, root, `${MID},type:x = mby,wait`), delay: 6, crop: EDITOR },
  // signature help after `(`
  { name: "signature", open: OPEN, do: `${MID},type:x$ = MID$(,wait`, delay: 6, crop: [244, 90, 792, 260] },
  // the hover over a statement
  { name: "hover", open: OPEN, do: `${MID},type:ShowMessage "Hi",key:Escape,key:Home,key:Right,key:Right,edit.showHover,wait,wait`, delay: 7, crop: [244, 90, 792, 260] },
  // F12 on a call: the caret on its SUB
  { name: "go-to-definition", open: OPEN, do: `view.code,${START},type:greet,key:Escape,key:Left,key:F12,wait`, delay: 6, crop: [244, 68, 792, 498] },
  // (S3) the Search tab beside Project: "Area" in the whole project, each
  // result file:line:col with the match marked
  { name: "search-tab", open: SHAPES, do: "wait,key:Ctrl+Shift+F,type:Area,key:Enter,wait", delay: 7, crop: [0, 68, 640, 440] },
  // (S3) Shift+F12 on AreaText in the window's file: its uses, by file
  { name: "find-references", open: SHAPES, do: "wait,open:forms/shapesform.inc,wait,focus:codedoc(1),caret:AreaTe,key:Shift+F12,wait", delay: 7, crop: [0, 68, 900, 440] },
  // (S5) Problems grouped by file, errors first: Targets Web, an error typed
  // into ui.rr
  { name: "problems-grouped", setup: (work, root) => counter(work, root, "wait,targets:web,open:ui.rr,view:Code,wait,code:StopServer=>StopServr,wait,view.problems,wait"), delay: 8, crop: [244, 560, 792, 190] },
  // mbYes without RAPIDQ.INC in a RapidQ program: squiggled, in Problems
  { name: "problems", setup: (work, root) => asBas(work, root, `${START},type:x = mbYes,key:Escape,wait,wait,wait,view.problems`), delay: 8, crop: [244, 480, 792, 225] },
  // Ctrl+. on it: the fix
  { name: "quick-fix", setup: (work, root) => asBas(work, root, `${MID},type:x = mbYes,key:Escape,wait,wait,wait,key:Left,key:Ctrl+.,wait`), delay: 8, crop: [244, 90, 792, 260] },
  // `NOT FILEEXISTS(…)` is always true (K4b): underlined, and Ctrl+. offers `= 0`
  { name: "not-fileexists", open: OPEN, do: `${MID},type:IF NOT FILEEXISTS("notes.txt") THEN PRINT 1,key:Escape,wait,wait,wait,wait,wait,key:Home,key:Right,key:Right,key:Right,key:Right,key:Ctrl+.,wait`, delay: 12, crop: [244, 90, 792, 260] },
  // (E0–E2, docs/code-helpers-plan.md) The helpers on examples/studio/helpers.rr:
  // the colours' swatches and the picture's thumbnail, the caret on the form's
  // Color
  // (line 38 brings the form's CREATE block into view, lines 17 to 38; the
  // caret then to its Color, line 18, already in view: nothing scrolls)
  { name: "helpers", open: HELPERS, do: `${HELPERS_READY},line:38,wait,wait,wait,line:18,key:End`, delay: 8, crop: EDITOR },
  // Ctrl+. on it: the colour editor
  { name: "helpers-colour", open: HELPERS, do: `${HELPERS_READY},line:38,wait,wait,wait,line:18,key:End,key:Ctrl+.,wait`, delay: 11, crop: [244, 110, 800, 520], smooth: true },
  // OnClick: the handler's list, Create handler first
  { name: "helpers-handler", open: HELPERS, do: `${HELPERS_READY},line:38,wait,wait,wait,line:36,key:End,key:Ctrl+.,wait`, delay: 11, crop: [244, 330, 792, 400] },
  // the stripes' FillRects: the argument names, the line's help at its end
  { name: "helpers-names", open: HELPERS, do: `${HELPERS_READY},line:49,key:End`, delay: 8, crop: [244, 90, 792, 440] },
  // the debugger paused: the values at the lines' ends
  { name: "helpers-values", open: "tests/fixtures/studio_debug/counter.rr", do: "line:6,debug.toggleBreakpoint,run.start,wait,wait,wait,wait", delay: 7, crop: [244, 90, 792, 260] },
];
