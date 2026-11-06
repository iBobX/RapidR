// The manual's "RFormEx: a form with more" (docs/manual/forms-ex.md): the
// example (examples/gui/form_ex.rr) after two files were dropped on its
// white box (its DragZone) — each listed with where it landed.
export const topic = "forms-ex";

export const scenes = [
  {
    name: "form-ex",
    program: "examples/gui/form_ex.rr",
    delay: 2,
    env: { RAPIDR_TEST_DROP: "holiday-photo.png;notes.txt", RAPIDR_TEST_EVENTS: "main.__drop_120_60,main.__drop_220_90" },
  },
];
