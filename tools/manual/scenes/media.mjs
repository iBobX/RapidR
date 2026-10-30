// The manual's "DirectX, media and devices" page (docs/manual/
// directx-and-media.md), its sound made sample by sample: RAudioStream's
// examples caught playing — the headless host, the test hooks' clicks, the
// tests' silent sink taking the samples in real time (nothing is heard).
// (50 ms an event: `wait` is n clicks on a label that does nothing with
// them, so the stream and the timers run meanwhile.)
export const topic = "media";

const wait = (comp, n) => Array(n).fill(`${comp}.onclick`).join(",");

export const scenes = [
  // A4 (440 Hz) as a sine, playing: the scope of the samples written
  { name: "tone-generator", program: "examples/media/tone_generator.rr", delay: 1,
    env: { RAPIDR_TEST_EVENTS: `startbtn.onclick,${wait("info", 16)}` } },
  // the one-bit beeper a moment into its tune: the bit's flips over a
  // frame above, the frame's 882 samples below
  { name: "beeper", program: "examples/media/beeper.rr", delay: 1,
    env: { RAPIDR_TEST_EVENTS: `startbtn.onclick,${wait("info", 40)}` } },
];
