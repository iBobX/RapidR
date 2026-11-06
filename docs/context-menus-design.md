# Context menus and row actions: design

Status: **approved by Robert on 2026-11-06 (D1–D8 as proposed); Phases 2a and 2b built** (sections 9 and 10).
Mockups (light and dark): [design/context-menus/](../design/context-menus/), drawn in
Pen (the source, `design/context-menus/context-menus.pen`, joins them once it is
saved from the Pen app).

Robert, 2026-11-06: right-click on files, on chats, everywhere, "everybody has
that muscle memory, you always try right-click first". And the RAI chat list's
big Open / Rename / Delete buttons should become small icon actions on each row.

This page says which menu each part of RapidR Studio gets, in which order, with
which keys, what is greyed or hidden and when, and how the same thing becomes a
component feature that people's own programs get too.

## 1. What people expect, and what they dislike

From a short look at the best development tools, file managers and AI chat apps,
and at what their users praise and complain about (private notes in the lane's
scratch folder; no product is named here):

- **Liked:** short menus that change with what was clicked; the same verb in the
  same place everywhere; the shortcut shown next to the item (people learn it
  there); the default action first and in bold; right-click selecting what is
  under the pointer first; Copy Path, Copy Relative Path and Reveal in the file
  manager.
- **Disliked:** long menus full of rarely used items; deep submenus; common verbs
  hidden behind a "more options" item or behind unlabelled icons; a different
  order in similar places; Delete next to Open or Rename (mis-clicks); destructive
  items without a way back; actions that appear only under the mouse (no touch,
  no keyboard); menus the keyboard can't open, or that open at the mouse when the
  keyboard opened them; greyed items with no reason.
- **Accessibility practice:** Shift+F10 and the Menu key open the menu of the
  focused thing, placed at that thing; screen readers have their own "show menu"
  command; row actions must appear on keyboard focus too, never only on hover,
  and a list must say that its rows have actions.

## 2. The rules every Studio menu follows

**R1. Right-click selects first.** What is under the pointer is selected before
the menu opens. If it is already part of a multiple selection (the designer, a
list), the selection stays and the menu acts on all of it. Right-clicking empty
space gives the container's menu (the project, the form, the list).

**R2. Every way to open it.** The right mouse button; **Shift+F10** and the
**Menu key**; a long press (half a second) on touch screens and in the browser;
and the screen reader's own "show menu" command (VoiceOver's VO+Shift+M,
Narrator's Shift+F10), through the accessibility tree's show-context-menu action.
A menu opened from the keyboard sits **under the focused item** (at its text,
below its row), never at the mouse, and its first enabled item is chosen. Arrows,
Home/End, the underlined letters, Enter; Esc closes it and gives the focus back
to the item.

**R3. One order of groups, everywhere.** Empty groups vanish; separators never
double.

| # | Group | Examples |
|---|---|---|
| 1 | What it is for | Open (bold: what a double click or Enter does), Go to Handler, Edit Value, the problem's fix |
| 2 | Make something here | New File Here..., Add Existing File... |
| 3 | Edit it | Cut, Copy, Paste, Duplicate, Rename |
| 4 | Find and show | Go to Definition, Find References, Copy Path, Reveal in Finder, Show in Project |
| 5 | The mode's own | Toggle Breakpoint, Run to Cursor (debugging); Compare With, Show History (Versions); Align (designer) |
| 6 | Ask RAI | one item, "Ask RAI About ..." |
| 7 | What loses something | Remove from Project, Delete File..., Remove Watch, Clear |

**R4. Short.** At most 14 items and 7 groups. One submenu level at most, and a
submenu holds at least three items. The first group and the last (destructive)
group are never inside a submenu.

**R5. Hidden or greyed.** An item that never applies to this kind of thing is
hidden (a folder has no Open With). An item that applies but not right now is
greyed, and its hint says why ("Nothing is selected", "The main file stays in the
project"): shown as a tooltip and read by screen readers. So the same verb stays
in the same place and nothing jumps.

**R6. Words and keys.** The same words as Studio's main menus, because every item
comes from the same command (R9): Title Case, the verb first, `...` when more is
asked before it acts, an underlined letter unique in the menu. The command's own
shortcut, right-aligned, in the secondary text colour (`page.dim`). Mode
words follow the system where the person must recognise them: "Reveal in Finder"
on macOS, "in File Explorer" on Windows, "in Files" on Linux (Studio's
`Proj.FileManager`); hidden in the browser.

**R7. What loses something.** Last, after a separator. Its words say what is lost
("Remove from Project" keeps the file on disk; "Delete File..." moves it to the
Trash). If it can be undone at once, it acts and offers Undo (a chat, a watch).
If it touches the disk or can't be undone, it asks first with the small
confirmation of the design guide (a question, one sentence, verb buttons, the
safe answer the default). No red text: the words, the place and the question say
it (design guide: calm).

**R8. RAI's item.** Always worded "Ask RAI About ..." (This File, This Code, This
Problem, Caption, These Components), always in group 6. It **never sends by
itself**: it opens the Assistant with the thing quoted in the question box (a
file's name, the selected code, the problem's text and line) and the cursor
after it, so the person adds their question and presses Send. Nothing leaves the
computer without that press. Hidden when School Lock turns AI off; greyed with
the hint "Set up RAI first: Tools > Set Up RAI" when no model is chosen.

**R9. One implementation.** Every item is a command of Studio's command table
(`ide/commands.inc`): its title, keys, icon and "enabled when" come from the one
row the main menus, the tool bar and the Command Palette use. A new verb is first
a new command, so it is also in the Command Palette, in Keyboard Shortcuts, and
callable by RAI and AI apps. A menu is a list of command ids and separators.

**R10. Drawn by RapidR, seen by everyone.** Studio's menus are drawn by RapidR's
UI kernel (the same panels as today's AI right-click), so they follow the theme,
show in screenshots and recordings, are listed by `app_layout` and can be picked
by `app_click {menu: ...}`. For screen readers the menu is a menu named after its
target ("Actions for orders.rr"), each item a menu item with its hint as the
description.

**R11. Sizes.** In RapidR's looks: rows 24 px (today 20), the panel's 4-px padding
and 8-px corners, a 24-px gutter for check marks, keys in `page.dim`, separators
9 px. The classic look is unchanged (RapidQ's 20-px rows and raised edge). See
decision D1.

## 3. Row actions on lists

Some lists have two or three verbs used so often that a menu is one step too
many: the RAI chats above all. They get small icon buttons on the row itself.

- **When they show:** on the row under the mouse, **and** on the row with the
  keyboard focus (and on the selected row on a touch screen). Never on hover
  alone. While they show, the row's secondary text (a time, a count) steps aside.
- **Shape:** 16-px icons from RapidR's own set (`rename`, `delete`, `more`) in
  24 x 24 targets, 4 apart, 8 from the row's right edge, in `page.dim`; under the
  mouse, a soft rounded `hot` square. At most three; the last one is **More**
  (...), which opens the row's context menu (the same menu as a right-click).
- **Never the only way:** every row action is also in the row's menu and has a
  key on the focused row (F2 Rename, Del Delete, Shift+F10 More). The list has
  one Tab stop; the keys work on the focused row, so a keyboard user never
  tabs through dozens of small buttons.
- **Named:** each icon has a tooltip with its key ("Rename (F2)") and an
  accessible name ("Rename the chat Why does Greet not run?"); the row's
  description says "Actions: Rename F2, Delete Del, more with Shift+F10".

**The RAI chat list** (mockup 02):

- A click opens the chat (today a double click, or Open). Up and Down move
  without opening; Enter opens.
- Each row: the title (bold for the chat open now, instead of "(open)"), its time
  at the right; Rename, Delete and More on the hovered and the focused row.
- Rename edits the title in place (Enter keeps, Esc cancels); the rename strip
  at the bottom goes.
- Delete removes it at once and shows the existing Undo strip (no question: it
  can be undone).
- The footer's **Open / Rename / Delete buttons are removed.** New Chat stays in
  the header, compact. An empty list shows one line and the one big button left:
  **Start a Chat** (primary).
- The chats are grouped by Today / Yesterday / Earlier, so a long list scans.
- The row's menu: **Open** (bold, Enter) | Rename (F2), Copy as Markdown, Move to
  This Project (greyed unless the chat is a no-project chat and a project is
  open: the memory note's "Bring this chat along") | Delete (Del).

**Other lists that get row actions in Phase 2:** the Welcome page's recent
projects (Remove from Recent), Watch (Remove), Breakpoints (Remove; its check
box stays the enable switch). Each is a small change once the component has it.

## 4. The inventory: every surface and its menu

Legend: **bold** is the default action; `|` separates groups; keys in brackets;
"NEW" marks a command Studio doesn't have yet (it is added to the command table
first, R9). "Today" says what exists now.

### 4.1 Project tree

Today: no menu. F2 renames, Del removes from the project (a confirm strip at the
tree's bottom), Enter opens.

| Right-clicked | Menu (in order) | Greyed / hidden | Commands |
|---|---|---|---|
| A source file (form, module, include) | **Open** (Enter); Open Code (F7) for a form \| New File Here..., Add Existing File... \| Rename (F2), Copy Path, Copy Relative Path, Reveal in Finder \| Compare With > (the Disk's Version, the Last Version, Another File...), Show History \| Ask RAI About This File... \| Remove from Project (Del), Delete File... | Remove from Project greyed for the main file ("The main file stays in the project"); Compare With > the Last Version and Show History hidden when the project isn't versioned; Reveal hidden on the web | existing: `project.addFile` (renamed "Add Existing File..."), `view.code`, `versions.localHistory`, `file.compareDisk`; NEW: `project.openFile`, `project.newFileHere`, `project.rename` (the tree's F2 as a command), `file.copyPath`, `file.copyRelativePath`, `file.reveal` (`Proj.Reveal`, today the build log's link), `file.compareWith`, `ai.askAbout`, `project.removeFile` (the tree's Del as a command), `file.deleteFile` |
| A non-code file (README.md, a picture, data) | **Open** (Preview for .md, pictures, SVG); Open With > (Preview, Code Editor) \| the same groups as a source file | Open With hidden when only one way exists | as above, NEW `file.openWith` |
| A file outside the project's folder (linked) | as a source file, without Delete File... | Delete File... hidden: Studio never deletes files outside the project's folder | |
| A folder | New File Here..., Add Existing File..., Add Folder... \| Rename (F2), Copy Path, Copy Relative Path, Reveal in Finder \| Collapse All \| Remove from Project (Del) | | `project.addFolder`, NEW `tree.collapseAll` |
| A group (Forms, Modules, Assets...) | Add Form / Add Module / Add Existing File... (as the group fits) \| Collapse All | | `project.addForm`, `project.addModule` |
| The project (root) or empty space | Add Form, Add Module, Add Existing File..., Add Folder... \| Copy Path, Reveal in Finder \| Start Versioning / Save a Version... \| Ask RAI About This Project... \| Project Options... \| Close Project | | `versions.start`, `versions.save`, `project.options`, `file.closeProject` |
| A form's component (under its file) | **Select in the Designer** \| Copy Name \| Help: RButton (F1) | | NEW `designer.selectComponent` |

### 4.2 Document tabs

Today: no menu (Close is the tab's x and Ctrl+W).

| Right-clicked | Menu | Greyed / hidden | Commands |
|---|---|---|---|
| A tab (mockup 05) | Close (Ctrl+W), Close Others, Close to the Right, Close Saved, Close All \| Copy Path, Copy Relative Path, Reveal in Finder, Show in Project \| Split Right, Split Down \| Local History of This File... | Close Others / to the Right greyed when nothing to close; an unsaved file asks first, as Close does | existing: `file.close`, `file.closeAll`, `view.splitVertically`, `view.splitHorizontally`, `versions.localHistory`; NEW: `file.closeOthers`, `file.closeToRight`, `file.closeSaved`, `project.showFile`, and the path commands of 4.1 |
| The strip's empty part | New File..., Reopen Closed Document (Ctrl+Shift+T) \| Close All | | NEW `file.reopenClosed` |

### 4.3 Code editor

Today: Go to Definition (F12), Find References (Shift+F12), Rename Symbol... (F2)
| Cut, Copy, Paste | Select All, Find in Files... (always the same menu).

| Right-clicked | Menu | Greyed / hidden | Commands |
|---|---|---|---|
| Plain code, nothing selected | Go to Definition (F12), Find References (Shift+F12), Rename Symbol... (F2) \| Cut, Copy, Paste \| Toggle Breakpoint (F9), Run to Cursor (Ctrl+F10) \| Toggle Comment (Ctrl+/), Versions > (Show This Line's Change, Undo This Line's Change, Local History of This File...) \| Ask RAI About This Code... | the language group greyed when not on a name; Cut and Copy greyed with nothing selected; Versions hidden when not versioned | existing `edit.*`, `debug.toggleBreakpoint`, `debug.runToCursor`, `edit.toggleComment`, `versions.*` |
| A selection | the same; Ask RAI About This Code... quotes the selection; Copy with Formatting joins Copy (for worksheets and slides) | | NEW `edit.copyFormatted` |
| On a SUB or FUNCTION name | the same; the language group fully enabled (it's a name) | | |
| On a problem's squiggle (mockup 03) | **the fix itself** (e.g. Change to Text), Quick Fix... (Ctrl+.), Explain This Problem (its Help page) \| then the plain-code groups \| Ask RAI About This Problem... | | `edit.quickFix`, NEW `edit.explainProblem` |
| While paused in the debugger | adds Set Next Statement (Ctrl+Shift+F10), Add Watch to the debug group | hidden when not paused | `debug.setNextStatement`, `debug.addWatch` |
| The margin (line numbers, breakpoints) | Toggle Breakpoint (F9), Edit Breakpoint... (Shift+F9), Disable Breakpoint \| Run to Here \| Show This Line's Change | | NEW `debug.disableBreakpoint` |

Select All and Find in Files leave the editor's menu (they stay in Edit, with
their keys): the menu becomes about the place clicked.

### 4.4 Form designer

Today: no menu (the Format menu has the commands).

| Right-clicked | Menu | Greyed / hidden | Commands |
|---|---|---|---|
| One component | **Edit OnClick Handler** (its main event: what a double click does), Edit Caption (F2) \| Cut, Copy, Paste, Duplicate (Ctrl+D) \| Bring to Front, Send to Back, Center in Form > (Horizontally, Vertically) \| Tab Order..., Properties (F4), Help: RButton (F1) \| Ask RAI About This Component... \| Delete (Del) | Edit Caption hidden for components without a caption | NEW `designer.editHandler`, `designer.editCaption`, `edit.*`, `designer.*`, `designer.tabOrder`, `view.properties`, NEW `help.component` |
| Several components (mockup 04) | Align > (Lefts, Rights, Tops, Bottoms, Centers, Middles), Make Same Size > (Width, Height, Size), Space Evenly > (Across, Down) \| Cut, Copy, Paste, Duplicate \| Bring to Front, Send to Back \| Properties (F4) \| Ask RAI About These Components... \| Delete (Del) | Space Evenly greyed with fewer than three | existing `designer.align*`, `designer.same*`, `designer.space*` |
| The form (empty part) | Paste (Ctrl+V), Select All (Ctrl+A) \| Menu Editor..., Tab Order... \| Zoom > (In, Out, Actual Size, to Fit) \| View Code (F7), Properties (F4), Help: RForm (F1) \| Ask RAI About This Form... | | `designer.menuEditor`, `designer.zoom*`, `view.code` |

### 4.5 Properties and Events

Today: only the value field's text menu while editing.

| Right-clicked | Menu | Greyed / hidden | Commands |
|---|---|---|---|
| A property (mockup 07) | **Edit Value** (Enter), Reset to Default (Del) \| Copy Value (Ctrl+C), Copy Name, Copy as Code \| Help: Caption (F1) \| Ask RAI About Caption... | Reset greyed at the default ("Already the default") | NEW `inspector.edit`, `inspector.reset` (the row's Del), NEW `inspector.copyValue`, `inspector.copyName`, `inspector.copyCode`, `help.member` |
| An event with a handler | **Go to Handler** (Enter), Remove Handler (Del: the event's assignment goes, the SUB stays) \| Copy Name, Copy as Code \| Help: OnClick (F1) \| Ask RAI About OnClick... | | `inspector.event` (today's double click), NEW `inspector.removeHandler` |
| An event without one | **Create Handler** (Enter) \| Copy Name \| Help \| Ask RAI | | |
| A category heading | Collapse All, Expand All | | NEW |

### 4.6 Toolbox

| Right-clicked | Menu | Commands |
|---|---|---|
| A component | **Add to the Form** (Enter) \| Help: RButton (F1), Open an Example \| Ask RAI How to Use RButton... | `toolbox.pick` (today's OnPick), `help.component`, NEW `help.example` |
| A group | Collapse All, Expand All | |

### 4.7 Problems, Output, Immediate

Today: Copy (Ctrl+C), Copy Line, Copy All / Copy All Problems, Select All,
Wrap Long Lines, Clear.

| Right-clicked | Menu | Commands |
|---|---|---|
| A problem | **Go to Problem** (Enter), Quick Fix... (Ctrl+.), Explain This Problem \| Copy (Ctrl+C), Copy All Problems \| Ask RAI About This Problem... | NEW `problems.goTo`, `edit.quickFix`, `edit.explainProblem` |
| An Output line | Go to main.rr, Line 12 (only on a line with a place) \| Copy (Ctrl+C), Copy Line, Copy All, Select All (Ctrl+A) \| Wrap Long Lines (check) \| Clear | today's items, reordered; NEW go-to item |
| Immediate | Copy, Paste, Select All \| Clear | |

### 4.8 Debug panes

| Right-clicked | Menu (mockup 08 shows Watch) | Today |
|---|---|---|
| Variables | Set Value... (F2) \| Copy Value (Ctrl+C), Copy Expression, View Text... (strings) \| Add Watch | the same five items in another order |
| Watch | Set Value... (F2) \| Copy Value, Copy Expression, View Text... \| Remove Watch (Del) | the Del key existed, the menu didn't say so |
| Call Stack | **Go to This Call** (Enter) \| Copy (Ctrl+C), Copy All | Copy, Copy All |
| Breakpoints | **Go to Breakpoint** (Enter), Edit Condition... (Shift+F9), Disable \| Copy, Copy All \| Remove Breakpoint (Del), Delete All Breakpoints... (Ctrl+Shift+F9) | Copy, Copy All |

### 4.9 Versions timeline

Today, already right: What Changed? | Go Back to This Version..., Go Back to This
Version for This File... | Undo: what. Right click, Shift+F10 and the Menu key
open it under the row (the pattern every other surface follows). Kept; only the
default item becomes bold.

### 4.10 Help

| Right-clicked | Menu |
|---|---|
| A page | Back (Alt+Left), Forward (Alt+Right) \| Copy (Ctrl+C), Select All \| Open in the Browser \| Ask RAI About This Page... |
| A code example | **Try It** (opens the example in Studio), Copy Code, Insert at the Cursor \| Ask RAI About This Example... |

### 4.11 RAI's chat

| Right-clicked | Menu (mockup 06) |
|---|---|
| An answer | Copy as Text (Ctrl+C), Copy as Markdown, Copy with Formatting \| Quote in a New Question, Retry (the last answer only) \| Select All |
| A code block in an answer | Copy Code (Ctrl+C), Insert at the Cursor \| Quote in a New Question |
| Your question | **Edit Question...** \| Copy as Text, Quote in a New Question |
| A chat in the list | see section 3 |

Under each answer, on hover and on keyboard focus: Copy (as text), Retry (last
answer), More (the menu). "Copy with Formatting" puts rich text on the clipboard,
so a word processor keeps the headings, lists and bold (Robert, 11-02).

### 4.12 Status bar, panes, Welcome

| Right-clicked | Menu |
|---|---|
| The status message | Copy Message |
| The project and its version | Reveal in Finder, Project Options... \| Show Versions |
| Ln, Col | Go to Line... (Ctrl+G) |
| The AI apps status | AI Apps Activity, AI Apps Settings... |
| School Lock's chip | School Lock... |
| A pane's tab or header | Close Pane \| Float / Dock \| Reset Layout |
| A recent project on the Welcome page | **Open** \| Copy Path, Reveal in Finder \| Remove from Recent |

## 5. What it becomes in RapidR (Phase 2, after approval)

**One event on every visual component** (a RapidR addition; classic programs
behave as before):

```basic
SUB TreeMenu(X AS INTEGER, Y AS INTEGER, Key AS INTEGER, Sender AS RProjectTree)
  ' X, Y: where the menu goes, in the component (under the item when Key = 1)
  ' Key: 0 the mouse, 1 the keyboard (Shift+F10 / the Menu key),
  '      2 a long press, 3 a screen reader's "show menu"
  FileMenu.PopupAt Sender, X, Y
END SUB
ProjectTree.OnContextMenu = TreeMenu
```

- Before it fires, the item under the pointer is selected (R1) and the component
  says which one: `RProjectTree.ContextKey`, `RListBox.ItemIndex`, `RTreeView`'s
  selected item, `RGrid.Row` / `Col`, `RCodeEditor.ContextLine` / `ContextColumn`
  / `ContextWord` (with the problem there, if any), `RDesignSurface`'s selection,
  `RPropertyInspector.ContextProperty`.
- A component with a `PopupMenu` and no `OnContextMenu` keeps today's behaviour
  for the mouse; Shift+F10 and the Menu key open it too, at the focused item.
  That keyboard part is checked against real RapidQ first (RC.EXE in the
  Windows VM); if RapidQ didn't do it, it is RapidR-only and classic programs
  are left exactly as they were.
- **Row actions as a component feature:** `RListBox`, `RListView` and `RTreeView`
  get `RowActions` (a list of action names with their icons, e.g.
  `"rename,delete,more"`) and `OnRowAction(Index, Action)`; drawn by the kernel as
  in section 3, keys F2 / Del / Shift+F10 included, named for screen readers.
- **Kernel-drawn pop-up menus:** `RPopupMenu` gets a way to be drawn by RapidR
  instead of the system (decision D6); Studio uses it for every menu.
- **AI:** `app_layout` already lists an open menu's items and `app_click {menu}`
  picks them. Added: `app_click` with `button: "right"` on an item (or the
  keyboard's Shift+F10) opens its menu, so RAI and AI apps use Studio's menus
  exactly as a person does; one parity test for RAI and the MCP app.
- Desktop and web share the model (web parity); Spanish texts through
  `rapidr i18n studio`; the manual's Studio pages get a "Right-click menus"
  section with the pictures; `examples/gui/context_menus.rr` shows a tree and a
  list with OnContextMenu, row actions and keyboard opening; Help and the
  registry's AI facets list the event and the properties.

Tests: kernel unit tests (keyboard opening places the menu under the item, first
item chosen, Esc returns focus, a screen reader's show-menu action); the shared
row-actions model; `tests/studio_flows.mjs` flows per surface, by mouse and by
keyboard; `tests/app_eyes.mjs` for RAI and the MCP app; Studio goldens for the
open menus in the three looks; a real walk with VoiceOver on macOS.

## 6. Decisions for Robert

| | Question | Proposal |
|---|---|---|
| D1 | Menu rows in RapidR's looks | **24 px** (today 20): a 24-px target (WCAG 2.5.8), easier on touch and the web. The classic look keeps 20. Also changes the menus of RapidR-look programs (menus float; no form layout moves). |
| D2 | Icons in menus | **None** (only check marks and submenu arrows), so the words line up and scan. Icons stay in tool bars and row actions. |
| D3 | Destructive items in red? | **No**: last place, clear words, a question or Undo. Calm, as the design guide asks. |
| D4 | RAI items | **Prepare the question, never send** (privacy: nothing leaves without Send). |
| D5 | Chat list | **A single click opens a chat** (today a double click or Open). |
| D6 | Who draws the menus | **RapidR's kernel, in Studio on every system** (themed, seen by RAI and AI apps, in screenshots and videos). Programs keep the system's menu by default and can choose the kernel's. |
| D7 | Del in the Project tree | Stays **Remove from Project** (the file stays on disk), as today; Delete File... has no key and asks first. |
| D8 | Phase 2 in two steps | **2a:** the component event + row actions, Project tree, chats, document tabs, code editor, designer, Properties. **2b:** the rest of section 4 (Problems, Output, debug panes, Help, status bar, Welcome, Toolbox). |

## 7. The mockups

Eight surfaces, each in RapidR Light and RapidR Dark, 2x, with a one-line note at
the bottom: [design/context-menus/](../design/context-menus/).

| # | Surface | Shows |
|---|---|---|
| 01 | Project tree, a file | right-click selection, bold default, keys, destructive last |
| 02 | RAI chats | row actions on hover and on focus, the row's menu from More, empty state, Undo after Delete, rename in place |
| 03 | Code editor on a problem | the problem's group first, greyed (not hidden) Cut and Copy, one submenu |
| 04 | Form designer, three selected | group-only commands first, one submenu level |
| 05 | Document tab | the Close family, the shared path words |
| 06 | RAI answer | Copy as Text / as Markdown / with Formatting, icons under the answer, code block and question menus |
| 07 | Properties and Events | bold default, Reset greyed at the default, an event's menu |
| 08 | Watch from the keyboard | a menu placed under the focused row, first item chosen |

The icons in the row actions are RapidR's own (`rename`, `delete`, `copy`,
`refresh`, `more`, `ai-chat` from design/icons); the tree and tab icons in the
mockups are stand-ins for Studio's file icons.

## 8. The five pillars

- **Fast:** menus are built from the command table at open time (a few dozen
  rows); nothing is computed until the right-click. Row actions are drawn only on
  the hovered and focused rows.
- **Easy to use:** right-click works where people try it first; the same verbs in
  the same places; the keys shown teach them; the chat list loses three big
  buttons and gains direct actions.
- **Accessible:** every menu opens from the keyboard and from a screen reader,
  placed at the item; row actions show on focus, have keys, names and tooltips;
  greyed items say why; 24-px targets.
- **Beautiful:** one menu look, from theme tokens, in light, dark, high contrast
  and classic; calm destructive items; no icon clutter in menus.
- **Legal:** no new dependency; RapidR's own icons and words; no other product
  named; AI items never send anything by themselves.

## 9. Phase 2a: what is built, what waits

Built (2026-11-06): the component side of section 5 — `OnContextMenu (X, Y,
Key)` on every component that can take the keyboard (the mouse, Shift+F10,
the Menu key, a screen reader's show-menu action, a row's More button),
`RListBox.ContextIndex`, `RowActions` / `OnRowAction`, `RMenuItem.Default`,
`RPopupMenu.SystemMenu`, `RDockManager.ContextDocument`, a `PopupMenu`
opening from the keyboard in RapidR programs (classic ones keep the right
click alone until RC.EXE is checked); the menus' 24-pixel rows in RapidR's looks (D1),
the keys in the secondary colour, a keyboard-opened menu's first item
chosen. In Studio (`ide/contextmenu.inc`, one pop-up filled from the command
table): the Project tree (file, folder, group, project, a form's component),
the document tabs, the code editor, the designer (one component, several,
the form), Properties and Events, the Assistant's chats with their row
actions. Manual page [right-click-menus.md](manual/right-click-menus.md) (EN +
ES), example `examples/gui/context_menus.rr`, Studio flows `ctx-*`.

## 10. Phase 2b: what is built, what still waits

Built (2026-11-06), on 2a's builder (one pop-up, every item a command of
`ide/commands.inc`):

- **Sections 4.6–4.10 and 4.12**: the Toolbox (a component — selected first
  — and a group), the Problems page, the Output and Build pages (the place a
  line names first), Immediate, Variables, Watch, Call Stack, Breakpoints,
  Help (a page and a code example; Back and Forward are new: Help keeps the
  pages shown), the status bar (its message, the project, Ln Col, AI apps,
  School Lock's chip), a pane's tab or title, a recent project on the
  Welcome page. The old per-pane menus (Output's, Variables', Watch's, Call
  Stack's, Breakpoints') are gone; the existing flows moved to the one menu.
- **Section 9's deferred items:** Compare With ▸ (the disk's version for an
  open file, the last version with versions, another file — Versions'
  compare pane); Delete File... (asked first with the design guide's small
  question, `MOVETOTRASH`, never the main file nor a file outside the
  project's folder, hidden in the browser); the problem's own group in the
  code editor (the language service's first fix inline and bold, Quick
  Fix..., Explain This Problem: a Help page of the problem); Help: RButton
  in the designer and the Toolbox; the chats grouped Today / Yesterday /
  Earlier (`RListBox.Heading`), renamed in place (`RListBox.EditItem`; the
  rename strip is gone) and Copy as Markdown; a greyed item's reason as a
  tooltip beside the menu, the chosen greyed item framed rather than filled
  so its words stay readable (the kernel's menus: `menubar.rs`,
  `tooltip.rs`); a pop-up menu named for screen readers
  (`RPopupMenu.AccessibleName`: "Actions for orders.rr").
- **Components:** `RTreeView.ContextIndex`, `RListBox.Heading` /
  `EditItem` / `OnEdited`, `RToolbox.ContextKind`, `RDockManager.ContextPane`
  / `DockBack`, `ROutputConsole.PlaceFile` / `PlaceLine`,
  `RMarkdownView.ContextCode`, `RCodeEditor.ProblemAt` / `FixAt` /
  `ApplyFixAt`, the builtin `MOVETOTRASH` (macOS' Foundation through the
  Objective-C runtime, Windows' shell with FOF_ALLOWUNDO, the
  freedesktop.org Trash written in RapidR; no new crate; a run-time error in
  the browser).

Still waiting, and why:

- **A long press on touch screens:** the hosts don't hand the kernel touch
  input as such — the web host turns a touch into mouse presses without
  saying it was a touch, and the desktop host takes no touch events — so a
  long press can't be told from a held click yet. It needs the hosts' touch
  first.
- **The status bar and a pane's tab from the keyboard:** neither takes the
  keyboard, so their menus open by the mouse (and a screen reader reaches
  the same commands in the main menus and the Command Palette).
- **Move to This Project** (a chat without a project): waits for the chats
  following the project (the RAI chat lane).
- **4.11, RAI's answers** (Copy as Text / as Markdown / with Formatting,
  Quote in a New Question, Retry, Edit Question...): the RAI chat redesign's.
- **Classic programs' PopupMenu from the keyboard** stays the right click
  alone until RC.EXE is checked (a question for the lead).
- Some letters of 2a's own menus are shared in a menu (the code editor's
  &Copy and Toggle &Comment): a small follow-up.
