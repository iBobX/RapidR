// The manual's "Showcase" page (docs/manual/showcase.md): the programs in
// examples/showcase/ caught mid-action — the headless host, the test hooks'
// scripted clicks and keys, a made-up (and so reproducible) scene each.
// (50 ms an event: `wait` is n clicks on a label that does nothing with
// them, so the program's timers run meanwhile.)
export const topic = "showcase";

const wait = (comp, n) => Array(n).fill(`${comp}.onclick`).join(",");

export const scenes = [
  // the song started at its 44th second (the arpeggio is in), played a
  // little: columns up in the notes' colours, sparks flying, the crystal big
  { name: "midi-visualizer", program: "examples/showcase/midi_visualizer.rr", args: ["44"], delay: 1,
    env: { RAPIDR_TEST_EVENTS: `playbtn.onclick,${wait("info", 60)}` } },
  // the planet, a moment after it opened
  { name: "spin3d", program: "examples/showcase/spin3d.rr", delay: 1,
    env: { RAPIDR_TEST_EVENTS: wait("info", 30) } },
  // the lab as it opens, the noise brought down with its slider's Home key
  { name: "signal-lab", program: "examples/showcase/signal_lab.rr", delay: 1,
    env: { RAPIDR_TEST_EVENTS: `noisebar.__key_36,${wait("info", 30)}` } },
  // a ball, two rings and a heart, sent up with the number keys and clicks
  { name: "fireworks", program: "examples/showcase/fireworks.rr", delay: 1,
    env: {
      RAPIDR_TEST_EVENTS: `form.__key_49,sky.__mousedown_570_160,sky.__mouseup_570_160,${wait("info", 5)},form.__key_51,sky.__mousedown_420_240,sky.__mouseup_420_240,${wait("info", 5)},form.__key_52,sky.__mousedown_230_170,sky.__mouseup_230_170,${wait("info", 12)}`,
    } },
];
