// The manual's "Big numbers" (docs/manual/big-numbers.md): its
// screenshots, made by tools/manual/shots.mjs. The programs are the page's
// own (tools/manual/scenes/big-numbers/); the steps are Studio's --do steps.
//
// Studio's window is 1280 × 800 (logical pixels): the editor from (244, 70),
// the panes below it from y = 568; the Properties column from x = 1036
// isn't needed here.

const FIVE = "tools/manual/scenes/big-numbers/five64.rr";
const WATCH = "tools/manual/scenes/big-numbers/watch64.rr";

// (the editor and the panes below it)
const EDITOR_AND_PANES = [244, 64, 758, 684];

export const topic = "big-numbers";
export const scenes = [
  // (F5: the five 64-bit items and the bit functions, in the Output pane)
  { name: "run", open: FIVE, do: "run.start,wait,wait,wait,wait,wait,wait", delay: 12, crop: EDITOR_AND_PANES },
  // (the debugger at a breakpoint: a UINT64 above 2^63 in the code's
  // values, the Watch pane — `big + 1` wraps to 0 — and a VARIANT)
  { name: "watch", open: WATCH, do: "line:8,debug.toggleBreakpoint,watch:big,watch:v,watch:big + 1,run.start,wait,wait,wait,wait,wait,view.watch,wait", delay: 14, crop: EDITOR_AND_PANES },
];
