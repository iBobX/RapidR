// The manual's "Recipes for systems programs" page (docs/manual/recipes-systems.md):
// three of the ten recipes in examples/recipes/ caught mid-action — the headless
// host and the test hooks' scripted clicks (one scene each, made up and so
// reproducible; `wait` is n clicks on a label that does nothing with them, so
// the program's timer runs meanwhile).
export const topic = "systems";

const wait = (comp, n) => Array(n).fill(`${comp}.onclick`).join(",");

export const scenes = [
  // the tiny CPU after its three first Steps: PC at 0005, A holding 3 + 5
  { name: "cpu-step", program: "examples/recipes/cpu_step.rr", delay: 1,
    env: { RAPIDR_TEST_EVENTS: wait("stepbtn", 3) } },
  // the hex dump's first page: a text (and the ASCII column beside it) and the tiny CPU's program
  { name: "hex-dump", program: "examples/recipes/hex_dump.rr", delay: 1,
    env: { RAPIDR_TEST_EVENTS: wait("status", 1) } },
  // the video memory drawn on the canvas, the ball some frames into its bounce
  { name: "video-memory", program: "examples/recipes/video_memory.rr", delay: 1,
    env: { RAPIDR_TEST_EVENTS: wait("framebtn", 60) } },
];
