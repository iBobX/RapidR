// The manual's "Panes and splitters" (docs/manual/getting-started.md): the
// splitters at rest, under the mouse and given the keyboard, made by
// tools/manual/shots.mjs.
//
// Studio's window is 1280 × 800 (logical pixels): the splitter between the
// left column and the documents stands at x ≈ 242, the one above the bottom
// panel lies at y ≈ 566; the crop holds where they meet and both grips.

const COUNTER = "tools/manual/scenes/debugger/counter.rr";
const MEET = [130, 330, 620, 280];

export const topic = "panes";

export const scenes = [
  { name: "splitters-rest", open: COUNTER, do: "wait", delay: 4, crop: MEET },
  // (the mouse resting on the left splitter: lit after its short delay)
  { name: "splitters-hover", open: COUNTER, do: "wait", delay: 4, crop: MEET, env: { RAPIDR_TEST_EVENTS: "dock.__mousemove_242_300" } },
  // (Window ▸ Resize Panes from the editor: the splitter above the bottom
  // panel takes the keyboard, its grip longer)
  { name: "splitters-keyboard", open: COUNTER, do: "wait,window.resizePanes", delay: 4, crop: MEET },
];
