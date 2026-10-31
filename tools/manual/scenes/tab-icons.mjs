// The manual's "Tabs with icons: TabIcons" (docs/manual/components.md):
// the tab icons example (examples/gui/tab_icons.rr) wide — each tab its
// icon and its name — and after Narrower — the icons alone, never a cut
// name —, in light and dark.
export const topic = "components";

const TABS = "examples/gui/tab_icons.rr";
const NARROW = "widthbtn.onclick";

export const scenes = [
  { name: "tab-icons", program: TABS, delay: 2 },
  { name: "tab-icons-narrow", program: TABS, delay: 2, env: { RAPIDR_TEST_EVENTS: NARROW } },
  { name: "tab-icons-dark", program: TABS, delay: 2, theme: "rapidr-dark" },
];
