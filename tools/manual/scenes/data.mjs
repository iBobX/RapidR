// The manual's "Data science" page (docs/manual/data-science.md): the CSV
// Explorer example (examples/data/csv_explorer.rr) — its window with the
// sample, a CSV dropped on it (the OnDropFiles test hook), the filter —
// and RapidR Studio with it: the Welcome page's card, the designer (the
// RPLOT's sample bars) (DEMO-CSV).
import { cpSync, mkdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";

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
  // (I7, docs/manual/data-science-with-rapidr.md) linked data built in the
  // designer by its own tools: an RCSV dropped (in the tray) with
  // FileName = staff.csv, an RDBGrid and an REdit with DataSource = CSV1 —
  // the rows and Alice show at once. The project: an empty form beside a
  // copy of examples/data/staff.csv.
  {
    name: "linked-build",
    delay: 10,
    crop: [244, 64, 1036, 506],
    setup(work, root) {
      mkdirSync(join(work, "staffapp"), { recursive: true });
      cpSync(join(root, "examples", "data", "staff.csv"), join(work, "staffapp", "staff.csv"));
      writeFileSync(join(work, "staffapp", "app.rr"), 'CREATE Form1 AS RForm\n    Caption = "Staff"\n    Width = 520\n    Height = 360\nEND CREATE\n\nForm1.ShowModal\n');
      return {
        open: join(work, "staffapp", "app.rr"),
        do: "wait,view.designer,wait,tool:RCSV,prop:FileName=staff.csv,wait,tool:RDBGrid,prop:DataSource=CSV1,wait,tool:REdit,prop:DataSource=CSV1,prop:DataField=Name,filter:Data,wait,wait",
      };
    },
  },
  // the example in the designer: it reads staff.csv itself — the grid,
  // edits and labels show it
  { name: "linked-designer", open: "examples/data/linked_data.rr", delay: 6, crop: [245, 96, 790, 470] },
  // …running: Diana's row current, the grid sorted by Salary, highest first
  { name: "linked-running", program: "examples/data/linked_data.rr", delay: 3,
    env: { RAPIDR_TEST_EVENTS: "grid.__cell_1_2,nextbtn.onclick,grid.__cell_2_-1,grid.__cell_2_-1" } },
  // a million rows in the grid (big.csv written once, beside the example)
  { name: "million-rows", program: "examples/data/million_rows.rr", delay: 8 },
  // JSON records as a source: Calculator picked
  { name: "json-source", program: "examples/data/json_source.rr", delay: 3,
    env: { RAPIDR_TEST_EVENTS: "grid.__cell_0_4" } },
];
