# The IDE's building blocks as public components (planned API)

Every building block of RapidR's IDE is a public RapidR component that users can put in their own programs: a code editor for a SQL tool, a form designer for a report builder, a property inspector for a configuration app, a dock manager for any large application. The IDE is assembled from them ([docs/ide-plan.md](ide-plan.md) §3.2), so they are complete by construction. This document plans their API in RapidQ's style (properties, methods, events), the declarative language definitions, the data components of stage I7, and the extension model (stage I9).

Status: planned (2026-10-05). Names and members are proposals until each stage lands; the language registry (stage I0) becomes the reference once a component exists, and this document is then trimmed to what the registry doesn't say.

---

## 1. Rules for every public component

1. **R names.** New components are RapidR's own, so they have R names only (RCODEEDITOR, RFORMDESIGNER …), written that way by the designer ([docs/q-and-r-components.md](q-and-r-components.md)). A new name must not be "R + the rest of a RapidQ component's name" for a different component, since `canonical_type_name` maps `Q` + name to `R` + name.
2. **RapidQ conventions.** Properties are read and set like QFORM's; events are properties holding a SUB name (`OnChange = EditorChange`); handlers' first parameter is `Sender` where RapidQ's events have one; indexes are 0-based where RapidQ's are (`Line(i)`, `Item(i)`) and that's stated per member; strings lists are QSTRINGLIST-compatible; colours are RapidQ's BGR numbers; sizes are logical pixels.
3. **One model, both hosts.** Each component is a shared model in `rapidr-value` drawn by `rapidr-ui-kernel`; it behaves and looks the same native, interpreted and on the web, at every scale and theme.
4. **Accessible.** Each describes itself (role, name, value, states, actions) through `objects::a11y`; everything works from the keyboard.
5. **Described for AI.** Each component that does something an assistant might want offers a **tool provider**: a list of tools (name, description, JSON schema, read-only or mutating) and their implementations over the component's model. The IDE's MCP server is the union of its components' providers; in a user's program the same tools are offered to `RAI` only when the developer attaches the component (`AI.AttachComponent Editor1, "edit"`), with the `OnToolCall` veto ([docs/ide-ai.md](ide-ai.md) §8).
6. **In the registry.** Every member has a registry entry (type, default, docs written in our words, origin RapidR) — completion, the manual and the AI prompt come from there.
7. **No private hooks.** The IDE uses only what is documented here; what it needs, users get.

---

## 2. Language definitions (declarative)

RCodeEditor is language-agnostic. A language is a TOML file (bundled ones are built in; users and extensions add more). The tokenizer is a small state machine: each state has ordered rules (a regex, the token kind, an optional push / pop of a state); the state at a line's end is carried to the next line, so editing a line re-colours only until a line's end state is unchanged. Token kinds are a fixed vocabulary (keyword, keyword.control, type, function, variable, constant, number, string, string.escape, comment, operator, punctuation, directive, label, invalid, plus `.custom` names) that colour schemes map to colours.

```toml
[language]
id = "rapidq-basic"
name = "RapidQ / RapidR BASIC"
extensions = ["bas", "rr", "inc"]
case_insensitive = true
line_comment = ["'", "REM "]
word = '[A-Za-z_][A-Za-z0-9_]*[$%&!#]?'

[brackets]
pairs = [["(", ")"], ["[", "]"], ["{", "}"]]
auto_close = [["(", ")"], ["\"", "\""]]

[indent]
increase = '^\s*(SUB|FUNCTION|IF .* THEN\s*$|FOR|WHILE|DO|SELECT|CREATE|TYPE|WITH)\b'
decrease = '^\s*(END|NEXT|WEND|LOOP|ELSE|ELSEIF|CASE)\b'

[folding]
markers = [['^\s*SUB\b', '^\s*END SUB\b'], ['^\s*FUNCTION\b', '^\s*END FUNCTION\b'],
           ['^\s*CREATE\b', '^\s*END CREATE\b'], ['^\s*TYPE\b', '^\s*END TYPE\b']]

[keywords]                     # BASIC's lists are generated from the language registry
control = ["IF", "THEN", "ELSE", "FOR", "NEXT", "…"]

[[states.root]]
match = "'.*$"
token = "comment"
[[states.root]]
match = '"'
token = "string"
push = "string"
[[states.string]]
match = '"'
token = "string"
pop = true

[[snippets]]
prefix = "sub"
body = "SUB ${1:Name}(${2})\n\t$0\nEND SUB"
```

- Embedded languages: a rule can `embed = "rust"` until an end pattern (RapidR's `RUSTSTART … RUSTEND`, SQL inside `RDBQuery.SQL` strings later).
- RapidQ BASIC's semantic colours (which identifiers are SUBs, components, locals) come on top from the language service's semantic tokens, not from the definition.
- Colour schemes are TOML too (`[colors] keyword = "#0000B4"`, per theme: light, dark, high contrast), so the editor follows the IDE's theme.
- Built in: RapidQ / RapidR BASIC, plain text, JSON, SQL, CSV, Markdown, HTML, CSS, JavaScript, TOML, Rust.

---

## 3. Component reference

Only the members beyond the usual visual ones (Left, Top, Width, Height, Align, Anchors, Visible, Enabled, Font, Color, Hint, TabOrder, Parent, Tag, AccessibleName …) are listed.

### 3.1 RCodeEditor (stage I2; the existing RCODEEDITOR grown)

Kept exactly as today: Text, Lines, `Line(i)` (0-based), LineCount, SelStart, SelLength, SelText, WhereX, WhereY, Modified, ReadOnly, WantTabs, AddStrings, Clear, SelectAll, GetSubList, GotoSub, `GotoLine(n)` (0-based), OnChange.

| Kind | Member | Notes |
|---|---|---|
| Property | `Language` | A language id (`"rapidq-basic"`, `"sql"` …) or a definition file's path |
| Property | `ColorScheme` | `"auto"` (follows the theme) or a scheme name |
| Property | `TabSize`, `InsertSpaces`, `WordWrap`, `ShowLineNumbers`, `ShowFolding`, `ShowMinimap`, `ShowWhitespace`, `HighlightCurrentLine`, `Rulers` | |
| Property | `CaretLine`, `CaretColumn` | 1-based (as users see them) |
| Property | `CursorCount` (read-only), `CanUndo`, `CanRedo` | |
| Property | `CompletionTrigger` | Characters that raise OnCompletionRequest (default `"."`) |
| Method | `Undo`, `Redo` | Undo groups typing by word |
| Method | `Find(Text, Options)` → found, `FindNext`, `Replace(Find, With, Options)`, `ReplaceAll(Find, With, Options)` → count | Options: `"case"`, `"word"`, `"regex"`, `"selection"` |
| Method | `AddCursor(Line, Column)`, `SelectNextOccurrence`, `ClearCursors` | |
| Method | `Fold(Line)`, `Unfold(Line)`, `FoldAll`, `UnfoldAll` | |
| Method | `InsertText(Text)`, `ReplaceRange(StartLine, StartCol, EndLine, EndCol, Text)`, `ApplyEdits(Json)` | `ApplyEdits` is one undo step |
| Method | `SetDiagnostics(Json)`, `ClearDiagnostics`, `AddMarker(Line, Kind)`, `ClearMarkers(Kind)` | Kinds: breakpoint, current, error, warning, bookmark, custom |
| Method | `ShowCompletion(Items)`, `ShowHover(Text)`, `ShowSignature(Text, ActiveParam)`, `HidePopups` | Items: a QSTRINGLIST of tab-separated lines (label, kind, detail, text to insert) or JSON |
| Method | `LoadFromFile(File)`, `SaveToFile(File)`, `BeginUpdate`, `EndUpdate` | Line endings and encoding preserved |
| Event | `OnCaretMove(Line, Column)`, `OnSelectionChange` | |
| Event | `OnGutterClick(Line, Area)` | Area: `"number"`, `"marker"`, `"fold"` |
| Event | `OnCompletionRequest(Line, Column, Prefix)`, `OnHoverRequest(Line, Column)`, `OnSignatureRequest(Line, Column)` | The program answers with ShowCompletion / ShowHover / ShowSignature |
| Event | `OnSave` | Ctrl / Cmd+S inside the editor |
| AI tools | read text / selection / range, find, replace range, apply edits (diff-previewed), go to line | §1.5 |

### 3.2 RDiffView (I2)

`LeftText`, `RightText`, `Language`, `Mode` (`"split"`, `"inline"`), `HunkCount`, `AcceptHunk(i)`, `RejectHunk(i)`, `AcceptAll`, `RejectAll`, `ResultText`; `OnHunkChange(i, Accepted)`. Used by the AI flow, "compare with saved" and merge conflicts.

### 3.3 RFormDesigner (I4) and the existing RDESIGNSURFACE

| Kind | Member | Notes |
|---|---|---|
| Property | `Source` | The form's CREATE block text (or a file and form name) |
| Property | `GridSize` (8), `ShowGrid`, `SnapToGrid`, `SnapToGuides`, `ShowGuides`, `Zoom`, `PreviewTheme`, `PreviewScale` | |
| Property | `SelCount`, `Selected(i)` (names), `ComponentCount` | |
| Method | `AddComponent(Type, Name, X, Y, W, H, Parent)`, `Delete`, `SelectAll`, `Select(Name, Add)` | |
| Method | `Align(How)`, `Distribute(How)`, `SameSize(How)`, `CenterInParent(How)` | How: `"left"`, `"center"`, `"right"`, `"top"`, `"middle"`, `"bottom"`, `"horizontal"`, `"vertical"` |
| Method | `BringToFront`, `SendToBack`, `Nudge(DX, DY)`, `Undo`, `Redo`, `Cut`, `Copy`, `Paste` | |
| Method | `GetProperty(Name, Prop)`, `SetProperty(Names, Prop, Value)` | |
| Event | `OnSelectionChange`, `OnPropertyChange(Name, Prop)`, `OnComponentAdded(Name)`, `OnComponentRemoved(Name)`, `OnDblClick(Name)`, `OnSourceChange(Patch)` | `OnSourceChange` carries the minimal text edit |
| AI tools | list components, add / remove / move, set and link properties, align, create handlers | |

RDESIGNSURFACE keeps its members (AddComponent, GetName, GetType, GetCompX/Y/W/H, SetProp, GetProp, SetCompBounds, SetName, SelectComp, RemoveComponent, ClearAll, Count / CompCount, FormCaption; OnSelect, OnDblClick, OnBgClick, OnMove) answered by the same model, so existing programs keep working.

Companions: **RComponentTray** (the non-visual components of a form; `Designer`, `Links` to show link lines), **RTabOrderEditor** (`Designer`; a list to reorder), **RMenuEditor** (`Menu` — a QMAINMENU / QPOPUPMENU; captions, mnemonics, shortcuts, checked, enabled, separators, OnClick).

### 3.4 RPropertyInspector (I1)

`Target` (a component, or a designer's selection via `Designer`), `View` (`"categories"`, `"alphabetic"`), `Filter`, `ShowEvents`, `ShowRapidRExtensions`, `ReadOnly`; `Refresh`, `ExpandAll`, `CollapseAll`; `OnPropertyChange(Prop, Value)`, `OnEventDblClick(Event)`, `OnEditorRequest(Prop)` (custom editors). Editors come from the registry's property types (number, text, Boolean, enum, set, colour, font, picture / asset, strings, columns, component reference, SQL, expression, file). Users can inspect their own objects: properties the registry doesn't know are shown as text.

### 3.5 RProjectTree (I1)

`Project` (an RProject), `ShowFiles`, `ShowForms`, `ShowComponents`, `Selected`; `Refresh`, `Reveal(Path)`, `Rename`, `NewFile(Kind)`, `Delete` (asks); `OnOpen(Path)`, `OnSelect(Path)`, `OnRename(Old, New)`. Built on the tree view model; drag and drop to move.

### 3.6 RToolbox (I1)

`Groups` (from the registry; `"RapidQ"`, `"RapidR"`, user templates), `Filter`, `ShowNames` (`"as-written"`: QBUTTON for RapidQ components, R names for the rest), `Selected`; `AddTemplate(Name, Source)`; `OnPick(Type)`, `OnDragStart(Type)`.

### 3.7 RDockManager (I1, built: `rapidr_value::dock`)

| Kind | Member | Notes |
|---|---|---|
| Method | `AddPane(Component, Title, Where, Icon)` → True / False | Where: `"left"`, `"right"`, `"top"`, `"bottom"` (an outer edge), `"documents"`, `"float"`, `"tab:<pane>"` (into its group), `"<side>:<pane>"` (beside its group), `"<side>:documents"`, `"autohide:<side>"`. Component: the component or its Handle. Icon: a built-in name (`explorer`, `search`, `output`, `problems`, `properties`, `toolbox`, `outline`, `form`, `code`, `debug`, `database`, `chart`, `list`) or "" |
| Method | `ShowPane(Name)`, `HidePane(Name)`, `FloatPane(Name)`, `AutoHide(Name, On)`, `FocusPane(Name)`, `ClosePane(Name)`, `DockPane(Name, Where)`, `MovePane(Name)` | ShowPane puts a pane back where it was (or slides an auto-hidden one out); ClosePane closes a document (asking OnDocumentClose) or hides a tool pane; MovePane starts the keyboard's move with the compass |
| Method | `SaveLayout` → text, `LoadLayout(Text)` → True / False, `ResetLayout` | The text round-trips exactly (`Layout::save`'s format: `rapidr-dock 1`, the mode, the tree with each node's extent, auto-hidden, floating, documents, hidden) |
| Method | `Pane(i)` (0-based), `Document(i)`, `PaneTitle(Name[, Title])`, `PaneState(Name)` (`docked`, `tabbed`, `autohide`, `floating`, `document`, `hidden`), `PaneVisible(Name)`, `NextDocument`, `PreviousDocument` | Names come back lowercase (as the component registries keep them) |
| Method | `Cascade`, `TileHorizontal`, `TileVertical`, `ArrangeIcons` | The documents' MDI client (`rapidr_value::mdi`) |
| Property | `DocumentMode` (`"mdi"`, the default (D4); `"tabs"`), `ActiveDocument`, `ActivePane`, `PaneCount`, `DocumentCount`, `Layout` (= SaveLayout / LoadLayout) | |
| Event | `OnPaneChange(Name)`, `OnDocumentActivate(Name)`, `OnDocumentClose(Name, Cancel)`, `OnLayoutChange` | Cancel set keeps the document open |

Mouse: a tab or header shows its pane and makes it active; dragged, it brings up the docking compass (outer edges, and the cross over the group or documents under the mouse: beside, or into as a tab / a document) with an outline and a label of where it lands; dropped away from the compass it floats. A double click on a header floats a group (docks a floating one); the header's buttons auto-hide (pin) and close; splitters drag; an auto-hide strip's tab slides its pane out; a middle click closes a document tab.

Keyboard: F6 / Shift+F6 between areas (the groups' shown panes, the active document), Ctrl+Tab / Ctrl+Shift+Tab between documents, Ctrl+Shift+M (or `MovePane`) moves the active pane: the arrows choose the compass's side (the same arrow again: the outer edge), Tab the next area, Space the centre, Enter docks, F floats, Escape stops; Escape in a slid-out pane slides it in.

### 3.8 Others

- **RToolBar** (I1): a real kernel kind — `AddButton(Name, Icon, Hint, Command)`, `AddSeparator`, `Customizable`, overflow menu; OnClick(Name).
- **ROutputConsole** (I1): ANSI output (CLS / COLOR / LOCATE as the console has them), `Write`, `Clear`, `MaxLines`, `Filter`, clickable `file:line` links (`OnLinkClick(File, Line)`).
- **RCommandPalette** (I1): `AddCommand(Id, Title, Shortcut)`, `Show`; `OnCommand(Id)`.
- **RProgramView** (I5): shows a running program's remote form; `Session`, `FormId`, `InspectMode`; `OnInspect(Component)`.
- **Debugger views** (I6): **RBreakpointList**, **RCallStackView**, **RVariablesView** (`Scope`: `"locals"`, `"globals"`, `"watches"`; `AddWatch(Expr)`), **RImmediateWindow** — all bound to an RProgramSession (`Session = Program1`).
- **RDataPreview** (I7): `DataSet` (any dataset / frame), `Tab` (`"schema"`, `"rows"`, `"stats"`), `MaxRows`; `Refresh`; `OnCellClick(Row, Col)`.
- **RAIChat** (I8 / Phase 5): the assistant panel; [docs/ide-ai.md](ide-ai.md).

### 3.9 Non-visual

| Component | Members (summary) |
|---|---|
| **RProject** | `FileName`, `Name`, `MainFile`, `FileCount`, `File(i)`, `CompatMode` (`"rapidq"` or `""`); `Open`, `Save`, `New(Template)`, `AddFile`, `RemoveFile`, `Build(Target)`; `OnChange`, `OnBuildOutput(Text)`, `OnBuildDone(Ok)` |
| **RProgramSession** | `Program` (a file or project), `Args`, `Debug`, `SeparateWindows`, `State` (`"stopped"`, `"running"`, `"paused"`), `CurrentFile`, `CurrentLine`; `Start`, `Stop`, `Pause`, `Continue`, `StepIn`, `StepOver`, `StepOut`, `SetBreakpoint(File, Line, Condition)`, `Evaluate(Expr)`, `SetProperty(Object, Prop, Value)`; `OnOutput(Text)`, `OnStopped(Reason, File, Line)`, `OnExit(Code)`, `OnFormShown(Id)` |
| **RLanguageService** | `Project`; `Complete(File, Line, Col)` → items, `Hover`, `Signature`, `Definition`, `References`, `Rename(File, Line, Col, NewName)` → edits, `Diagnostics(File)`, `Outline(File)`, `Format(File)` |
| **RMCPServer** | `Enabled`, `Transport` (`"local"`, `"http"`), `Port`, `Permission`, `ClientCount`; `Start`, `Stop`, `AddProvider(Component)`, `ConnectionCommand` → text; `OnClientConnect(Name)`, `OnToolCall(Client, Tool, ArgsJson, Allow)` — lets users give their own programs an MCP server |
| **RAI** | Phase 5 / [docs/ide-ai.md](ide-ai.md) §8 |
| **RKeychain** | `Service`; `SetSecret(Name, Value)`, `GetSecret(Name)`, `DeleteSecret(Name)`, `Available` (False on the web unless the user opts in) — the OS keychain for users' programs |

---

## 4. Data components (stage I7)

All are R-only (no RapidQ counterpart) and additive. Links are component-reference properties, set in the designer with pickers or in code (`DS1.DataSet = Query1`).

| Component | Properties | Methods / events |
|---|---|---|
| **RDBConnection** | `Driver` (`"sqlite"`, `"mysql"`), `Database`, `Host`, `Port`, `User`, `Password` (never written by the designer), `Connected`, `ReadOnly` | `Connect`, `Disconnect`, `Execute(SQL, params…)`; OnConnect, OnError |
| **RDataFile** | `FileName`, `Format` (`"auto"`, `"csv"`, `"json"`, `"ndjson"`, `"parquet"`), `Delimiter`, `HasHeader`, `Encoding`, `Watch` | `Reload`; OnLoad, OnError |
| **RDBQuery** / **RDBTable** | `Connection`, `SQL` + `Params` / `TableName`, `Filter`, `OrderBy`, `Active`, `RecordCount`, `State`, `Field(Name)` | `Open`, `Close`, `First`, `Prior`, `Next`, `Last`, `Edit`, `Insert`, `Post`, `Cancel`, `Delete`, `Refresh`; OnAfterScroll, OnBeforePost, OnAfterPost |
| **RDataFrame** (existing) | + `Source` (a file, query, table or transform) | its current methods unchanged |
| **RDFFilter** | `Input`, `Condition` (a BASIC expression over columns) | OnChange |
| **RDFSort** | `Input`, `Columns`, `Descending` | |
| **RDFGroup** | `Input`, `GroupBy`, `Aggregates` (`"Sales:sum, Qty:mean"`) | |
| **RDFJoin** | `Left`, `Right`, `On`, `How` (`"inner"`, `"left"`, `"right"`, `"full"`) | |
| **RDFCompute** | `Input`, `Column`, `Expression` | |
| **RDFSelect** / **RDFLimit** | `Input`, `Columns`, `Rename` / `Rows`, `Offset` | |
| **RDataSource** | `DataSet`, `AutoEdit`, `Enabled`, `State` | OnDataChange, OnStateChange |
| **RDBGrid** | `DataSource`, `Columns` (editor), `ReadOnly` | OnCellClick, OnTitleClick |
| **RDBEdit**, **RDBLabel**, **RDBMemo**, **RDBCheckBox**, **RDBComboBox** | `DataSource`, `DataField` | OnChange |
| **RDBLookupCombo** | `DataSource`, `DataField`, `ListSource`, `KeyField`, `ListField` | |
| **RDBNavigator** | `DataSource`, `VisibleButtons` | OnClick(Button) |
| **RPlot** (existing) | + `DataSource` / `Frame`, `Kind`, `X`, `Y`, `Series`, `ColorBy`, `Title`, `XLabel`, `YLabel`, `Legend`, `Stacked` | its current methods unchanged |

Example (what the designer writes for "a CSV, grouped by month, as bars"):

```basic
CREATE Form1 AS QFORM
    Caption = "Sales"
    CREATE Sales AS RDATAFILE
        FileName = "data/sales.csv"
    END CREATE
    CREATE ByMonth AS RDFCOMPUTE
        Input = Sales
        Column = "Month"
        Expression = "FORMAT$(Date, ""yyyy-mm"")"
    END CREATE
    CREATE Totals AS RDFGROUP
        Input = ByMonth
        GroupBy = "Month"
        Aggregates = "Amount:sum"
    END CREATE
    CREATE Chart AS RPLOT
        Left = 8: Top = 8: Width = 480: Height = 300
        Frame = Totals
        Kind = "bar": X = "Month": Y = "Amount"
    END CREATE
END CREATE
```

---

## 5. Extensions (stage I9)

### 5.1 What an extension can contribute

| Contribution | Needs code | Needs permissions |
|---|---|---|
| Language definitions, colour schemes, snippets, icon themes | No | No |
| Toolbox templates (a component with preset properties, a CREATE snippet) | No | No |
| Keybindings, menu entries bound to built-in commands | No | No |
| Commands, panels (a form docked in the IDE), status bar items | Yes | `ui` |
| Completion, hover, signature, diagnostics, formatting, code actions for a language | Yes | `workspace.read` (for the document; granted implicitly for the document being served) |
| Project-wide tools (refactorings, generators) | Yes | `workspace.read`, `workspace.write` (edits go through the diff preview) |
| Network access (an API lookup) | Yes | `net:<host>` per host |
| Running programs / processes | Yes | `process` (desktop only; strongly warned) |
| AI tools (exposed to the assistant and MCP) | Yes | `ai.tools`; plus whatever the tools themselves touch |

### 5.2 Code: RapidR in a sandbox

- An extension's code is a RapidR program compiled to bytecode. It runs in **its own VM** (desktop: a thread of the IDE process with its own object store; web: a worker), under a **capability-filtered host** (ROADMAP Phase 5's `SandboxHost`): every host call — files, network, processes, the registry, the clipboard, FFI — is checked against the granted capabilities; `RUSTSTART` and `DECLARE … LIB` are refused at install (the compiler's flag); fuel (instructions per request) and memory limits stop runaways without freezing the IDE.
- It talks to the IDE through one object, `IDE`, and its events — the same RapidQ style as everything else:

```basic
' extension.rr — a diagnostics provider for "todo" comments
IDE.RegisterLanguageProvider "rapidq-basic", "diagnostics"
IDE.OnDiagnosticsRequest = CheckTodos

SUB CheckTodos(Doc AS STRING, Text AS STRING)
    DIM i AS INTEGER, L AS STRING
    FOR i = 0 TO TALLY(Text, CHR$(10))
        L = FIELD$(Text, CHR$(10), i + 1)
        IF INSTR(UCASE$(L), "TODO") THEN IDE.AddDiagnostic Doc, i + 1, 1, "info", "TODO left here"
    NEXT
END SUB
```

- The `IDE` object (planned members): `RegisterCommand(Id, Title, Sub)`, `RegisterLanguageProvider(Lang, Kind)`, `AddCompletion(Label, Kind, Insert, Detail)`, `AddDiagnostic(Doc, Line, Col, Severity, Message)`, `AddPanel(Form, Title, Where)`, `ActiveDocument`, `GetText(Doc)`, `ProposeEdit(Doc, Edits)` (diff-previewed), `ShowMessage`, `Settings(Key)`; events `OnCompletionRequest`, `OnHoverRequest`, `OnDiagnosticsRequest`, `OnFormatRequest`, `OnDocumentOpen`, `OnDocumentSave`, `OnActivate`, `OnDeactivate`.

### 5.3 Packaging

- A `.rrext` file is a zip: `extension.toml` (id, name, version, publisher, licence, minimum RapidR version, contributions, requested permissions), `main.rrbc` (if any code), `languages/`, `themes/`, `snippets/`, `icons/`, `LICENSE`, `README.md`.
- Installed per user (`~/.rapidr/extensions/<id>/<version>`; on the web in OPFS), from a file or a URL; the install dialog lists the permissions in plain words, and each can be refused (the extension then runs without it or not at all, as it declares).
- **Signatures** (optional, ed25519 over the zip's manifest of hashes): a signed package shows its publisher; an unsigned one shows a warning. Updates keep the granted permissions only if no new ones are asked.
- An extension's licence is the publisher's; RapidR's own bundled extensions are MIT.
- Projects can recommend extensions (`[extensions] recommended = [...]` in `.rrproj`), never install them.
