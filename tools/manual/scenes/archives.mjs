// The manual's "Archives: zip, 7z, tar.gz" (docs/manual/archives.md): the
// archives example (examples/data/archives.rr) with its zip sample open —
// the entries listed, the .z80 inside found and its first bytes shown — and
// with the 7z sample picked.
export const topic = "archives";

const PROGRAM = "examples/data/archives.rr";

export const scenes = [
  { name: "archives-zip", program: PROGRAM, delay: 2 },
  { name: "archives-7z", program: PROGRAM, delay: 2, env: { RAPIDR_TEST_EVENTS: "samples.__item_1" } },
];
