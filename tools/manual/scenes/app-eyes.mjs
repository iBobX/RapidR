// docs/manual/app-eyes-and-hands.md's pictures: the program's own eyes and
// hands — examples/gui/snapshot.rr checking its own layout after its
// button was narrowed (RAppCapture), examples/gui/guided_tour.rr while its
// "Show me" tour types (the virtual pointer over the box) and when it is
// done (RAppDriver). Programs of RapidR's own, nothing saved outside the
// scene's folder.
export const topic = "app-eyes";

export const scenes = [
  {
    name: "snapshot-check",
    program: "examples/gui/snapshot.rr",
    delay: 1,
    env: { RAPIDR_TEST_SYSTEM_LOOK: "light", RAPIDR_TEST_EVENTS: "sizebutton.onclick,checkbutton.onclick" },
  },
  {
    name: "tour-pointer",
    program: "examples/gui/guided_tour.rr",
    delay: 1,
    env: { RAPIDR_TEST_SYSTEM_LOOK: "light", RAPIDR_TEST_EVENTS: "tourbutton.onclick,form.__wait_1700" },
  },
  {
    name: "tour-done",
    program: "examples/gui/guided_tour.rr",
    delay: 1,
    env: { RAPIDR_TEST_SYSTEM_LOOK: "light", RAPIDR_TEST_EVENTS: "tourbutton.onclick,form.__wait_7000" },
  },
];
