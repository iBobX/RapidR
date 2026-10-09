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
id = "rapidr-basic"
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

### 3.1 RCodeEditor (I2, built: model `rapidr_value::objects::codeedit` over `rapidr-editor`'s document, kernel `components/codeeditor/`)

Kept exactly as before (the `code_editor` GUI case, unchanged on native, interpreted and web): Text ('\n' breaks), Lines, `Line(i)` (0-based), LineCount, SelStart, SelLength, SelText (characters), WhereX, WhereY (0-based), Modified, ReadOnly, MaxLength, CharCase, WantTabs, AddStrings, Clear, SelectAll, ClearSelection, GetSubList, GotoSub, `GotoLine(n)` (0-based), the clipboard methods, LoadFromFile / SaveToFile, OnChange (the user's edits only). Everything new counts lines and columns from 1 (a column is a character of the line).

| Kind | Member | Notes |
|---|---|---|
| Property | `Language` | An id (`"rapidr-basic"` default, `"sql"`, `"json"`, `"html"` …), a file name whose extension says, or a definition file (`.toml`) |
| Property | `ColorScheme` | `"auto"` (the theme's: `rapidr_value::code_scheme`) or `"classic"`, `"modern"`, `"dark"`, `"highcontrast"` |
| Property | `FontName` (JetBrains Mono), `FontSize` (points, 10), `TabSize`, `InsertSpaces`, `AutoClose`, `AutoIndent`, `WordWrap`, `ShowLineNumbers`, `ShowFolding`, `ShowMinimap`, `ShowWhitespace`, `HighlightCurrentLine`, `Rulers` (`"80,120"`) | |
| Property | `CaretLine`, `CaretColumn` | 1-based; setting moves the caret |
| Property | `CursorCount`, `CanUndo`, `CanRedo`, `DiagnosticCount`, `FoldCount`, `Encoding`, `Outline` (read-only) | `Outline`: kind, name, line, depth tab-separated per entry (the language service's, else the SUBs / FUNCTIONs) |
| Property | `CompletionTrigger` (`"."`), `LanguageService` (True), `KeywordCase` (`"upper"`, `"lower"`, `"proper"`, `"preserve"`) | The built-in service answers on every runtime (desktop and web install RapidR's at start); off, or for a language it doesn't serve, the request events |
| Property | `CompletionItems`, `CompletionSelected`, `HoverText`, `SignatureText` (read-only) | What the popups show now: the completion labels one a line, best first (fuzzy: the start as typed, any case, the word starts — `ss` → SelStart —, together, in order; lately accepted first among equals), the selected one, the hover's Markdown, the signature |
| Property | `FileName`, `LineEnding` (`"CRLF"`, `"LF"`, `"CR"`) | LoadFromFile keeps the file's breaks and encoding (UTF-8, UTF-8 BOM, Latin-1); SaveToFile writes them back |
| Method | `Undo`, `Redo` | Typing grouped by word / 400 ms |
| Method | `Find(Text, Options)` → found, `FindNext`, `FindPrevious`, `Replace(Find, With, Options)` → 1/0, `ReplaceAll(Find, With, Options)` → count | Options: `"case"`, `"word"`, `"regex"`, `"selection"`; `$1` in a regex's replacement |
| Method | `AddCursor(Line, Column)`, `SelectNextOccurrence`, `ClearCursors`, `GotoLineColumn(Line, Column)` | |
| Method | `Fold(Line)`, `Unfold(Line)`, `FoldAll`, `UnfoldAll` | |
| Method | `InsertText(Text)`, `ReplaceRange(StartLine, StartCol, EndLine, EndCol, Text)`, `ApplyEdits(Json)`, `ToggleComment`, `Indent`, `Outdent`, `CopyText` | `ApplyEdits`: `[{"line","column","endLine","endColumn","text"}]` or `start` / `end` offsets, one undo step, refused whole on an overlap |
| Method | `ApplyPatches(Patches, [Continues])` → done | One designer change's source patches as RDESIGNSURFACE's `OnSourceEdit` gives them, one per line (`StartLine⇥StartCol⇥EndLine⇥EndCol⇥Text`, lines from **0**, columns in characters, Text with `\n` `\t` `\\`), each in the text the ones before left: one undo step, or with `Continues` (OnSourceStep's) part of the step before; the carets keep their place in the text; OnChange once; all or nothing (ide-plan.md, "I2 / S-EDITOR results") |
| Method | `SetDiagnostics(Json)`, `ClearDiagnostics`, `AddMarker(Line, Kind)`, `RemoveMarker(Line, [Kind])`, `ClearMarkers([Kind])`, `GetMarkers([Kind])` → `"3,10"`, `HasMarker(Line, [Kind])` | Kinds: breakpoint, current, error, warning, bookmark, or the program's; markers follow their lines through edits |
| Method | `ShowCompletion(Items)`, `ShowHover(Text)`, `ShowSignature(Text, ActiveParam)`, `HidePopups` | Items: tab-separated lines (label, kind, detail, text to insert), a QSTRINGLIST, or JSON |
| Method | `TriggerCompletion`, `TriggerSignature`, `TriggerHover([Line, Column])`, `FormatDocument`, `GotoDefinition`, `FindReferences`, `Rename(NewName)`, `OpenFind([Mode])` | What the keys do, for a shell's menus and commands; Mode: `"find"`, `"replace"`, `"goto"`, `"rename"` |
| Method | `BeginUpdate`, `EndUpdate` | |
| Event | `OnCaretMove(Line, Column)`, `OnSelectionChange` | The user's moves |
| Event | `OnGutterClick(Line, Area)` | Area: `"marker"` (breakpoints), `"number"`, `"fold"` |
| Event | `OnCompletionRequest(Line, Column, Prefix)`, `OnHoverRequest(Line, Column)`, `OnSignatureRequest(Line, Column)` | Raised when no language service answers; the program answers with ShowCompletion / ShowHover / ShowSignature |
| Event | `OnSave` | Ctrl / Cmd+S inside the editor |
| Event | `OnNavigate(File, Line, Column)`, `OnReferences(Json)`, `OnFileEdits(File, Json)` | Go to definition in another file; Find references' places; a rename's edits to another file |
| AI tools | read text / selection / range, find, replace range, apply edits (diff-previewed), go to line | §1.5 |

Keys (VS Code's map; Cmd for Ctrl on macOS): arrows / Home (smart) / End / PgUp / PgDn, Ctrl+arrows by word, Ctrl+Home / End; Ctrl+Alt+Up / Down a caret above / below, Alt+click another caret, Alt+Shift+drag a column, Ctrl+D the next occurrence, Ctrl+Shift+L all of them, Escape back to one caret; Alt+Up / Down move lines, Shift+Alt+Up / Down copy them, Ctrl+Shift+K delete them, Ctrl+Enter / Ctrl+Shift+Enter a line below / above, Ctrl+/ comment, Tab / Shift+Tab (and Ctrl+] / Ctrl+[) indent, Ctrl+Shift+\ the matching bracket; Ctrl+Shift+[ / ] fold / unfold, Ctrl+K Ctrl+0 / Ctrl+J all; Ctrl+F find, Ctrl+H replace, F3 / Shift+F3, Ctrl+G go to line, F2 rename, F12 / Ctrl+click definition, Shift+F12 references, Shift+Alt+F format; Ctrl+Space completion, Ctrl+Shift+Space signature, Ctrl+K Ctrl+I hover; Ctrl+= / Ctrl+- / Ctrl+0 zoom (Ctrl+wheel too), Alt+Z word wrap; Ctrl+Z / Ctrl+Shift+Z / Ctrl+Y.


### 3.2 RDiffView (I2, built: `rapidr_value::objects::diffview`, kernel `components/diffview.rs`)

`LeftText`, `RightText` (setting either compares again; CR LF → LF), `Language` (an id, a file name or a definition file; default `"rapidr-basic"`), `Mode` (`"split"`, `"inline"`), `HunkCount`, `CurrentHunk` (0-based, -1: none; setting it scrolls there), `ResultText` (the left text with the accepted hunks' right lines — undecided counts as rejected), `AcceptedCount`, `RejectedCount`; `AcceptHunk(i)`, `RejectHunk(i)`, `AcceptAll`, `RejectAll`, `HunkState(i)` (1 / -1 / 0), `NextHunk`, `PreviousHunk` (wrap around; give the index); `OnHunkChange(Index, Accepted)` — the user's decisions only. The diff is our own Myers (linear space, lines only one side has set aside first; 2 × 20,000 lines with 100 changes in ~18 ms release); hunks have no context lines; paired lines get character marks. Keys: F7 / Alt+Down, Shift+F7 / Alt+Up, Enter / Ctrl+Y accept, Backspace / Ctrl+N reject (then the next hunk), arrows, Page Up / Down, Home / End. Used by the AI flow, "compare with saved" and merge conflicts.

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

### 3.4 RPropertyInspector (I1, built: `rapidr_value::panels::inspector`)

The language registry (`crates/rapidr-lang/data/components/ide.toml`) is the reference for every member; in short:

- **Properties**: `Target` (a component's name or Handle; several names separated by commas inspect them together), `Designer` (a designer whose selection it follows), `View` (`"categories"` / `"alphabetic"`), `Page` (`"properties"` / `"events"`), `Filter`, `ShowEvents`, `ShowRapidRExtensions`, `ReadOnly`, `Handlers` (the SUBs an event may run, one `Name(parameters)` per line), `NameWidth`, `Selected`, `RowCount`, `TargetType`, `Rows` (every row shown, `Name=Value` a line), `Doc` (the selected row in words), `EmptyText` (what it says while nothing is inspected). Following a designer, `Target` reads the names selected there.
- **Methods**: `Refresh`, `ExpandAll`, `CollapseAll`, `Expand(Name)`, `Collapse(Name)`, `Value(Prop)`, `SetValue(Prop, Value)`, `ResetValue(Prop)`, `IsDefault(Prop)`, `EditValue(Prop)`, `AddProperty(Name, Type, Value, Category)` / `ClearProperties` (the program's own objects), `Row(i)`.
- **Events**: `OnPropertyChange(Prop, Value)`, `OnEventDblClick(Event)`, `OnEditorRequest(Prop)`, `OnSelect(Prop)`.

Editors from the registry's types and `editor` key: number, text, Boolean, enum (a dropped list), set (a row per flag), colour (an inline picker: standard and system colours, a value field), font (Name, Size, Color, styles), picture / file (a "…" that raises OnEditorRequest), strings (a row per line), columns, component reference; **Anchors** has a visual pin editor. A value at its default is dimmed, a changed one bold; Delete or the reset glyph puts it back. RapidR-only members of a RapidQ component are grouped under "RapidR extensions" with an "R" badge.

**What it inspects** (`rapidr_value::panels::subject`): a `Subject` — the program's live components (`Live`), RDESIGNSURFACE's selection, or the designer model (`inspector::designer_model::DesignerModelSubject` over `rapidr_value::designer::Designer`: each change is one undoable `set_property`). A designer component registers its maker with `subject::register_designer(type, make)` and calls `inspector::designer_changed(host, name)` after each selection change or command; the inspector hands every change back as the program writes it (`set_source`: `alClient`, `akLeft + akTop`, `&H0000FF`). A subject may also say what an unset property is (`fallback`: a designer's laid-out Left / Top / Width / Height, shown dimmed). On an RDESIGNSURFACE each change is committed into the surface's code at once (`DesignSurface::commit`) and heard as the surface's own events (OnSourceEdit, OnChange); every inspector following a surface reads it again on its changes and selections (`objects::design`'s change hook).

**Handlers** (`rapidr_designer::Document::create_handler`, shared with the designer's double click): an event's SUB is the one its CREATE block names, else written with the registry's parameters — a `DECLARE SUB` after the file's last one and the SUB at its end when the file declares its SUBs, else the SUB just before the form's CREATE (RC.EXE wants a SUB known above the CREATE that names it) — and bound, in one undo step.

### 3.5 RProjectTree (I1, built: `rapidr_value::panels::project_tree`)

- **Properties**: `Project` (an `.rrproj`, or a `.bas` / `.rr` file and its `$INCLUDE`s, read through the runtime's file hooks), `ProjectName`, `ShowFiles`, `ShowForms`, `ShowComponents`, `Selected` (a path, `path#Component`, a folder as `path/`), `FileCount`, `Modified`, `EmptyText` (what it says with no project).
- **Methods**: `Refresh`, `LoadText(Text, Path)`, `SetFileText(Path, Text)`, `File(i)`, `FileKind(Path)`, `AddFile`, `RemoveFile`, `FileModified(Path[, On])` (a dot after a file with changes not saved), `NewFile(Kind[, Name])`, `Rename([Path], [NewName])`, `Delete([Path])` (an inline confirmation), `Reveal`, `Save`, `ProjectText`, `ExpandAll`, `CollapseAll`.
- **Events**: `OnOpen(Path)`, `OnSelect(Path)`, `OnRename(Old, New, Cancel)`, `OnDelete(Path, Cancel)`, `OnMove(Path, NewPath, Index)`, `OnNewFile(Path, Kind)`.

The program does the work on the disk in those events; the tree changes only the project. Files are grouped by kind, forms open into the components their CREATE blocks make, F2 renames in place, files drag to reorder (Alt+Up / Alt+Down from the keyboard).

### 3.6 RToolbox (I1, built: `rapidr_value::panels::toolbox`)

- **Properties**: `Filter`, `ShowNames` (`"rapidr"`, the default: RapidR's names, `RButton`; `"rapidq"`: as a file written with RapidQ's names writes them, QBUTTON for RapidQ's components; `"titles"`), `Selected`, `Count`.
- **Methods**: `Item(i)`, `Expand(Group)`, `Collapse(Group)`, `ExpandAll`, `CollapseAll`, `AddTemplate(Name, Source, Icon)`, `RemoveTemplate`, `Template(Name)`.
- **Events**: `OnPick(Type)`, `OnSelect(Type)`, `OnDragStart(Type)`, `OnDragDrop(Type, Target, X, Y)` (the deepest component under the drop, in its own pixels).

Groups come from `design/icons/inventory.toml` (`rapidr_icons::TOOLBOX_GROUPS`), by purpose (Standard, Additional, Dialogs, System, Network, Data, Data Science, Media, DirectX, Direct3D, Web, AI, IDE, Templates — none named after RapidQ or RapidR: R-NAMES); only components the compilers create are shown. Items and events name components by RapidR's names (`RButton`); a designer writes a component in its file's own style. Search is fuzzy, with the matched letters marked, by any of a component's names (`qbut` finds RButton); the words of a component's doc find it too ("chart" finds RPlot). An item's tooltip is its card: its name, what it is, its RapidQ name (or "RapidR's own"), desktop or web only.

### 3.7 RDockManager (I1, built: `rapidr_value::dock`)

| Kind | Member | Notes |
|---|---|---|
| Method | `AddPane(Component, Title, Where, Icon)` → True / False | Where: `"left"`, `"right"`, `"top"`, `"bottom"` (an outer edge), `"documents"`, `"float"`, `"tab:<pane>"` (into its group), `"<side>:<pane>"` (beside its group), `"<side>:documents"`, `"autohide:<side>"`. Component: the component or its Handle. Icon: a built-in name (`explorer`, `search`, `output`, `problems`, `properties`, `toolbox`, `outline`, `form`, `code`, `debug`, `database`, `chart`, `list`) or "" |
| Method | `ShowPane(Name)`, `HidePane(Name)`, `FloatPane(Name)`, `AutoHide(Name, On)`, `FocusPane(Name)`, `ClosePane(Name)`, `DockPane(Name, Where)`, `MovePane(Name)` | ShowPane puts a pane back where it was (or slides an auto-hidden one out); ClosePane closes a document (asking OnDocumentClose) or hides a tool pane; MovePane starts the keyboard's move with the compass |
| Method | `SaveLayout` → text, `LoadLayout(Text)` → True / False, `ResetLayout` | The text round-trips exactly (`Layout::save`'s format: `rapidr-dock 1`, the mode, the tree with each node's extent, auto-hidden, floating, documents, hidden) |
| Method | `Pane(i)` (0-based), `Document(i)`, `PaneTitle(Name[, Title])`, `PaneState(Name)` (`docked`, `tabbed`, `autohide`, `floating`, `document`, `hidden`), `PaneVisible(Name)`, `NextDocument`, `PreviousDocument` | Names come back lowercase (as the component registries keep them) |
| Method | `DocumentState(Name[, State])` → 0 normal, 1 minimized, 2 maximized (-1: not an MDI document) | An MDI document's window, set as its title bar's buttons would (RapidR Studio maximizes its start page) |
| Method | `Cascade`, `TileHorizontal`, `TileVertical`, `ArrangeIcons` | The documents' MDI client (`rapidr_value::mdi`) |
| Method | `AddView(Document, Component, Caption)`, `DocumentView(Name[, View])` → caption (`"Split"`: side by side) | (S-SHELL-2, tabbed documents) A document's views: two or more give its tab strip a switch at the right (a segment per view, then side by side); one view shows at a time, or the first two side by side with a splitter between them. `FocusPane` on a view shows that view (side by side stays so when it's one of the two). RapidR Studio: Design \| Code |
| Method | `DocumentModified(Name[, Modified])`, `SplitDocument(Name, Side)` | A dot on the tab of a document with changes not saved (the close button under the mouse); a document moved into a new group of tabs on a side of its group |
| Property | `DocumentMode` (`"mdi"`, the default (D4); `"tabs"`, RapidR Studio's), `ActiveDocument`, `ActivePane`, `PaneCount`, `DocumentCount`, `DocumentGroupCount`, `Layout` (= SaveLayout / LoadLayout) | |
| Event | `OnPaneChange(Name)`, `OnDocumentActivate(Name)`, `OnDocumentClose(Name, Cancel)`, `OnDocumentView(Name, View)`, `OnLayoutChange` | Cancel set keeps the document open |

Mouse: a tab or header shows its pane and makes it active; dragged, it brings up the docking compass (outer edges, and the cross over the group or documents under the mouse: beside, or into as a tab / a document) with an outline and a label of where it lands; dropped away from the compass it floats. A double click on a header floats a group (docks a floating one); the header's buttons auto-hide (pin) and close; splitters drag; an auto-hide strip's tab slides its pane out; a middle click closes a document tab.

Tabbed documents (VS Code's editor groups): a document's tab dragged along a strip reorders it (a bar shows where it goes); dragged over a group's page, its outer quarters split the group there (a new group on that side, its half outlined), its middle moves it into that group; the splitters between groups drag (the groups keep their shares when the area is resized). The layout's text keeps the groups (`groups split row` …, weights) and each document's view (`view <document> <first | n | split> <share per mille>`).

Keyboard: F6 / Shift+F6 between areas (the groups' shown panes, the active document), Ctrl+Tab / Ctrl+Shift+Tab between documents, Ctrl+Shift+M (or `MovePane`) moves the active pane: the arrows choose the compass's side (the same arrow again: the outer edge), Tab the next area, Space the centre, Enter docks, F floats, Escape stops; Escape in a slid-out pane slides it in.

### 3.8 Others

- **RToolBar** (I1, built: `rapidr_value::panels::toolbar`; RTOOLBAR stays a container for programs that place buttons on it): `AddButton(Name, Icon, Hint, [Command], [Caption])`, `AddToggle`, `AddSeparator([Name])`, `RemoveButton`, `Clear`, `Button(i)`, `ButtonEnabled` / `ButtonDown` / `ButtonVisible` / `ButtonHint(Name[, value])`; `ButtonSize`, `ShowCaptions`, `Customizable` (which buttons show, from the overflow menu), `Layout` (the hidden ones), `ButtonCount`, `ClickedButton`; `OnButtonClick(Name, Command)`, then `OnClick`. Buttons that don't fit go behind an overflow button.
- **ROutputConsole** (I1, built: `rapidr_value::panels::console`): pages Output (ANSI: colours, CLS, LOCATE, as the console writes them; `Font` is the output's, the tabs and problems are in the theme's chrome font; `EmptyText` while it's empty), Build and Problems; `Write`, `WriteLine`, `Clear([Page])`, `AddBuildLine`, `AddProblem(File, Line, Column, Severity, Message)`, `ClearProblems`, `Line(i)`, `Find(Text)`, `FindNext`; `Page`, `MaxLines`, `Filter`, `ShowTabs`, `AutoScroll`, `LineCount`, `ProblemCount`, `Text`; `OnLinkClick(File, Line)` (`file:line` places and problems), `OnPageChange(Page)`.
- **RCommandPalette** (I1, built: `rapidr_value::panels::palette`): `AddCommand(Id, Title, [Shortcut], [Category], [Icon])`, `AddPrefix(Prefix, Title)` (typing the prefix first gives one command made from the rest: `":"`, `"Go to line {}"` → OnCommand(`":42"`)), `RemoveCommand`, `Clear`, `CommandEnabled`, `Command(i)`, `Show`, `Hide`; `Filter`, `Placeholder`, `Count`, `CommandCount`, `Selected`, `MaxRows`; `OnCommand(Id)`, `OnCancel`. Fuzzy search over "Category: Title", the most recently used first.
- **RProgramView** (I5): shows a running program's remote form; `Session`, `FormId`, `InspectMode`; `OnInspect(Component)`.
- **Debugger views** (I6): **RBreakpointList**, **RCallStackView**, **RVariablesView** (`Scope`: `"locals"`, `"globals"`, `"watches"`; `AddWatch(Expr)`), **RImmediateWindow** — all bound to an RProgramSession (`Session = Program1`).
- **RDataPreview** (I7): `DataSet` (any dataset / frame), `Tab` (`"schema"`, `"rows"`, `"stats"`), `MaxRows`; `Refresh`; `OnCellClick(Row, Col)`.
- **RAIChat** (I8 / Phase 5): the assistant panel; [docs/ide-ai.md](ide-ai.md).

### 3.9 Non-visual

| Component | Members (summary) |
|---|---|
| **RProject** (built, I1: `crates/rapidr-studio`) | `FileName`, `Folder`, `Kind` (`project`, `file`, `v1`, ""), `Error`, `Name`, `MainFile`, `FileCount`, `CompatMode` (`"rapidq"` or `""`); `Open(Path)` → True / False (a `.rrproj` v2, a web IDE v1 project — its files written beside it where none is there yet —, or a `.rr` / `.bas` / `.rqw` / `.rqb` / `.rq` / `.inc` with what it includes), `ImportRapidQ(Source[, Dest])` → the copy's project file (File ▸ Import RapidQ Project or File: a copy with RapidR's names, proved, with its report and project; `ImportSummary`, `ImportReport`; on the web from and into the page's store), `Save([Path])`, `New(Template, Name, Folder)`, `AddFile(Path[, Kind])`, `RemoveFile(Path)`, `Close`, `File(i)`, `FileKind(i)`, `FullPath(i)`; Find in Files' `Find(Pattern, Options, Text)` → lines `line, column, length, line text` (Options: `case`, `word`, `regex`; the code editor's search), `Replace(Pattern, Options, Text, With)`, `FileText(Path)`; `OnChange`. Planned: `Build(Target)`, `OnBuildOutput(Text)`, `OnBuildDone(Ok)` |
| **RProgramSession** (built, I1) | `Program` (a source file, saved), `Args` (one string, COMMAND$'s), `Debug` (True), `BreakOnError`, `Theme` (the look of a program that names none: "" its default, `classic` for Studio's "Preview in classic"), `State` (`"stopped"`, `"running"`, `"paused"`), `CurrentFile`, `CurrentLine`, `ExitCode`, `Error`; `Start` → True / False, `Stop`, `Pause`, `Continue`, `StepIn`, `StepOver`, `StepOut`, `SetBreakpoint(File, Line[, Condition])`, `ClearBreakpoint(File, Line)`, `Evaluate(Expr)` (desktop; `? x` prints, a statement runs), `Input(Text)`; `OnOutput(Text)`, `OnStopped(Reason, File, Line)`, `OnContinue`, `OnExit(Code)`, `OnFormShown(Id)`. The program runs in its own process on the desktop (`rapidr run --session`, its forms real windows) and in a sandboxed frame on the web (the page's `RAPIDR_STUDIO_HOST`: ide/web/studio.js, run.html). Planned: `SeparateWindows`, `SetProperty(Object, Prop, Value)` |
| **RLanguageService** (built in part, I1) | `CompatMode`, `ErrorCount`; `Update(File, Text)`, `Close(File)`, `Outline(File)` → lines `depth, kind, name, detail, line`, `Diagnostics(File)` → lines `severity, line, column, message, file` (tab-separated), `Help(Word[, OfType])` → the registry's entry (title, syntax, what it is, doc, members: a line each; F1's Help pane). Planned (I3): `Project`; `Complete(File, Line, Col)` → items, `Hover`, `Signature`, `Definition`, `References`, `Rename(File, Line, Col, NewName)` → edits, `Format(File)` |
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
IDE.RegisterLanguageProvider "rapidr-basic", "diagnostics"
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
