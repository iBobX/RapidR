// The manual's "Versions" (docs/manual/versions.md): RDiffView's three
// views in the diff views example (examples/studio/diff_views.rr) — the
// text view with its changed words, the form view (the form before and
// after, the changes listed), the table view of two CSV files.
export const topic = "versions";

const DIFF = "examples/studio/diff_views.rr";

export const scenes = [
  { name: "diff-text", program: DIFF, delay: 2, crop: [0, 0, 860, 600] },
  { name: "diff-form", program: DIFF, delay: 2, crop: [0, 0, 860, 600], env: { RAPIDR_TEST_EVENTS: "bform.onclick" } },
  { name: "diff-table", program: DIFF, delay: 2, crop: [0, 0, 860, 600], env: { RAPIDR_TEST_EVENTS: "btable.onclick" } },
];
