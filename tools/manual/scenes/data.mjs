// The manual's "Data science" page (docs/manual/data-science.md): the CSV
// Explorer example (examples/data/csv_explorer.rr) — its window with the
// sample, a CSV dropped on it (the OnDropFiles test hook), the filter —
// and RapidR Studio with it: the Welcome page's card, the designer (the
// RPLOT's sample bars) (DEMO-CSV).
export const topic = "data";

const CSV = "examples/data/csv_explorer.rr";
// (keys typed into the filter box: S, U)
const FILTER_SU = "filterbox.__key_83,filterbox.__key_85";

export const scenes = [
  // the sample, as the program starts
  { name: "csv-explorer", program: CSV, delay: 3 },
  // staff.csv dropped on the window: X Years, a scatter chart, the table
  // sorted by Salary (its heading clicked)
  { name: "csv-explorer-dropped", program: CSV, delay: 3,
    env: { RAPIDR_TEST_DROP: "staff.csv", RAPIDR_TEST_EVENTS: "form.__drop,xbox.__item_3,kindbox.__item_2,table.__mousedown_216_10,table.__mouseup_216_10" } },
  // "su" in Filter: the summer months, Revenue as a line
  { name: "csv-explorer-filter", program: CSV, delay: 3,
    env: { RAPIDR_TEST_EVENTS: `${FILTER_SU},ybox.__item_2,kindbox.__item_1` } },
  // RapidR Studio's Welcome page: the example's card
  { name: "studio-welcome", delay: 5, crop: [245, 96, 790, 470] },
  // the example in the designer: the chart shows sample bars until the
  // program gives it data
  { name: "studio-designer", open: CSV, delay: 6 },
];
