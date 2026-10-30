// The manual's "The inspector" (docs/manual/designing-forms.md): the
// Properties pane under the headings of what each property is for — the same
// for every component, Accessibility among them — and, in a project kept
// classic-compatible, the small "new" a member classic programs lack carries.
// Made by tools/manual/shots.mjs; the pane is the right-hand column of
// Studio's 1280 × 800 window (272 wide), made taller (extent) so more of it
// shows, scrolled to the row named (row:).

export const topic = "inspector";

const PANE = [1008, 64, 272, 526];
const FORM = "wait,view.designer,pick:Form,extent:inspector:height:520,wait,row:WindowState,wait";
const BUTTON = "wait,view.designer,pick:GoBtn,extent:inspector:height:520,wait,row:MinHeight,wait";
const CLASSIC = "tests/fixtures/studio_classic/Classic.rrproj";

export const scenes = [
  // a form's rows: Help, Layout, Window — each with the properties of its purpose
  { name: "categories-light", open: "examples/gui/pantry.rr", do: FORM, delay: 8, crop: PANE },
  { name: "categories-dark", open: "examples/gui/pantry.rr", do: FORM, delay: 8, crop: PANE, theme: "rapidr-dark" },
  // the same pane in a project kept classic-compatible: the small "new"
  { name: "classic-marks-light", open: CLASSIC, do: BUTTON, delay: 8, crop: PANE },
  { name: "classic-marks-dark", open: CLASSIC, do: BUTTON, delay: 8, crop: PANE, theme: "rapidr-dark" },
  { name: "classic-marks-contrast", open: CLASSIC, do: BUTTON, delay: 8, crop: PANE, theme: "rapidr-high-contrast" },
];
