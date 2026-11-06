// The manual's "The mouse wheel" (docs/manual/components.md): the wheel
// list example (examples/gui/wheel_list.rr) after two notches of the wheel
// over its painted rows (the card's OnMouseWheel moved the bar: the rows
// start at 3. Earth) and a planet picked in the list by a click, then
// Down (its OnChange: "Picked: Mars"); "8 rows" checked from code.
export const topic = "components";

const WHEEL = "examples/gui/wheel_list.rr";
const USED = "card.__wheel_50_40_2,pick.__item_2,pick.__key_40";

export const scenes = [
  { name: "wheel-list", program: WHEEL, delay: 2, env: { RAPIDR_TEST_EVENTS: USED } },
];
