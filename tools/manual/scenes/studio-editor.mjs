// The manual's "The code editor" page (docs/manual/studio-editor.md): RapidR
// Studio's completion on examples/gui/hello_form.rr, typed through the
// keyboard the way a user types (S-EDITOR). The RAPIDQ.INC shots are of a
// RapidQ program: the same code as hello_form.bas (a .rr program has
// RAPIDQ.INC's constants built in).
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
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
  // mbYes without RAPIDQ.INC in a RapidQ program: squiggled, in Problems
  { name: "problems", setup: (work, root) => asBas(work, root, `${START},type:x = mbYes,key:Escape,wait,wait,wait,view.problems`), delay: 8, crop: [244, 480, 792, 225] },
  // Ctrl+. on it: the fix
  { name: "quick-fix", setup: (work, root) => asBas(work, root, `${MID},type:x = mbYes,key:Escape,wait,wait,wait,key:Left,key:Ctrl+.,wait`), delay: 8, crop: [244, 90, 792, 260] },
];
