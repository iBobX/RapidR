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
  // the first note, as the program opens
  { name: "notes", program: "examples/showcase/notes.rr", delay: 1,
    env: { RAPIDR_TEST_EVENTS: wait("info", 8) } },
  // the snake with the computer playing (C), the same apples every time
  // (the seed 7), a while into the game
  { name: "snake", program: "examples/showcase/snake.rr", args: ["7"], delay: 1,
    env: { RAPIDR_TEST_EVENTS: `form.__key_32,form.__key_67,${wait("info", 500)}` } },
  // four sets, one set of numbers
  { name: "same-numbers", program: "examples/showcase/same_numbers.rr", delay: 1,
    env: { RAPIDR_TEST_EVENTS: wait("info", 3) } },
  // the darts at 300,000 a second for a moment: the circle full, the estimate settled
  { name: "monte-carlo-pi", program: "examples/showcase/monte_carlo_pi.rr", delay: 1,
    env: { RAPIDR_TEST_EVENTS: `speedbar.__key_35,${wait("info", 30)}` } },
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
