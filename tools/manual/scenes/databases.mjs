// The manual's "Databases" page (docs/manual/databases.md) and "The code
// editor in your programs" (docs/manual/code-editor.md): the background
// queries example (examples/data/million_rows_async.rr) — a million rows
// streamed into an RDBGrid, a Ping answered meanwhile — and its SQL
// editor completing the table's columns (REQUEST-001 items 1-3); the
// transactions example and its close question (DB-D).

export const topic = "databases";

const EXAMPLE = "examples/data/million_rows_async.rr";
// (keys typed into the editor: Ctrl+End, Enter, then "select c")
const TYPE_SELECT_C = ["__key_35_16", "__key_13", "__key_83", "__key_69", "__key_76", "__key_69", "__key_67", "__key_84", "__key_32", "__key_67"].map((k) => `editor.${k}`).join(",");

export const scenes = [
  // a million rows made and queried in the background: the grid has them,
  // the status says how long, a Ping clicked while the table was made
  { name: "million-rows-async", program: EXAMPLE, delay: 2,
    env: { RAPIDR_TEST_EVENTS: "makebtn.onclick,pingbtn.onclick,db.__until_busy_0,grid.__cell_2_3" } },
  // the SQL editor: "select c" typed, completion offers the table's columns
  // (SetSchema), the table, then SQLite's words
  { name: "sql-completion", program: EXAMPLE, args: ["20000"], delay: 2, crop: [0, 0, 780, 330],
    env: { RAPIDR_TEST_EVENTS: `makebtn.onclick,db.__until_busy_0,${TYPE_SELECT_C}` } },
  // (DB-D) the transactions example (examples/data/transactions.rr): a
  // transaction begun, two rows added — the status says what InTransaction
  // says; then closed: the question whose default is No (MESSAGEDLG's
  // fifth argument), its own window (components.md)
  { name: "transactions", program: "examples/data/transactions.rr", delay: 2,
    env: { RAPIDR_TEST_SYSTEM_LOOK: "light", RAPIDR_TEST_EVENTS: "beginbtn.onclick,addbtn.onclick,addbtn.onclick" } },
  { topic: "components", name: "message-default-no", program: "examples/data/transactions.rr", delay: 2, window: 2,
    env: { RAPIDR_TEST_SYSTEM_LOOK: "light", RAPIDR_TEST_EVENTS: "beginbtn.onclick,addbtn.onclick,form.__close" } },
];
