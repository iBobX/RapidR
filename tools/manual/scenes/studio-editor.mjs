// The manual's "The code editor" page (docs/manual/studio-editor.md): RapidR
// Studio's IntelliSense on examples/gui/hello_form.rr, typed through the
// keyboard the way a user types (S-EDITOR).
export const topic = "studio-editor";

const OPEN = "examples/gui/hello_form.rr";
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
  // a RAPIDQ.INC constant offered before the program includes it
  { name: "completion-include", open: OPEN, do: `${MID},type:x = mby,wait`, delay: 6, crop: EDITOR },
  // signature help after `(`
  { name: "signature", open: OPEN, do: `${MID},type:x$ = MID$(,wait`, delay: 6, crop: [244, 90, 792, 260] },
  // the hover over a statement
  { name: "hover", open: OPEN, do: `${MID},type:ShowMessage "Hi",key:Escape,key:Home,key:Right,key:Right,edit.showHover,wait,wait`, delay: 7, crop: [244, 90, 792, 260] },
  // F12 on a call: the caret on its SUB
  { name: "go-to-definition", open: OPEN, do: `view.code,${START},type:greet,key:Escape,key:Left,key:F12,wait`, delay: 6, crop: [244, 68, 792, 498] },
  // mbYes without RAPIDQ.INC: squiggled, in Problems
  { name: "problems", open: OPEN, do: `${START},type:x = mbYes,key:Escape,wait,wait,wait,view.problems`, delay: 8, crop: [244, 480, 792, 225] },
  // Ctrl+. on it: the fix
  { name: "quick-fix", open: OPEN, do: `${MID},type:x = mbYes,key:Escape,wait,wait,wait,key:Left,key:Ctrl+.,wait`, delay: 8, crop: [244, 90, 792, 260] },
];
