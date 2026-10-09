// RapidR Studio's flows on both hosts (docs/ide-plan.md I1): the shell
// driven through its own commands (`--do`, the `do` parameter on the web)
// on the desktop's headless host and on the web page, the same properties
// read on both (RAPIDR_TEST_DUMP) — open an example, run it (in its own
// process on the desktop, a sandboxed frame on the web) and read its
// output, the outline and problems from the language service, the palette,
// a theme switch, the document mode, a project saved and opened again.
//
//   tools/build_studio_web.sh
//   python3 -m http.server -d target/studio-web 18473 --bind 127.0.0.1
//   node tests/studio_flows.mjs [filter…]

import { spawnSync } from "node:child_process";
import { copyFileSync, cpSync, existsSync, mkdirSync, readFileSync, rmSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";

const HERE = dirname(fileURLToPath(import.meta.url));
const ROOT = dirname(HERE);
// (STUDIO_WEB_URL: each lane serves its own build on its own port)
const URL_BASE = (process.env.STUDIO_WEB_URL || process.env.RAPIDR_STUDIO_URL || "http://127.0.0.1:18473/").replace(/\/+$/, "");
const RAPIDR = process.env.RAPIDR || join(ROOT, "rapidr");
const WORK = join(ROOT, "tests", "results", "studio-flows");
const filters = process.argv.slice(2);

// (I4) The designer's steps on notepad.bas: the placing tool's click, the
// form's right edge dragged, Button1 dragged (the form sits 24 px in on the
// surface's backdrop).
const DESIGN_STEPS = [
  "__mousedown_112_132", "__mouseup_112_132",
  "__mousedown_503_262", "__mousemove_533_262", "__mousemove_563_262", "__mouseup_563_262",
  "__mousedown_122_142", "__mousemove_142_162", "__mousemove_162_182", "__mouseup_162_182",
].map((e) => `designdoc(0).${e}`).join(",");

// (S-DESIGN-2) Keys pressed on the designer: "Ab&" is Shift+A, B, Shift+7
// (a US keyboard, as the hosts' test hooks type them); {Enter} and the
// like by name.
const KEYS = { Enter: "13", Escape: "27", Tab: "9", F2: "113", "Ctrl+O": "79_16", "Ctrl+=": "187_16", "Ctrl+-": "189_16", "Ctrl+0": "48_16" };
function typed(text) {
  const out = [];
  for (const m of text.matchAll(/\{([^}]+)\}|(.)/g)) {
    if (m[1]) { out.push(KEYS[m[1]]); continue; }
    const c = m[2];
    if (/[a-z]/.test(c)) out.push(String(c.toUpperCase().charCodeAt(0)));
    else if (/[A-Z]/.test(c)) out.push(`${c.charCodeAt(0)}_256`);
    else if (/[0-9 ]/.test(c)) out.push(String(c.charCodeAt(0)));
    else if (")!@#$%^&*(".includes(c)) out.push(`${48 + ")!@#$%^&*(".indexOf(c)}_256`);
    else out.push({ "-": "189", ".": "190" }[c]);
  }
  return out.map((k) => `designdoc(0).__key_${k}`).join(",");
}
// (the mouse on the designed form: its client area's (x, y), the form's
// frame 24 px in, its title bar 29 px, a menu bar 28 px)
const at = (x, y, menu = 0) => `${x + 25}_${y + 54 + menu}`;
const click = (x, y, menu = 0) => `designdoc(0).__mousedown_${at(x, y, menu)},designdoc(0).__mouseup_${at(x, y, menu)}`;

// Each case: what Studio opens and does (`do`: its commands; `events`:
// RAPIDR_TEST_EVENTS, input through the kernel), how long it waits before
// the properties are read, and what each must say (a regular expression;
// `same`: equal to a file's text, line ends as the editor keeps them).
const CASES = [
  {
    name: "run-console",
    open: "examples/basics/hello.rr",
    do: "run.start,wait,wait,wait",
    delay: 5,
    dump: { "outputbox.text": /Hello from RapidR![\s\S]*ended, exit code 0/, "session.state": /^stopped$/, "session.exitcode": /^0$/ },
  },
  {
    // (the old web IDE's console suite) CLS, COLOR and LOCATE in Output as
    // on a terminal: CLS cleared "one" / "two"; LOCATE 1, 7 overwrote row 1
    // from column 7; the next PRINT went on row 2 over "yellow on blue"; no
    // escape sequence left in the text (the colours: rapidr-value's
    // panels::console::screen tests)
    name: "run-ansi",
    open: "tests/fixtures/studio_console_ansi.bas",
    webFiles: ["tests/fixtures/studio_console_ansi.bas"],
    do: "run.start,wait,wait,wait",
    delay: 5,
    dump: { "outputbox.text": /^(?![\s\S]*(\x1b|\bone\b|\btwo\b))[\s\S]*^first LINE\nrow2ow on blue$/m, "session.exitcode": /^0$/ },
  },
  {
    name: "outline-problems",
    open: "examples/gui/hello_form.rr",
    do: "wait",
    delay: 3,
    dump: { "outlinetree.itemcount": /^[5-9]|1\d$/, "lang.errorcount": /^0$/, "proj.kind": /^file$/, "proj.filecount": /^1$/ },
  },
  {
    // A program with errors: Problems lists them, and Run runs nothing
    name: "problems",
    open: "tests/fixtures/studio_problems.bas",
    webFiles: ["tests/fixtures/studio_problems.bas"],
    do: "wait,run.start,wait,wait",
    delay: 4,
    dump: { "lang.errorcount": /^[1-9]\d*$/, "outputbox.problemcount": /^[1-9]\d*$/, "outputbox.page": /^problems$/, "session.state": /^stopped$/, "outputbox.text": /^(?![\s\S]*fine)/ },
  },
  {
    name: "theme-and-tabs",
    open: "examples/gui/hello_form.rr",
    do: "view.theme.dark",
    delay: 3,
    dump: { "application.theme": /^rapidr dark$/, "dock.documentmode": /^tabs$/ },
  },
  {
    // (the desktop works on a copy: Save All writes the project file)
    name: "save-project",
    open: "examples/gui/hello_form.rr",
    copy: true,
    do: "file.saveAll,wait",
    delay: 3,
    dump: { "proj.kind": /^project$/, "proj.filename": /hello_form\.rrproj$/, "proj.filecount": /^1$/ },
  },
  {
    name: "open-folder",
    open: "",
    folder: "examples/gui",
    // (the web: the folder's files in the page's store, as showDirectoryPicker leaves them)
    webFiles: ["examples/gui/dialogs.rr", "examples/gui/hello_form.rr", "examples/gui/menus.rr"],
    do: "file.openFolder,wait,wait",
    delay: 4,
    dump: { "proj.mainfile": /^dialogs\.rr$/, "studio.caption": /^dialogs - RapidR Studio$/ },
  },
  {
    // Run > Build: the app for this system (interpreted: the project says),
    // with the project's own icon; on the web Build says it's the desktop's
    name: "build-app",
    open: "tests/fixtures/studio_app/Notes.rrproj",
    copyDir: true,
    webFiles: ["tests/fixtures/studio_app/Notes.rrproj", "tests/fixtures/studio_app/main.rr", "tests/fixtures/studio_app/note.svg"],
    do: "run.build",
    // (cargo checks the runner first: a minute on a busy machine)
    delay: 90,
    dump: { "proj.builtpath": /(Notes\.app|Notes\.AppDir|main\.exe)$/, "outputbox.text": /icon: .*note\.svg[\s\S]*Built /, "proj.building": /^0$/ },
    webDump: { "outputbox.text": /Can't build: Build makes apps in RapidR Studio on the desktop/, "proj.builtpath": /^$/ },
  },
  {
    // Project > Project Options: the app's name, ID, version, icon (previewed)
    name: "app-options",
    open: "tests/fixtures/studio_app/Notes.rrproj",
    copyDir: true,
    webFiles: ["tests/fixtures/studio_app/Notes.rrproj", "tests/fixtures/studio_app/main.rr", "tests/fixtures/studio_app/note.svg"],
    do: "project.options",
    delay: 4,
    dump: { "appnameedit.text": /^Notes$/, "appversionedit.text": /^1\.2\.0$/, "appiconedit.text": /^note\.svg$/, "appiconnote.caption": /every size/ },
  },
  {
    name: "palette",
    open: "",
    do: "view.commandPalette",
    delay: 3,
    dump: { "palette.count": /^[1-9]\d+$/, "palette.commandcount": /^[1-9]\d+$/ },
  },
  // (I4) The designer on the source: notepad.bas's form at its own size;
  // its right edge dragged 60 px (Width written), a QBUTTON placed from the
  // toolbox's tool (Button1's CREATE block written), then moved by 40, 40
  // (snapped); every step real input through the kernel (the mouse at
  // surface coordinates: the form's frame starts 12 px in).
  {
    name: "designer",
    open: "examples/rapidq/notepad.bas",
    do: "view.designer,designer.place.QBUTTON",
    events: DESIGN_STEPS,
    delay: 4,
    dump: {
      "designdoc(0).formname": /^Form$/,
      "designdoc(0).statustext": /^Button1 \(QBUTTON\), 128, 88, 75 × 25$/,
      "codedoc(0).text": /Width = 540\n    Height = 340[\s\S]*    CREATE Button1 AS QBUTTON\n        Caption = "Button1"\n        Left = 128\n        Top = 88\n        Width = 75\n        Height = 25\n    END CREATE\nEND CREATE/,
    },
  },
  // (I4) Enter on a toolbox item: AddComponent through the program (a
  // method's edits heard as OnSourceEdit on both hosts), named after the
  // registry's spelling (CheckBox1).
  {
    name: "designer-add",
    open: "examples/rapidq/notepad.bas",
    do: "view.designer,designer.add.QCHECKBOX",
    delay: 3,
    dump: { "designdoc(0).statustext": /^Added CheckBox1 \(QCHECKBOX\)/, "codedoc(0).text": /    CREATE CheckBox1 AS QCHECKBOX\n        Caption = "CheckBox1"\n/ },
  },
  // (I4) The same, then Ctrl+Z three times: the exact text back.
  {
    name: "designer-undo",
    open: "examples/rapidq/notepad.bas",
    do: "view.designer,designer.place.QBUTTON",
    events: DESIGN_STEPS + ",designdoc(0).__key_90_16,designdoc(0).__key_90_16,designdoc(0).__key_90_16",
    delay: 4,
    dump: { "designdoc(0).canundo": /^(0|False)$/i, "codedoc(0).text": /CREATE Form AS QFORM/ },
    same: { "codedoc(0).text": "examples/rapidq/notepad.bas" },
  },
  // (S-PANELS) The inspector on the designer: pantry's AddBtn selected,
  // its Caption and Width set in the inspector — the code shows them as the
  // smallest edit (the values on their line) and the inspector reads them
  // back.
  {
    name: "inspector-edits-code",
    open: "examples/gui/pantry.rr",
    do: "wait,view.designer,pick:AddBtn,prop:Caption=Go,prop:Width=120,wait",
    delay: 6,
    dump: {
      "inspector.target": /^AddBtn$/,
      "inspector.rows": /^Caption=Go$[\s\S]*^Width=120$/m,
      "codedoc(0).text": /    CREATE AddBtn AS QBUTTON\n        Caption = "Go": Left = 314: Top = 252: Width = 120\n        OnClick = AddItem\n/,
    },
  },
  // (S-PANELS) …then Undo twice on the designer: the exact text back.
  {
    name: "inspector-undo",
    open: "examples/gui/pantry.rr",
    do: "wait,view.designer,pick:AddBtn,prop:Caption=Go,prop:Width=120,wait,edit.undo,edit.undo,wait",
    delay: 7,
    dump: { "designdoc(0).canundo": /^(0|False)$/i, "inspector.rows": /^Caption=&Add to shelf$[\s\S]*^Width=110$/m, "codedoc(0).text": /CREATE AddBtn AS QBUTTON/ },
    same: { "codedoc(0).text": "examples/gui/pantry.rr" },
  },
  // (S-PANELS) The code edited (a Caption typed over): the designer reads
  // it and the inspector shows it.
  {
    name: "code-edits-inspector",
    open: "examples/gui/pantry.rr",
    do: 'wait,view.designer,pick:AddBtn,code:"&Add to shelf"=>"Store it",wait,wait,wait',
    delay: 7,
    dump: { "inspector.rows": /^Caption=Store it$/m, "designdoc(0).source": /Caption = "Store it": Left = 314/ },
  },
  // (S-PANELS) An event's row double-clicked in the inspector: its SUB
  // written with the registry's parameters (a DECLARE beside pantry's, the
  // SUB at the end), bound in the CREATE block, the caret inside it.
  {
    name: "inspector-event-handler",
    open: "examples/gui/pantry.rr",
    do: "wait,view.designer,pick:NameEdit,page:events,event:OnKeyDown,wait",
    delay: 6,
    dump: {
      "codedoc(0).text": /DECLARE SUB AddItem\nDECLARE SUB NameEditKeyDown \(Key AS WORD, Shift AS INTEGER\)\n[\s\S]*OnKeyDown = NameEditKeyDown\n[\s\S]*\nSUB NameEditKeyDown \(Key AS WORD, Shift AS INTEGER\)\n    \nEND SUB\n?$/,
      "inspector.rows": /^OnKeyDown=NameEditKeyDown$/m,
    },
  },
  // (S-PANELS) Typed values and a reset: Default as RapidQ writes a
  // Boolean, a colour constant, Width put back to its default (its
  // assignment taken out of the line); the Events page offers the file's
  // SUBs.
  {
    name: "inspector-typed",
    open: "examples/gui/pantry.rr",
    do: "wait,view.designer,pick:AddBtn,prop:Default=True,prop:Color=clRed,reset:Width,wait",
    delay: 6,
    dump: {
      "codedoc(0).text": /    CREATE AddBtn AS QBUTTON\n        Caption = "&Add to shelf": Left = 314: Top = 252\n        OnClick = AddItem\n        Default = 1\n        Color = clRed\n/,
      "inspector.rows": /^Default=True$[\s\S]*^Width=75$/m,
    },
  },
  // (S-DESIGN-2) Each kind of the inspector's editors writes the code a
  // program needs: a font by its parts (Font.Name quoted, Size, Color,
  // Bold as 1), an enum, a colour and a Boolean — the RapidQ constants a
  // program without RAPIDQ.INC doesn't define as their numbers (they'd
  // read as nothing), a caption with its & — and the inspector reads them
  // back.
  {
    name: "inspector-kinds",
    open: "examples/gui/hello_form.rr",
    do: "wait,view.designer,pick:Answer,prop:Font.Bold=True,prop:Font.Size=12,prop:Font.Color=clBlue,prop:Font.Name=Arial,prop:Alignment=taCenter,prop:Color=clYellow,prop:Caption=Say hi && bye,prop:WordWrap=True,wait",
    delay: 6,
    dump: {
      "codedoc(0).text": /    CREATE Answer AS QLABEL\n        Caption = "Say hi && bye"\n        Left = 16: Top = 96: Width = 300\n        Font\.Bold = 1\n        Font\.Size = 12\n        Font\.Color = &HFF0000\n        Font\.Name = "Arial"\n        Alignment = 2\n        Color = &H00FFFF\n        WordWrap = 1\n    END CREATE/,
      "inspector.rows": /^Alignment=taCenter$[\s\S]*^Caption=Say hi && bye$[\s\S]*^Color=(clYellow|&H00FFFF)$[\s\S]*^WordWrap=True$[\s\S]*^Font=Arial, 12 pt, Bold$/m,
    },
  },
  // (S-PANELS) The toolbox: Enter on QCHECKBOX adds one to the form (its
  // CREATE block in the code), selected in the inspector.
  {
    name: "toolbox-add",
    open: "examples/gui/pantry.rr",
    do: "wait,view.designer,tool:QCHECKBOX,wait",
    delay: 6,
    dump: { "codedoc(0).text": /CREATE CheckBox1 AS QCHECKBOX/i, "inspector.target": /^CheckBox1$/i },
  },
  // (S-DESIGN-2) The menu editor, on the form's own menu bar: Format >
  // Menu Editor gives hello_form a QMAINMENU; typing on its Type Here makes
  // File (its & mnemonic), Enter goes into its menu: Open… with Ctrl+O typed
  // in the ShortCut field (Tab), a separator, Exit — each item a QMENUITEM
  // CREATE block, one undo step.
  {
    name: "designer-menu",
    open: "examples/gui/hello_form.rr",
    do: "wait,view.designer,designer.menuEditor",
    events: typed("&File{Enter}&Open...{Tab}{Ctrl+O}{Enter}-{Enter}E&xit{Enter}{Escape}"),
    delay: 4,
    dump: {
      "codedoc(0).text": /    CREATE MainMenu1 AS QMAINMENU\n        CREATE File1 AS QMENUITEM\n            Caption = "&File"\n            CREATE Open1 AS QMENUITEM\n                Caption = "&Open\.\.\."\n                ShortCut = "Ctrl\+O"\n            END CREATE\n            CREATE N1 AS QMENUITEM\n                Caption = "-"\n            END CREATE\n            CREATE Exit1 AS QMENUITEM\n                Caption = "E&xit"\n            END CREATE\n        END CREATE\n    END CREATE\n/,
    },
  },
  // (S-DESIGN-2) The Tab-order editor: GreetButton clicked first, then
  // NameEdit — only GreetButton's TabOrder written (TabOrder = 0 puts it
  // first, the others after it in their order, as RapidQ's TabOrder does).
  {
    name: "designer-taborder",
    open: "examples/gui/hello_form.rr",
    do: "wait,view.designer,designer.tabOrder",
    events: `${click(150, 60)},${click(150, 22)}`,
    delay: 4,
    dump: {
      "designdoc(0).tabordermode": /^(-1|1|True)$/i,
      "designdoc(0).statustext": /^NameEdit: Tab order 1$/,
      "codedoc(0).text": /    CREATE NameEdit AS QEDIT\n        Text = "World"\n        Left = 112: Top = 16: Width = 200\n        OnChange = NameChanged\n    END CREATE\n    CREATE GreetButton AS QBUTTON\n[\s\S]*        OnClick = Greet\n        TabOrder = 0\n    END CREATE/,
    },
  },
  // (S-DESIGN-2) A caption edited in place: GreetButton clicked, then
  // clicked again (a slow click) — its caption typed over, Enter writes it.
  {
    name: "designer-caption",
    open: "examples/gui/hello_form.rr",
    do: "wait,view.designer",
    events: `${click(150, 60)},${click(150, 60)},${typed("Say &hi{Enter}")}`,
    delay: 4,
    dump: { "designdoc(0).editing": /^(0|False)$/i, "codedoc(0).text": /    CREATE GreetButton AS QBUTTON\n        Caption = "Say &hi"\n/ },
  },
  // (S-DESIGN-2) Smart guides while dragging: Answer held and moved a
  // little — its left edge lines up with NameLabel's (the capture shows the
  // guide; the button isn't let go).
  {
    name: "designer-guides",
    open: "examples/gui/hello_form.rr",
    do: "wait,view.designer",
    events: [`__mousedown_${at(100, 100)}`, `__mousemove_${at(104, 104)}`, `__mousemove_${at(103, 106)}`].map((e) => `designdoc(0).${e}`).join(","),
    delay: 4,
    dump: { "designdoc(0).guides": /^(edge|centre|baseline|margin|spacing \d+) [xy] -?\d+/ },
  },
  // (S-DESIGN-2) Zoom: View > Zoom In twice, then Ctrl+− and Ctrl+= on the
  // designer (110 %, 125 %, 110 %, 125 %).
  {
    name: "designer-zoom",
    open: "examples/rapidq/notepad.bas",
    do: "wait,view.designer,designer.zoomIn,designer.zoomIn",
    events: typed("{Ctrl+-}{Ctrl+=}"),
    delay: 4,
    dump: { "designdoc(0).zoom": /^125$/, "designdoc(0).statustext": /^Zoom 125 %$/ },
  },
  // (S-DESIGN-2) notepad.bas makes its OpenDialog and SaveDialog outside
  // the form: they show in its tray, and selecting one inspects it.
  {
    name: "designer-tray",
    open: "examples/rapidq/notepad.bas",
    do: "wait,view.designer,pick:SaveDialog,wait",
    delay: 4,
    dump: { "inspector.target": /^SaveDialog$/, "designdoc(0).statustext": /SaveDialog \(QSAVEDIALOG\)/ },
  },
  // (S-DESIGN-2) A console program has no form: Project > Add Form gives it
  // one (its CREATE block and ShowModal at the end), designed at once.
  {
    name: "designer-addform",
    open: "examples/basics/hello.rr",
    do: "wait,view.designer,project.addForm,designer.add.QBUTTON",
    delay: 4,
    dump: {
      "designdoc(0).formname": /^Form1$/,
      "codedoc(0).text": /\nCREATE Form1 AS RForm\n    Caption = "Form1"\n    Width = 320\n    Height = 240\n    CREATE Button1 AS RButton\n[\s\S]*END CREATE\n\nForm1\.ShowModal\n?$/,
    },
  },
  // (S-DESIGN-2) One undo history for the file: a designer change, a
  // change typed in the code, another designer change; Undo (from the
  // code) takes back only the last, in the order they were made.
  {
    name: "designer-undo-interleave",
    open: "examples/gui/pantry.rr",
    do: 'wait,view.designer,pick:AddBtn,prop:Width=120,code:"&Add to shelf"=>"Store it",wait,wait,wait,view.designer,pick:AddBtn,prop:Left=320,wait,view.code,edit.undo,wait',
    delay: 7,
    dump: { "codedoc(0).text": /    CREATE AddBtn AS QBUTTON\n        Caption = "Store it": Left = 314: Top = 252: Width = 120\n/ },
  },
  // (S-DESIGN-2) …and three Undos: the file's exact text; Redo twice: the
  // designer's Width and the typed Caption back, in order.
  {
    name: "designer-undo-all",
    open: "examples/gui/pantry.rr",
    do: 'wait,view.designer,pick:AddBtn,prop:Width=120,code:"&Add to shelf"=>"Store it",wait,wait,wait,view.designer,pick:AddBtn,prop:Left=320,wait,edit.undo,edit.undo,edit.undo,wait',
    delay: 8,
    dump: { "codedoc(0).text": /CREATE AddBtn AS QBUTTON/ },
    same: { "codedoc(0).text": "examples/gui/pantry.rr" },
  },
  {
    name: "designer-redo",
    open: "examples/gui/pantry.rr",
    do: 'wait,view.designer,pick:AddBtn,prop:Width=120,code:"&Add to shelf"=>"Store it",wait,wait,wait,view.designer,edit.undo,edit.undo,edit.redo,edit.redo,wait',
    delay: 8,
    dump: { "codedoc(0).text": /    CREATE AddBtn AS QBUTTON\n        Caption = "Store it": Left = 314: Top = 252: Width = 120\n/ },
  },
  // (S-DESIGN-2, Robert: "keep going until I can add new forms") A new
  // form program; Project > Add Form (Form2.rr, named in the project tree:
  // Enter) — the main file includes it, it opens on its designer; an
  // RLabel, an REdit and an RButton dropped on it with their captions, the
  // button's OnClick written (it closes Form2); Form1 gets a button whose
  // OnClick shows Form2. Saved: the program's two files, RapidR's names, no
  // errors (tests/studio_add_form.mjs runs what was made).
  {
    name: "add-form",
    open: "",
    do: [
      "newproject:gui|{dir}|Multi", "wait", "wait",
      "project.addForm", "wait", "key:Enter", "wait", "wait", "wait",
      "tool:RLABEL", "prop:Caption=Hello from Form2",
      "tool:REDIT", "prop:Text=Type here",
      "tool:RBUTTON", "prop:Caption=Close", "event:OnClick", "wait", "type:Form2.Close",
      "open:main.rr", "wait", "view.designer", "wait",
      "tool:RBUTTON", "prop:Caption=Show Form2", "event:OnClick", "wait", "type:Form2.Show",
      "file.saveAll", "wait", "wait", "wait",
    ].join(","),
    delay: 12,
    dump: {
      "codedoc(0).text": /^\$APPTYPE GUI\n\$INCLUDE "Form2\.rr"\n[\s\S]*SUB Button2Click\n    Form2\.Show\nEND SUB[\s\S]*CREATE Form1 AS RForm[\s\S]*    CREATE Button2 AS RButton\n        Caption = "Show Form2"[\s\S]*OnClick = Button2Click/,
      "codedoc(1).text": /SUB Button1Click\n    Form2\.Close\nEND SUB[\s\S]*CREATE Form2 AS RForm[\s\S]*    CREATE Label1 AS RLabel\n        Caption = "Hello from Form2"[\s\S]*    CREATE Edit1 AS REdit\n[\s\S]*Text = "Type here"[\s\S]*    CREATE Button1 AS RButton\n        Caption = "Close"[\s\S]*OnClick = Button1Click/,
      "proj.filecount": /^2$/,
      "lang.errorcount": /^0$/,
    },
  },
  // (S-DESIGN-2, Robert's report) RForm in the toolbox is Project > Add
  // Form (a form is a document, not a component); RFormMDI adds an MDI
  // main window.
  {
    name: "toolbox-form",
    open: "",
    do: "newproject:gui|{dir}|Tb,wait,wait,tool:RFORM,wait,key:Enter,wait,wait,wait,tool:RFORMMDI,wait,key:Enter,wait,wait,wait",
    delay: 8,
    dump: { "codedoc(0).text": /^\$APPTYPE GUI\n\$INCLUDE "Form2\.rr"\n\$INCLUDE "Form3\.rr"\n/, "codedoc(1).text": /\nCREATE Form2 AS RForm\n/, "codedoc(2).text": /\nCREATE Form3 AS RFormMDI\n/, "proj.filecount": /^3$/ },
  },
  // (S-DESIGN-2) RForm dragged onto a designed form: never nested — a new
  // window, and the status bar says why; onto an RFormMDI: one of its child
  // windows, RapidQ's way.
  {
    name: "toolbox-form-drop",
    open: "",
    do: "newproject:gui|{dir}|Dr,wait,wait,drop:RFORM|designdoc(0),wait,key:Enter,wait,wait,wait",
    delay: 7,
    dump: { "codedoc(1).text": /\nCREATE Form2 AS RForm\n/, "outputbox.text": /A form can't go inside a form: Form2 was added as a new window\. For child windows, make Form1 an RFormMDI\./ },
  },
  {
    name: "toolbox-form-drop-mdi",
    open: "",
    do: "newproject:mdi|{dir}|Md,wait,wait,drop:RFORM|designdoc(0),wait,wait,wait",
    delay: 7,
    // (onto the MDI template's window, Main: a child window, RapidQ's way —
    // a panel on Main and Main.AddChild after it)
    dump: { "codedoc(0).text": /\n    CREATE Form1 AS RPanel\n        Left = 0\n        Top = 0\n        Width = 320\n        Height = 240\n    END CREATE\nEND CREATE\nMain\.AddChild\(Form1\.Handle, "Form1", 0, 0, 0, 0, 0, 1\)\n/, "outputbox.text": /Form1 is a child window of Main/, "proj.filecount": /^1$/ },
  },
  // (S-DESIGN-2, Robert's report) The form itself (nothing selected) in
  // the inspector, with its events: OnShow's handler made, bound and
  // written as a component's is.
  {
    name: "form-events",
    open: "examples/gui/hello_form.rr",
    do: "wait,view.designer,wait,page:events,event:OnShow,wait",
    delay: 6,
    dump: { "inspector.target": /^Form$/, "codedoc(0).text": /^(?=[\s\S]*\n        OnShow = FormShow\n|[\s\S]*\n    OnShow = FormShow\n)(?=[\s\S]*\nSUB FormShow\b)/ },
  },
  // (S-DESIGN-2) Project > Add Module: Module1.rr, named in the tree,
  // included by the main file, opened on its code.
  {
    name: "add-module",
    open: "",
    do: "newproject:gui|{dir}|Mods,wait,wait,project.addModule,wait,key:Enter,wait,wait,wait",
    delay: 6,
    dump: { "codedoc(0).text": /^\$APPTYPE GUI\n\$INCLUDE "Module1\.rr"\n\nCREATE Form1 AS RForm\n/, "codedoc(1).text": /^' Module1\.rr: SUBs and FUNCTIONs the program's files share$/, "proj.filecount": /^2$/, "lang.errorcount": /^0$/ },
  },
  // (S-DESIGN-2) A form's file renamed in the project tree (F2): the main
  // file's $INCLUDE follows it; then taken out of the project (Delete,
  // confirmed with Enter): the main file no longer includes it.
  {
    name: "rename-form",
    open: "",
    do: "newproject:gui|{dir}|Ren,wait,wait,project.addForm,wait,key:Enter,wait,wait,wait,rename:Form2.rr|About.rr,wait,wait,wait",
    delay: 7,
    dump: { "codedoc(0).text": /^\$APPTYPE GUI\n\$INCLUDE "About\.rr"\n/, "proj.filecount": /^2$/, "lang.errorcount": /^0$/ },
  },
  {
    name: "remove-form",
    open: "",
    do: "newproject:gui|{dir}|Rem,wait,wait,project.addForm,wait,key:Enter,wait,wait,wait,remove:Form2.rr,wait,key:Enter,wait,wait,wait",
    delay: 7,
    dump: { "codedoc(0).text": /^\$APPTYPE GUI\n\nCREATE Form1 AS RForm\n/, "proj.filecount": /^1$/, "lang.errorcount": /^0$/ },
  },
  // (S-DESIGN-2) A program with an $INCLUDE on the web: the language
  // service and the designer read the included file from the page's store
  // (rapidr_preprocessor's source reader), as the desktop reads the disk —
  // no "missing include" error, the form designed, its button's SUB known.
  {
    name: "include-web",
    open: "tests/fixtures/studio_split/main.rr",
    webFiles: ["tests/fixtures/studio_split/main.rr", "tests/fixtures/studio_split/greeting.inc"],
    do: "wait,wait,designer.add.QCHECKBOX,wait,wait",
    delay: 5,
    dump: { "lang.errorcount": /^0$/, "outputbox.problemcount": /^0$/, "designdoc(0).formname": /^Form$/, "codedoc(0).text": /    CREATE CheckBox1 AS QCHECKBOX\n/ },
  },
  // (S-SHELL-2) Documents are tabs, never windows: a form's file is one
  // tab with the Design | Code switch (no MDI window, no "[Design]"
  // document); F12 toggles to the code (Delphi), then both side by side —
  // the dock's layout says each view.
  {
    name: "tabs-design-code",
    open: "examples/rapidq/notepad.bas",
    do: "wait,view.toggleDesigner,wait",
    delay: 4,
    dump: { "dock.documentmode": /^tabs$/, "dock.documentcount": /^1$/, "dock.layout": /^documents 0 codedoc\(0\)\n[\s\S]*^view codedoc\(0\) 1 500$/m },
  },
  {
    name: "side-by-side",
    open: "examples/gui/hello_form.rr",
    do: "wait,view.sideBySide,wait",
    delay: 4,
    dump: { "dock.layout": /^view codedoc\(0\) split 500$/m, "designdoc(0).formname": /^Form$/ },
  },
  // (S-SHELL-2) A tab moved to a new group on the right (Window > Split
  // Right, as dragging it to the right edge): two groups side by side.
  {
    name: "split-groups",
    open: "tests/fixtures/studio_split/main.rr",
    webFiles: ["tests/fixtures/studio_split/main.rr", "tests/fixtures/studio_split/greeting.inc"],
    do: "wait,view:Split,open:greeting.inc,view.splitVertically,wait",
    delay: 4,
    dump: { "dock.documentgroupcount": /^2$/, "dock.layout": /^groups split row\n  1000 group 0 codedoc\(0\)\n  1000 group 0 codedoc\(1\)$/m, "dock.activedocument": /^codedoc\(1\)$/ },
  },
  // (S-SHELL-2) …and Studio started again on the same settings (not
  // --fresh): the project's files open again, the groups and the
  // side-by-side view as they were.
  {
    name: "restore-layout",
    open: "tests/fixtures/studio_split/main.rr",
    webFiles: ["tests/fixtures/studio_split/main.rr", "tests/fixtures/studio_split/greeting.inc"],
    do: "wait,view:Split,open:greeting.inc,view.splitVertically,wait,wait,wait,wait,wait,wait",
    delay: 5,
    restart: { do: "wait,wait", delay: 5 },
    dump: { "dock.documentgroupcount": /^2$/, "dock.documentcount": /^2$/, "dock.layout": /^groups split row\n[\s\S]*^view codedoc\(0\) split 500$/m },
  },
  // (S-SHELL-2, CMD-3) Find in Files over a project and the file it
  // includes: the results by file, the open editor's text and the disk's.
  {
    name: "find-in-files",
    open: "tests/fixtures/studio_split/main.rr",
    webFiles: ["tests/fixtures/studio_split/main.rr", "tests/fixtures/studio_split/greeting.inc"],
    do: "wait,find:SayGreeting",
    delay: 4,
    dump: { "searchinfo.caption": /^3 results in 2 files$/, "searchtree.itemcount": /^5$/ },
  },
  // (S-SHELL-2, HLP-1) F1: the registry's entry in the Help pane — for a
  // statement, and for the word at the caret in the code (a member of the
  // component before the dot).
  {
    name: "help",
    open: "examples/gui/hello_form.rr",
    do: "wait,help:SHOWMESSAGE",
    delay: 4,
    dump: { "helptitle.caption": /^SHOWMESSAGE$/, "helpsyntax.text": /^SHOWMESSAGE/, "helpwhat.caption": /^Statement · RapidQ/ },
  },
  {
    name: "help-f1",
    open: "examples/gui/hello_form.rr",
    do: 'wait,view.code,code:Answer.Caption=>Answer.Caption,help.contents',
    delay: 4,
    dump: { "helptitle.caption": /^QLABEL\.Caption$/, "helpwhat.caption": /^Property of QLABEL/ },
  },
  // (S-PANELS) The project tree lists the form's components; the palette
  // finds a symbol of the file.
  {
    name: "tree-and-search",
    open: "examples/gui/pantry.rr",
    do: "wait,palette:stock",
    delay: 4,
    dump: { "projecttree.filecount": /^1$/, "palette.selected": /^line:21:Stock$/ },
  },
  // ---- the code editor (S-EDITOR; docs/studio-wow.md ED-1 … ED-8): typed
  // through the kernel's keyboard path (Application.SendKeys), IntelliSense
  // from RapidR's language service ----
  {
    // `form.` lists QFORM's members: properties, then methods, then events,
    // each A–Z
    name: "editor-completion",
    open: "examples/gui/hello_form.rr",
    do: "focus:codedoc(0),key:Ctrl+End,key:Enter,type:form.,wait,wait",
    delay: 6,
    dump: { "codedoc(0).completionitems": /^AccessibleDescription\n[\s\S]*\nCaption\n[\s\S]*\nWidth\n[\s\S]*\nShowModal\n[\s\S]*\nOnClick\n/ },
  },
  {
    // typing narrows it, best match first: on the word starts (`sm` →
    // ShowModal) before letters anywhere
    name: "editor-completion-fuzzy",
    open: "examples/gui/hello_form.rr",
    do: "focus:codedoc(0),key:Ctrl+End,key:Enter,type:form.sm,wait",
    delay: 6,
    dump: { "codedoc(0).completionselected": /^ShowModal$/ },
  },
  {
    // Tab accepts the selected item; the language's words in upper case as
    // they're typed (rapidr.keywordCase = upper)
    name: "editor-accept-and-case",
    open: "examples/gui/hello_form.rr",
    do: "focus:codedoc(0),key:Ctrl+End,key:Enter,type:form.capt,key:Tab,type: = \"Hi\",key:Enter,type:dim y as string,key:Escape,key:Enter",
    delay: 8,
    dump: { "codedoc(0).text": /\nForm\.Caption = "Hi"\nDIM y AS STRING\n?$/, "codedoc(0).completionitems": /^$/ },
  },
  {
    // Tab on a selected block indents every line by the file's unit (4
    // spaces); the selection stays
    name: "editor-tab-indent",
    open: "examples/gui/hello_form.rr",
    do: "focus:codedoc(0),key:Ctrl+End,key:Shift+Up,key:Shift+Up,key:Tab",
    delay: 6,
    dump: { "codedoc(0).text": /\nEND SUB\n\n {4}NameEdit\.SetFocus\n {4}Form\.ShowModal\n?$/, "codedoc(0).sellength": /^3[0-9]$/ },
  },
  {
    // Shift+Tab takes it back out: the text as it was
    name: "editor-tab-outdent",
    open: "examples/gui/hello_form.rr",
    do: "focus:codedoc(0),key:Ctrl+End,key:Shift+Up,key:Shift+Up,key:Tab,key:Shift+Tab",
    delay: 6,
    dump: { "codedoc(0).text": /\nEND SUB\n\nNameEdit\.SetFocus\nForm\.ShowModal\n?$/, "codedoc(0).canundo": /^(-1|1|True)$/i },
  },
  {
    // a snippet: `sub` and Tab — the skeleton, its name selected; Tab again
    // to the parameters
    name: "editor-snippet",
    open: "examples/gui/hello_form.rr",
    do: "view.code,key:Ctrl+End,key:Enter,type:sub,wait,key:Tab,type:Hello,key:Tab,type:n AS INTEGER",
    delay: 6,
    dump: { "codedoc(0).text": /\nSUB Hello\(n AS INTEGER\)\n {4}\nEND SUB\n?$/ },
  },
  {
    // a misspelt member: squiggled once typing pauses (RapidQ's compiler's
    // words), in Problems too; Ctrl+. offers the fix, Enter applies it
    name: "editor-diagnostic",
    open: "examples/gui/hello_form.rr",
    do: "view.code,key:Ctrl+End,key:Enter,type:x$ = NameEdit.Txet,key:Escape,wait,wait,wait,key:Ctrl+.,wait",
    delay: 8,
    dump: { "codedoc(0).diagnosticcount": /^1$/, "codedoc(0).completionitems": /^Change to Text$/ },
  },
  {
    name: "editor-quick-fix",
    open: "examples/gui/hello_form.rr",
    do: "view.code,key:Ctrl+End,key:Enter,type:x$ = NameEdit.Txet,key:Escape,wait,wait,wait,key:Ctrl+.,wait,key:Enter,wait,wait,wait",
    delay: 10,
    dump: { "codedoc(0).text": /\nx\$ = NameEdit\.Text\n?$/, "codedoc(0).diagnosticcount": /^0$/ },
  },
  {
    // F2 renames from the language service's references: the DECLARE, the
    // SUB, OnClick = and the calls
    name: "editor-rename",
    open: "examples/gui/hello_form.rr",
    do: "view.code,key:Ctrl+End,key:Enter,type:greet,key:Escape,key:Left,key:F2,wait,key:Ctrl+A,type:SayHi,key:Enter,wait",
    delay: 7,
    dump: { "codedoc(0).text": /DECLARE SUB SayHi\n[\s\S]*OnClick = SayHi\n[\s\S]*\nSUB SayHi\n[\s\S]*\nSayHi\n?$/ },
  },
  {
    // Ctrl+F with a regular expression (Alt+R): found as it is typed, the
    // first match selected
    name: "editor-find-regex",
    open: "examples/gui/hello_form.rr",
    do: "view.code,key:Ctrl+F,wait,key:Alt+R,type:Show\\w+,wait",
    delay: 6,
    dump: { "codedoc(0).seltext": /^ShowModal$/ },
  },
  {
    // Edit > Undo takes the typing back (a word at a time), Redo again
    name: "editor-undo",
    open: "examples/gui/hello_form.rr",
    do: "view.code,key:Ctrl+End,type:one two,edit.undo,wait,edit.undo,edit.redo,wait",
    delay: 6,
    dump: { "codedoc(0).text": /\nForm\.ShowModal\none ?\n?$/, "codedoc(0).canredo": /^(-1|1|True)$/i },
  },
  {
    // F12 on a call goes to its SUB
    name: "editor-go-to-definition",
    open: "examples/gui/hello_form.rr",
    do: "focus:codedoc(0),key:Ctrl+End,key:Enter,type:greet,key:Escape,key:Left,key:F12",
    delay: 6,
    dump: { "codedoc(0).caretline": /^42$/ },
  },
  {
    // Tab at a line's start: the file's unit (4 spaces, never a tab
    // glyph); Shift+Tab takes it back
    name: "editor-tab-line-start",
    open: "examples/gui/hello_form.rr",
    do: "focus:codedoc(0),key:Ctrl+End,key:Enter,key:Tab,type:x,key:Escape",
    delay: 6,
    dump: { "codedoc(0).text": /\nForm\.ShowModal\n\n {4}x\n?$/ },
  },
  {
    name: "editor-shift-tab-line-start",
    open: "examples/gui/hello_form.rr",
    do: "focus:codedoc(0),key:Ctrl+End,key:Enter,key:Tab,type:x,key:Escape,key:Shift+Tab",
    delay: 6,
    dump: { "codedoc(0).text": /\nForm\.ShowModal\n\nx\n?$/ },
  },
  {
    // hover: the registry's syntax and doc for a RapidQ statement
    name: "editor-hover",
    open: "examples/gui/hello_form.rr",
    do: "focus:codedoc(0),key:Ctrl+End,key:Enter,type:ShowMessage \"Hi\",key:Escape,key:Home,key:Right,key:Right,edit.showHover,wait,wait",
    delay: 6,
    dump: { "codedoc(0).hovertext": /SHOWMESSAGE|ShowMessage/ },
  },
  {
    // signature help after `(`: the parameters
    name: "editor-signature",
    open: "examples/gui/hello_form.rr",
    do: "focus:codedoc(0),key:Ctrl+End,key:Enter,type:x$ = MID$(,wait",
    delay: 6,
    dump: { "codedoc(0).signaturetext": /MID\$\(/i },
  },
  {
    // Shift+F12: every use selected here (DECLARE, OnClick =, the SUB, the
    // call) and listed in Output
    name: "editor-references",
    open: "examples/gui/hello_form.rr",
    do: "focus:codedoc(0),key:Ctrl+End,key:Enter,type:greet,key:Escape,key:Left,key:Shift+F12,wait",
    delay: 6,
    dump: { "codedoc(0).cursorcount": /^4$/, "outputbox.text": /References:[\s\S]*:42:5[\s\S]*4 references/ },
  },
  {
    // Edit > Advanced > Fold All: the CREATE blocks and SUBs folded
    name: "editor-fold",
    open: "examples/gui/hello_form.rr",
    do: "focus:codedoc(0),edit.foldAll,wait",
    delay: 5,
    dump: { "codedoc(0).foldcount": /^[3-9]$/ },
  },
  {
    // the find box's search, then Edit > Find Next (F3): the next one
    name: "editor-find-next",
    open: "examples/gui/hello_form.rr",
    do: "focus:codedoc(0),key:Ctrl+F,wait,type:Caption,wait,key:Escape,edit.findNext,wait",
    delay: 6,
    dump: { "codedoc(0).caretline": /^22$/, "codedoc(0).seltext": /^Caption$/ },
  },
  {
    // (S-DESIGN ↔ S-EDITOR) the designer's two additions came to the code as
    // two undo steps of the editor's (ApplyPatches): Ctrl+Z twice in the
    // code gives the file back exactly, and nothing is left to undo
    name: "designer-code-undo",
    open: "examples/rapidq/notepad.bas",
    do: "wait,view.designer,designer.add.QCHECKBOX,designer.add.QBUTTON,wait,view.code,focus:codedoc(0),key:Ctrl+Z,key:Ctrl+Z,wait",
    delay: 6,
    dump: { "codedoc(0).canundo": /^(0|False)$/i, "codedoc(0).text": /CREATE Form AS QFORM/ },
    same: { "codedoc(0).text": "examples/rapidq/notepad.bas" },
  },
];

function runDesktop(c) {
  const dir = join(WORK, `${c.name}-desktop`);
  rmSync(dir, { recursive: true, force: true });
  mkdirSync(dir, { recursive: true });
  const args = ["run", "ide/studio.rr", "--home", ".", "--fresh", "--theme", "rapidr-light"];
  // ({dir}: the case's own folder — a new project goes there)
  if (c.do) args.push("--do", c.do.replaceAll("{dir}", dir));
  if (c.open && c.copyDir) {
    // (the project's whole folder: Build writes the app beside it)
    cpSync(join(ROOT, dirname(c.open)), join(dir, "project"), { recursive: true });
    args.push(join(dir, "project", c.open.split("/").pop()));
  } else if (c.open && c.copy) {
    const to = join(dir, c.open.split("/").pop());
    copyFileSync(join(ROOT, c.open), to);
    args.push(to);
  } else if (c.open) {
    args.push(c.open);
  }
  const run = (args, delay) => spawnSync(RAPIDR, args, {
    cwd: ROOT,
    timeout: Math.max(90000, delay * 1000 + 60000),
    encoding: "utf8",
    env: {
      ...process.env,
      RAPIDR_CAPTURE: join(dir, "window"),
      RAPIDR_CAPTURE_DELAY: String(delay),
      RAPIDR_MENU: "window",
      RAPIDR_TEST_DUMP: Object.keys(c.dump).join(","),
      ...(c.events ? { RAPIDR_TEST_EVENTS: c.events } : {}),
      RAPIDR_PRINT_TO: join(WORK, "prints"),
      RAPIDR_REGISTRY: join(WORK, `${c.name}.reg`),
      ...(c.folder ? { RAPIDR_TEST_FILE_DIALOG: join(ROOT, c.folder) } : {}),
    },
  });
  rmSync(join(WORK, `${c.name}.reg`), { force: true });
  let r = run(args, c.delay);
  if (c.restart) {
    // (started again on the settings the first run left: not --fresh)
    const again = args.filter((a) => a !== "--fresh");
    const k = again.indexOf("--do");
    if (k >= 0) again.splice(k, 2);
    if (c.restart.do) again.splice(again.length - 1, 0, "--do", c.restart.do);
    r = run(again, c.restart.delay || c.delay);
  }
  return parseDump(r.stdout || "", Object.keys(c.dump));
}

// "name=value" lines, a value running on to the next "name=" line.
function parseDump(text, names) {
  const out = {};
  const lines = text.split("\n");
  let cur = null;
  for (const line of lines) {
    const k = names.find((n) => line.toLowerCase().startsWith(n.toLowerCase() + "="));
    if (k) {
      cur = k;
      out[k] = line.slice(k.length + 1);
    } else if (cur && !line.startsWith("[rapidr]")) {
      out[cur] += "\n" + line;
    }
  }
  for (const k of Object.keys(out)) out[k] = out[k].replace(/\n+$/, "");
  return out;
}

async function runWeb(browser, c) {
  if (!c.restart) return runWebPage(await browser.newContext({ viewport: { width: 1920, height: 1080 } }), c, true);
  // (started again in the same browser profile: the page's settings kept)
  const ctx = await browser.newContext({ viewport: { width: 1920, height: 1080 } });
  await runWebPage(ctx, c, false);
  return runWebPage(ctx, { ...c, do: c.restart.do, delay: c.restart.delay || c.delay, fresh: false }, true);
}

async function runWebPage(ctx, c, last) {
  const page = await ctx.newPage();
  const errors = [];
  page.on("pageerror", (e) => errors.push(e.message));
  try {
    const files = (c.webFiles || []).map((f) => ({ path: f, text: readFileSync(join(ROOT, f), "utf8") }));
    await page.addInitScript((files) => { window.RAPIDR_STUDIO_TEST_FILES = files; }, files);
    await page.addInitScript((env) => { window.RAPIDR_STUDIO_TEST = env; }, {
      RAPIDR_CAPTURE: "web",
      RAPIDR_CAPTURE_DELAY: String(c.delay),
      RAPIDR_TEST_DUMP: Object.keys(c.webDump || c.dump).join(","),
      ...(c.events ? { RAPIDR_TEST_EVENTS: c.events } : {}),
      ...(c.folder ? { RAPIDR_TEST_FILE_DIALOG: c.folder } : {}),
    });
    const q = new URLSearchParams({ theme: "rapidr-light", window: "normal" });
    if (c.fresh !== false) q.set("fresh", "");
    if (c.do) q.set("do", c.do.replaceAll("{dir}", `/flows/${c.name}`));
    if (c.open) q.set("open", c.open);
    await page.goto(`${URL_BASE}/index.html?${q}`, { waitUntil: "load" });
    await page.waitForFunction(() => window.rr && window.rr.rapidr_test_results(), null, { timeout: Math.max(90000, c.delay * 1000 + 60000), polling: 200 });
    const results = JSON.parse(await page.evaluate(() => window.rr.rapidr_test_results()));
    return { dump: parseDump(results.dump.join("\n"), Object.keys(c.webDump || c.dump)), errors };
  } finally {
    await page.close();
    if (last) await ctx.close();
  }
}

mkdirSync(WORK, { recursive: true });
const browser = await chromium.launch();
let passed = 0, failed = 0;
const check = (label, dump, c) => {
  for (const [k, re] of Object.entries(c.dump)) {
    const v = dump[k];
    const ok = v !== undefined && re.test(v);
    ok ? passed++ : failed++;
    console.log(`${ok ? "✓" : "✗"} ${c.name} (${label}): ${k} ${ok ? "" : `= ${JSON.stringify(v)} (wanted ${re})`}`);
  }
  for (const [k, file] of Object.entries(c.same || {})) {
    const want = readFileSync(join(ROOT, file), "utf8").replace(/\r\n/g, "\n").replace(/\n+$/, "");
    const v = dump[k];
    const ok = v !== undefined && v.replace(/\n+$/, "") === want;
    ok ? passed++ : failed++;
    console.log(`${ok ? "✓" : "✗"} ${c.name} (${label}): ${k} ${ok ? `equals ${file}` : `differs from ${file}`}`);
  }
};
for (const c of CASES.filter((c) => !filters.length || filters.some((f) => c.name.includes(f)))) {
  check("desktop", runDesktop(c), c);
  try {
    const web = await runWeb(browser, c);
    check("web", web.dump, c.webDump ? { ...c, dump: c.webDump } : c);
    if (web.errors.length) console.log(`  (page errors: ${web.errors.join("; ")})`);
  } catch (e) {
    failed++;
    console.log(`✗ ${c.name} (web): ${e.message.split("\n")[0]}`);
  }
}
await browser.close();
console.log(`\nRapidR Studio flows: ${passed} checks passed, ${failed} failed`);
process.exit(failed ? 1 : 0);
