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
];
