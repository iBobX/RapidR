// The manual's "Drawing: canvases, bitmaps and pixels" page (docs/manual/graphics.md):
// examples/graphics/pixels_fast.rr on its own, the headless host at the
// manual's 2x scale (so the picture is what a Retina screen shows: each
// pixel of the 256 x 192 picture a crisp 2 x 2 block, a Life cell 8 x 8)
// and the test hooks' scripted clicks (`wait` is n clicks on a label that
// does nothing with them, so the program's timer runs meanwhile).
export const topic = "graphics";

const wait = (comp, n) => Array(n).fill(`${comp}.onclick`).join(",");

export const scenes = [
  // the plasma, about a second into its palette cycling, with its frames-a-second under it
  { name: "pixels-fast", program: "examples/graphics/pixels_fast.rr", delay: 1,
    env: { RAPIDR_TEST_EVENTS: wait("hint", 30) }, crop: [0, 0, 544, 496] },
  // the Game of Life of the same program: 64 x 48 cells drawn eight times as large
  { name: "pixels-life", program: "examples/graphics/pixels_fast.rr", delay: 1,
    env: { RAPIDR_TEST_EVENTS: `effectbtn.onclick,${wait("hint", 40)}` }, crop: [0, 0, 544, 496] },
  // RoundRect with a border and an inside in one call: a card, three chips, the three forms, in the three looks
  { name: "round-rect", program: "examples/graphics/round_rect.rr", theme: "rapidr-light", delay: 1 },
  { name: "round-rect-dark", program: "examples/graphics/round_rect.rr", theme: "rapidr-dark", delay: 1 },
  { name: "round-rect-contrast", program: "examples/graphics/round_rect.rr", theme: "rapidr-high-contrast", delay: 1 },
  // two timers at Interval = 20: the default one's ticks and the FixedRate one's, counted over a second
  { name: "steady-timer", program: "examples/gui/steady_timer.rr", theme: "rapidr-light", delay: 3 },
  // CopyMode on one picture, scanlines and a selection box included
  { name: "copy-modes", program: "examples/graphics/copy_modes.rr", theme: "rapidr-light", delay: 1 },
  { name: "copy-modes-dark", program: "examples/graphics/copy_modes.rr", theme: "rapidr-dark", delay: 1 },
];
