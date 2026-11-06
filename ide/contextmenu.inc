' contextmenu.inc: RapidR Studio's right-click menus
' (docs/context-menus-design.md). One pop-up menu, filled each time from
' the command table — its words, keys and "enabled when" are the main
' menus' (commands.inc) — for what was right-clicked: by the mouse, by
' Shift+F10 or the Menu key, by a screen reader (the components'
' OnContextMenu). RapidR draws it (SystemMenu off): Studio's look on every
' system, seen by RAI and AI apps (app_layout, app_click's menu).
'
' The order of groups is the same everywhere: what it is for (the default
' in bold), make something, edit, find and show, the mode's own, Ask RAI,
' and last what loses something. An item that can't be used now is greyed
' and its hint says why; RAI's item never sends: it puts the thing in the
' question box.

DECLARE SUB CtxPicked(Sender AS RMenuItem)
DECLARE SUB ProjectContext(X AS INTEGER, Y AS INTEGER, Key AS INTEGER, Sender AS RProjectTree)
DECLARE SUB TabsContext(X AS INTEGER, Y AS INTEGER, Key AS INTEGER, Sender AS RDockManager)
DECLARE SUB InspectorContext(X AS INTEGER, Y AS INTEGER, Key AS INTEGER, Sender AS RPropertyInspector)
DECLARE SUB ChatsContext(X AS INTEGER, Y AS INTEGER, Key AS INTEGER, Sender AS RListBox)
DECLARE SUB CtxCmd(Id AS STRING)
DECLARE SUB CtxCmdIf(Id AS STRING, Enabled AS INTEGER, Why AS STRING)
DECLARE SUB CtxCmdAs(Id AS STRING, Caption AS STRING, Enabled AS INTEGER, Why AS STRING)
DECLARE SUB CtxAskRAI(Caption AS STRING, Quote AS STRING)
' (2b: the rest of section 4)
DECLARE SUB OutputContext(X AS INTEGER, Y AS INTEGER, Key AS INTEGER, Sender AS ROutputConsole)
DECLARE SUB ImmediateContext(X AS INTEGER, Y AS INTEGER, Key AS INTEGER, Sender AS RRichEdit)
DECLARE SUB VarsContext(X AS INTEGER, Y AS INTEGER, Key AS INTEGER, Sender AS RTreeView)
DECLARE SUB WatchContext(X AS INTEGER, Y AS INTEGER, Key AS INTEGER, Sender AS RTreeView)
DECLARE SUB StackContext(X AS INTEGER, Y AS INTEGER, Key AS INTEGER, Sender AS RTreeView)
DECLARE SUB BpContext(X AS INTEGER, Y AS INTEGER, Key AS INTEGER, Sender AS RTreeView)
DECLARE SUB HelpContext(X AS INTEGER, Y AS INTEGER, Key AS INTEGER, Sender AS RMarkdownView)
DECLARE SUB StatusContext(X AS INTEGER, Y AS INTEGER, Key AS INTEGER, Sender AS RStatusBar)
DECLARE SUB LockChipContext(X AS INTEGER, Y AS INTEGER, Key AS INTEGER)
DECLARE SUB ToolboxContext(X AS INTEGER, Y AS INTEGER, Key AS INTEGER, Sender AS RToolbox)
DECLARE SUB CtxCheck(IsOn AS INTEGER)
DECLARE SUB PaneContext(X AS INTEGER, Y AS INTEGER, Key AS INTEGER, Sender AS RDockManager)
DECLARE SUB ExplainProblem
DECLARE SUB DeleteFileToTrash(Path AS STRING)
DECLARE SUB HelpTryIt(Code AS STRING)
DECLARE SUB CompareWithOther(Path AS STRING)
DECLARE FUNCTION CtxSurface AS STRING
DECLARE FUNCTION CtxInFolder(Path AS STRING) AS INTEGER

CONST CTXMAX = 30
CONST CTXSUBMAX = 16
DIM CtxMenu AS RPopupMenu
DIM CtxItem(CTXMAX) AS RMenuItem
DIM CtxSub(CTXSUBMAX) AS RMenuItem
' (each item's command, by its Tag: the top items, then the submenus')
DIM CtxCmdOf(CTXMAX + CTXSUBMAX) AS STRING
DIM CtxCount AS INTEGER
DIM CtxSubCount AS INTEGER
DIM CtxSubStart AS INTEGER
DIM CtxSepWanted AS INTEGER
' (the submenu being filled: its holder's index; -1 none)
DIM CtxHolder AS INTEGER
' (the last item added: its Tag)
DIM CtxLast AS INTEGER
' What the menu is for, read by the commands while CtxUse is on (a pick
' from the menu); off, they act on what is active (the palette, a key).
DIM CtxUse AS INTEGER
DIM CtxPath AS STRING
DIM CtxTreePath AS STRING
DIM CtxDoc AS INTEGER
DIM CtxName AS STRING
DIM CtxAsk AS STRING
' (2b) The surface the menu is for — "output", "problems", "immediate",
' "vars", "watch", "stack", "bp", "help", "helpcode", "status", "pane",
' "recent", "toolbox", "code" (a problem's group) — and what on it: a row
' or line (-1 none), a place's line number, a pane's name, a text (a code
' block, a problem's message).
DIM CtxWhere AS STRING
DIM CtxRow AS INTEGER
DIM CtxPlaceLine AS INTEGER
DIM CtxPane AS STRING
DIM CtxText AS STRING

' (the commands only these menus show: in the palette too, under Context)
SUB DefineContextCommands
  AddCmd "project.openFile", "&Open", "Context", "", "Enter", "project", "-"
  AddCmd "project.rename", "Re&name", "Context", "", "F2", "project", "rename"
  AddCmd "project.removeFile", "Remove from Pro&ject", "Context", "", "Del", "project", "-"
  AddCmd "project.showFile", "Show in &Project", "Context", "", "", "document", "-"
  AddCmd "project.collapseAll", "Co&llapse All", "Context", "", "", "project", "-"
  AddCmd "file.copyPath", "Copy &Path", "Context", "", "", "document", "-"
  AddCmd "file.copyRelativePath", "Copy Relati&ve Path", "Context", "", "", "document", "-"
  AddCmd "file.reveal", "Reveal in File &Manager", "Context", "", "", "document", "-"
  AddCmd "file.closeOthers", "Close &Others", "Context", "", "", "document", "-"
  AddCmd "file.closeToRight", "Close to the &Right", "Context", "", "", "document", "-"
  AddCmd "file.closeSaved", "Close &Saved", "Context", "", "", "document", "-"
  AddCmd "designer.editHandler", "&Edit Event Handler", "Context", "", "", "design", "-"
  AddCmd "inspector.editValue", "&Edit Value", "Context", "", "Enter", "", "-"
  AddCmd "inspector.reset", "&Reset to Default", "Context", "", "Del", "", "-"
  AddCmd "inspector.copyValue", "&Copy Value", "Context", "", "Ctrl+C", "", "-"
  AddCmd "inspector.copyName", "Copy &Name", "Context", "", "", "", "-"
  AddCmd "inspector.copyCode", "Copy as Co&de", "Context", "", "", "", "-"
  AddCmd "inspector.goToHandler", "&Go to Handler", "Context", "", "Enter", "", "-"
  AddCmd "inspector.removeHandler", "&Remove Handler", "Context", "", "Del", "", "-"
  AddCmd "inspector.help", "&Help", "Context", "", "F1", "", "-"
  AddCmd "ai.askAbout", "As&k RAI...", "Context", "", "", "ai", "ai.ask"
  AddCmd "ai.chatOpen", "&Open", "Context", "", "Enter", "ai", "-"
  AddCmd "ai.chatRename", "Re&name", "Context", "", "F2", "ai", "-"
  AddCmd "ai.chatDelete", "&Delete", "Context", "", "Del", "ai", "-"
  ' (2b)
  AddCmd "ai.chatCopyMarkdown", "Copy as &Markdown", "Context", "", "", "ai", "-"
  AddCmd "file.compareLast", "Compare with the &Last Version", "Context", "", "", "versioned", "compare"
  AddCmd "file.compareOther", "Compare with &Another File...", "Context", "", "", "", "compare"
  AddCmd "file.deleteFile", "&Delete File...", "Context", "", "", "project", "-"
  AddCmd "edit.applyFix", "Apply the &Fix", "Context", "", "", "code", "-"
  AddCmd "edit.explainProblem", "E&xplain This Problem", "Context", "", "", "", "-"
  AddCmd "problems.goTo", "&Go to Problem", "Context", "", "Enter", "", "-"
  AddCmd "output.goTo", "&Go to the Place", "Context", "", "", "", "-"
  AddCmd "pane.copyLine", "Copy &Line", "Context", "", "", "", "-"
  AddCmd "pane.copyAll", "Copy &All", "Context", "", "", "", "-"
  AddCmd "pane.clear", "Cl&ear", "Context", "", "", "", "-"
  AddCmd "debug.setValue", "&Set Value...", "Context", "", "F2", "paused", "-"
  AddCmd "debug.copyValue", "&Copy Value", "Context", "", "Ctrl+C", "", "-"
  AddCmd "debug.copyExpression", "Copy &Expression", "Context", "", "", "", "-"
  AddCmd "debug.viewText", "&View Text...", "Context", "", "", "paused", "-"
  AddCmd "debug.removeWatch", "&Remove Watch", "Context", "", "Del", "", "-"
  AddCmd "debug.goToCall", "&Go to This Call", "Context", "", "Enter", "paused", "-"
  AddCmd "debug.goToBreakpoint", "&Go to Breakpoint", "Context", "", "Enter", "", "-"
  AddCmd "debug.editCondition", "Edit Co&ndition...", "Context", "", "Shift+F9", "", "-"
  AddCmd "debug.disableBreakpoint", "Disa&ble Breakpoint", "Context", "", "Space", "", "-"
  AddCmd "debug.removeBreakpoint", "&Remove Breakpoint", "Context", "", "Del", "", "-"
  AddCmd "help.component", "&Help on This Component", "Context", "", "F1", "", "-"
  AddCmd "help.example", "Open an &Example", "Context", "", "", "", "-"
  AddCmd "help.openBrowser", "Open in the Bro&wser", "Context", "", "", "", "-"
  AddCmd "help.tryIt", "&Try It", "Context", "", "", "", "-"
  AddCmd "help.copyCode", "Copy &Code", "Context", "", "", "", "-"
  AddCmd "help.insertCode", "&Insert at the Cursor", "Context", "", "", "code", "-"
  AddCmd "status.copyMessage", "&Copy Message", "Context", "", "", "", "-"
  AddCmd "view.closePane", "&Close Pane", "Context", "", "", "", "-"
  AddCmd "view.floatPane", "&Float", "Context", "", "", "", "-"
  AddCmd "recent.open", "&Open", "Context", "", "Enter", "", "-"
  AddCmd "recent.remove", "&Remove from Recent", "Context", "", "", "", "-"
  AddCmd "toolbox.add", "&Add to the Form", "Context", "", "Enter", "", "-"
  AddCmd "toolbox.expandAll", "&Expand All", "Context", "", "", "", "-"
END SUB

SUB SetUpContextMenus
  DIM k AS INTEGER
  CtxMenu.SystemMenu = 0
  FOR k = 0 TO CTXMAX - 1
    CtxItem(k).Tag = k
    CtxItem(k).OnClick = CtxPicked
  NEXT
  FOR k = 0 TO CTXSUBMAX - 1
    CtxSub(k).Tag = CTXMAX + k
    CtxSub(k).OnClick = CtxPicked
  NEXT
  CtxDoc = -1
  ProjectTree.OnContextMenu = ProjectContext
  Dock.OnContextMenu = TabsContext
  Inspector.OnContextMenu = InspectorContext
  AIChatList.OnContextMenu = ChatsContext
  ' (2b: the rest of section 4)
  OutputBox.OnContextMenu = OutputContext
  ImmediateBox.OnContextMenu = ImmediateContext
  VarsTree.OnContextMenu = VarsContext
  WatchTree.OnContextMenu = WatchContext
  StackTree.OnContextMenu = StackContext
  BpTree.OnContextMenu = BpContext
  HelpPage.OnContextMenu = HelpContext
  Status.OnContextMenu = StatusContext
  LockChipBox.OnContextMenu = LockChipContext
  LockChip.OnContextMenu = LockChipContext
  LockChipIcon.OnContextMenu = LockChipContext
  LockChipText.OnContextMenu = LockChipContext
  Toolbox.OnContextMenu = ToolboxContext
  CtxRow = -1
END SUB

' ---- building a menu ------------------------------------------------------------------

SUB CtxBegin
  DIM k AS INTEGER
  CtxMenu.Clear
  FOR k = 0 TO CTXMAX - 1
    CtxItem(k).Clear
    CtxCmdOf(k) = ""
  NEXT
  FOR k = 0 TO CTXSUBMAX - 1
    CtxCmdOf(CTXMAX + k) = ""
  NEXT
  CtxCount = 0
  CtxSubCount = 0
  CtxSepWanted = 0
  CtxHolder = -1
  CtxLast = -1
  CtxUse = 0
  CtxDoc = -1
  CtxWhere = ""
  CtxRow = -1
  CtxPlaceLine = 0
  CtxPane = ""
  CtxText = ""
  CtxPath = ""
  CtxTreePath = ""
END SUB

' (the last item added: a check mark, on or off)
SUB CtxCheck(IsOn AS INTEGER)
  IF CtxLast < 0 THEN EXIT SUB
  IF CtxLast < CTXMAX THEN CtxItem(CtxLast).Checked = Flag(IsOn) ELSE CtxSub(CtxLast - CTXMAX).Checked = Flag(IsOn)
END SUB

' (a separator before the next item, if any comes, never two, never first)
SUB CtxSep
  CtxSepWanted = 1
END SUB

' (an item's state, by its Tag)
SUB CtxSet(T AS INTEGER, Caption AS STRING, Enabled AS INTEGER, Why AS STRING, Bold AS INTEGER)
  IF T < CTXMAX THEN
    CtxItem(T).Caption = Caption
    CtxItem(T).Enabled = Flag(Enabled)
    CtxItem(T).Hint = Why
    CtxItem(T).Default = Flag(Bold)
    CtxItem(T).Checked = 0
    CtxItem(T).Visible = 1
  ELSE
    CtxSub(T - CTXMAX).Caption = Caption
    CtxSub(T - CTXMAX).Enabled = Flag(Enabled)
    CtxSub(T - CTXMAX).Hint = Why
    CtxSub(T - CTXMAX).Default = Flag(Bold)
    CtxSub(T - CTXMAX).Checked = 0
    CtxSub(T - CTXMAX).Visible = 1
  END IF
END SUB

' (the next item of the menu, or of the submenu being filled: its Tag;
' -1 when the pool is used up)
FUNCTION CtxTake AS INTEGER
  DIM k AS INTEGER
  Result = -1
  IF CtxHolder >= 0 THEN
    IF CtxSubCount >= CTXSUBMAX THEN EXIT FUNCTION
    k = CtxSubCount
    CtxSubCount = CtxSubCount + 1
    CtxItem(CtxHolder).AddItems CtxSub(k)
    Result = CTXMAX + k
  ELSE
    IF CtxCount >= CTXMAX THEN EXIT FUNCTION
    k = CtxCount
    CtxCount = CtxCount + 1
    CtxMenu.AddItems CtxItem(k)
    Result = k
  END IF
END FUNCTION

' An item: its words (with & for its letter), its keys, whether it can be
' used now and, if not, why; Id is the command it runs.
SUB CtxAdd(Id AS STRING, Caption AS STRING, Keys AS STRING, Enabled AS INTEGER, Why AS STRING)
  DIM t AS INTEGER, c AS STRING, first AS INTEGER
  IF CtxHolder >= 0 THEN first = Flag(CtxSubCount = CtxSubStart) ELSE first = Flag(CtxCount = 0)
  IF CtxSepWanted AND first = 0 THEN
    t = CtxTake
    IF t >= 0 THEN CtxSet t, "-", 1, "", 0
  END IF
  CtxSepWanted = 0
  t = CtxTake
  IF t < 0 THEN EXIT SUB
  c = Caption
  IF Keys <> "" THEN c = c + CHR$(9) + Keys
  CtxSet t, c, Enabled, Why, 0
  CtxCmdOf(t) = Id
  CtxLast = t
END SUB

' (the last item added: the default action, in bold)
SUB CtxDefault
  IF CtxLast < 0 THEN EXIT SUB
  IF CtxLast < CTXMAX THEN CtxItem(CtxLast).Default = 1 ELSE CtxSub(CtxLast - CTXMAX).Default = 1
END SUB

' A command of the table, as the main menus show it; Enabled -1: its own
' "enabled when".
SUB CtxCmdIf(Id AS STRING, Enabled AS INTEGER, Why AS STRING)
  DIM i AS INTEGER, en AS INTEGER
  i = CmdIndex(Id)
  IF i < 0 THEN EXIT SUB
  IF Enabled = -1 THEN en = Flag(CmdEnabled(i)) ELSE en = Flag(Enabled)
  CtxAdd Id, TR$(Cmd(i).Title), CmdKeys(i), en, Why
END SUB

SUB CtxCmd(Id AS STRING)
  CtxCmdIf Id, -1, ""
END SUB

' (a command in words of its own here: "Ask RAI About This File...")
SUB CtxCmdAs(Id AS STRING, Caption AS STRING, Enabled AS INTEGER, Why AS STRING)
  DIM i AS INTEGER, en AS INTEGER
  i = CmdIndex(Id)
  IF i < 0 THEN EXIT SUB
  IF Enabled = -1 THEN en = Flag(CmdEnabled(i)) ELSE en = Flag(Enabled)
  CtxAdd Id, Caption, CmdKeys(i), en, Why
END SUB

' A submenu (one level): its items follow, until CtxSubEnd.
SUB CtxSubBegin(Caption AS STRING)
  DIM t AS INTEGER
  CtxAdd "", Caption, "", 1, ""
  t = CtxLast
  IF t < 0 OR t >= CTXMAX THEN EXIT SUB
  CtxHolder = t
  CtxSubStart = CtxSubCount
END SUB

SUB CtxSubEnd
  CtxHolder = -1
  CtxSepWanted = 0
END SUB

' RAI's item: hidden while School Lock keeps AI off; greyed, saying how to
' set it up, while no model is chosen. Quote is what goes in the question box.
SUB CtxAskRAI(Caption AS STRING, Quote AS STRING)
  IF LockOffAI THEN EXIT SUB
  CtxSep
  CtxAsk = Quote
  CtxCmdAs "ai.askAbout", Caption, Flag(AIReady <> 0), TR$("Set up RAI first: Tools > Set Up RAI")
  CtxSep
END SUB

' (a row's More button, Key 4: the menu right-aligned at X)
SUB CtxAlign(Key AS INTEGER)
  IF Key = 4 THEN CtxMenu.Alignment = 1 ELSE CtxMenu.Alignment = 0
END SUB

SUB CtxPicked(Sender AS RMenuItem)
  DIM id AS STRING
  id = CtxCmdOf(Sender.Tag)
  IF id = "" THEN EXIT SUB
  ' (a document's tab: the commands of the active document act on it)
  IF CtxDoc >= 0 AND CtxDoc < DocCount THEN
    IF DocOpen(CtxDoc) THEN
      SELECT CASE id
        CASE "file.close", "view.splitVertically", "view.splitHorizontally", "versions.localHistory"
          Dock.ActiveDocument = DocPane(CtxDoc)
      END SELECT
    END IF
  ELSEIF CtxPath <> "" AND (id = "versions.localHistory" OR id = "view.code") THEN
    ' (a file of the Project tree: its document opened — in front — first)
    IF FILEEXISTS(CtxPath) THEN OpenDocument CtxPath
  END IF
  CtxUse = 1
  ' (a surface's own Copy, Select All, Clear …: the menu's surface, not the
  ' pane that has the keyboard — so before RunCommand's own order)
  IF CtxWhere <> "" THEN
    Proj.CrashDoing id
    IF ContextCommand(id) = 0 THEN RunCommand id
  ELSE
    RunCommand id
  END IF
  CtxUse = 0
END SUB

' ---- what a path is ------------------------------------------------------------------

' (a file's path from the project's folder: as the tree writes it)
FUNCTION CtxRelative(Path AS STRING) AS STRING
  DIM f AS STRING, p AS STRING
  p = REPLACESUBSTR$(Path, "\", "/")
  f = REPLACESUBSTR$(Proj.Folder, "\", "/")
  IF f <> "" AND LCASE$(LEFT$(p, LEN(f) + 1)) = LCASE$(f + "/") THEN Result = MID$(p, LEN(f) + 2) ELSE Result = p
END FUNCTION

' (a path inside the project's folder: Delete File... only acts there)
FUNCTION CtxInFolder(Path AS STRING) AS INTEGER
  DIM f AS STRING, p AS STRING
  p = LCASE$(REPLACESUBSTR$(Path, "\", "/"))
  f = LCASE$(REPLACESUBSTR$(Proj.Folder, "\", "/"))
  Result = Flag(f <> "" AND LEFT$(p, LEN(f) + 1) = f + "/")
END FUNCTION

' (the file the commands act on: the menu's, else the active document's)
FUNCTION CtxFile AS STRING
  IF CtxUse AND CtxPath <> "" THEN
    Result = CtxPath
  ELSEIF ActiveDoc >= 0 THEN
    Result = DocFile(ActiveDoc)
  ELSE
    Result = ""
  END IF
END FUNCTION

' (Reveal in Finder / File Explorer / Files; "" where there's none: the web)
FUNCTION RevealWords AS STRING
  IF Proj.FileManager = "" THEN Result = "" ELSE Result = TR$("Reveal in {manager}", "manager", Proj.FileManager)
END FUNCTION

' The three path items (Copy Path, Copy Relative Path, Reveal).
SUB CtxPathItems(Path AS STRING)
  CtxCmdIf "file.copyPath", Flag(Path <> ""), ""
  CtxCmdIf "file.copyRelativePath", Flag(Path <> "" AND ProjectOpen <> 0), ""
  IF RevealWords <> "" THEN CtxCmdAs "file.reveal", RevealWords, Flag(Path <> ""), ""
END SUB

' ---- the Project tree ------------------------------------------------------------------

SUB ProjectContext(X AS INTEGER, Y AS INTEGER, Key AS INTEGER, Sender AS RProjectTree)
  DIM kind AS STRING, sel AS STRING, p AS STRING, fk AS STRING, isMain AS INTEGER, versioned AS INTEGER
  IF ProjectOpen = 0 THEN EXIT SUB
  kind = Sender.SelectedKind
  sel = Sender.Selected
  CtxBegin
  CtxTreePath = sel
  CtxPath = ""
  versioned = Flag(Repo.Versioned <> 0)
  SELECT CASE kind
    CASE "file"
      p = ProjectPath(sel)
      CtxPath = p
      fk = Sender.FileKind(sel)
      isMain = Flag(LCASE$(p) = LCASE$(ProjectMain))
      CtxCmd "project.openFile"
      CtxDefault
      IF fk = "form" THEN CtxCmdAs "view.code", TR$("Open &Code"), 1, ""
      CtxSep
      CtxCmd "project.addModule"
      CtxCmd "project.addFile"
      CtxSep
      CtxCmd "project.rename"
      CtxPathItems p
      ' (2b: Compare With — the disk's version, the last version, another file)
      CtxSep
      CtxSubBegin TR$("Compare &With")
      CtxCmdAs "file.compareDisk", TR$("the &Disk's Version"), Flag(DocOf(p) >= 0), TR$("Open it first: the disk's version is compared with yours")
      IF versioned THEN CtxCmdAs "file.compareLast", TR$("the &Last Version"), 1, ""
      CtxCmdAs "file.compareOther", TR$("&Another File..."), 1, ""
      CtxSubEnd
      IF versioned THEN CtxCmdAs "versions.localHistory", TR$("Show &History"), 1, ""
      CtxAskRAI TR$("Ask RAI About This File..."), TR$("About {file}:", "file", sel) + " "
      CtxSep
      CtxCmdIf "project.removeFile", Flag(isMain = 0), TR$("The main file stays in the project")
      ' (2b: to the Trash, asked first — never the main file, never a file
      ' outside the project's folder: hidden then; not in the browser)
      IF Proj.FileManager <> "" AND CtxInFolder(p) THEN CtxCmdIf "file.deleteFile", Flag(isMain = 0), TR$("The main file stays in the project")
    CASE "folder"
      p = ProjectPath(sel)
      CtxPath = p
      CtxCmd "project.addModule"
      CtxCmd "project.addFile"
      CtxCmd "project.addFolder"
      CtxSep
      CtxPathItems p
      CtxSep
      CtxCmd "project.collapseAll"
    CASE "component"
      CtxCmdAs "project.openFile", TR$("&Select in the Designer"), 1, ""
      CtxDefault
      CtxSep
      CtxName = MID$(sel, INSTR(sel, "#") + 1)
      CtxCmd "inspector.copyName"
      ' (its type is known once the designer shows it: Help opens it there)
      CtxCmd "help.component"
      CtxAskRAI TR$("Ask RAI About This Component..."), TR$("About {name}:", "name", CtxName) + " "
    CASE "group"
      CtxCmd "project.addForm"
      CtxCmd "project.addModule"
      CtxCmd "project.addFile"
      CtxCmd "project.addFolder"
      CtxSep
      CtxCmd "project.collapseAll"
    CASE ELSE
      ' (the project's own row, the empty part)
      CtxPath = Proj.Folder
      CtxCmd "project.addForm"
      CtxCmd "project.addModule"
      CtxCmd "project.addFile"
      CtxCmd "project.addFolder"
      CtxSep
      CtxCmdIf "file.copyPath", 1, ""
      IF RevealWords <> "" THEN CtxCmdAs "file.reveal", RevealWords, 1, ""
      CtxSep
      IF versioned THEN CtxCmd "versions.save" ELSE CtxCmd "versions.start"
      CtxCmd "project.collapseAll"
      CtxAskRAI TR$("Ask RAI About This Project..."), TR$("About this project:") + " "
      CtxCmd "project.options"
      CtxSep
      CtxCmd "file.closeProject"
  END SELECT
  CtxMenu.AccessibleName = TR$("Actions for {what}", "what", IIF(sel = "", ProjectName, FileTitle(sel)))
  CtxAlign Key
  CtxMenu.PopupAt Sender, X, Y
END SUB

' ---- document tabs ---------------------------------------------------------------------

SUB TabsContext(X AS INTEGER, Y AS INTEGER, Key AS INTEGER, Sender AS RDockManager)
  DIM doc AS STRING, d AS INTEGER, others AS INTEGER, k AS INTEGER
  doc = Sender.ContextDocument
  IF doc = "" THEN
    PaneContext X, Y, Key, Sender
    EXIT SUB
  END IF
  d = DocOfPane(doc)
  IF d < 0 THEN EXIT SUB
  CtxBegin
  CtxDoc = d
  CtxPath = DocFile(d)
  FOR k = 0 TO DocCount - 1
    IF DocOpen(k) AND k <> d THEN others = others + 1
  NEXT
  CtxCmd "file.close"
  CtxCmdIf "file.closeOthers", Flag(others > 0), ""
  CtxCmdIf "file.closeToRight", Flag(others > 0), ""
  CtxCmd "file.closeSaved"
  CtxCmd "file.closeAll"
  CtxSep
  CtxPathItems CtxPath
  CtxCmdIf "project.showFile", Flag(ProjectOpen <> 0), ""
  CtxSep
  CtxCmd "view.splitVertically"
  CtxCmd "view.splitHorizontally"
  IF DocKind(d) = "code" OR DocKind(d) = "design" THEN
    CtxSep
    CtxCmd "versions.localHistory"
  END IF
  CtxMenu.AccessibleName = TR$("Actions for {what}", "what", FileTitle(CtxPath))
  CtxAlign Key
  CtxMenu.PopupAt Sender, X, Y
END SUB

' ---- the code editor ---------------------------------------------------------------------

SUB CodeContext(X AS INTEGER, Y AS INTEGER, Key AS INTEGER, Sender AS RCodeEditor)
  DIM d AS INTEGER, wd AS STRING, sel AS INTEGER, ro AS INTEGER, q AS STRING, fix AS STRING
  d = Sender.Tag
  IF d < 0 OR d >= DocCount THEN EXIT SUB
  IF DocOpen(d) = 0 THEN EXIT SUB
  CtxBegin
  CtxPath = DocFile(d)
  wd = CaretWord(d)
  sel = Flag(Sender.SelLength > 0)
  ro = Flag(Sender.ReadOnly <> 0)
  ' (2b: on a problem — its fix itself when the language service has one,
  ' Quick Fix..., Explain This Problem — then the usual groups)
  CtxText = Sender.ProblemAt(Sender.CaretLine, Sender.CaretColumn)
  IF CtxText <> "" THEN
    CtxWhere = "code"
    fix = Sender.FixAt(Sender.CaretLine, Sender.CaretColumn)
    IF fix <> "" AND ro = 0 THEN
      CtxCmdAs "edit.applyFix", fix, 1, ""
      CtxDefault
    END IF
    CtxCmd "edit.quickFix"
    CtxCmd "edit.explainProblem"
    CtxSep
  END IF
  CtxCmdIf "edit.goToDefinition", Flag(wd <> ""), TR$("Not on a name")
  CtxCmdIf "edit.findReferences", Flag(wd <> ""), TR$("Not on a name")
  CtxCmdIf "edit.rename", Flag(wd <> "" AND ro = 0), TR$("Not on a name")
  IF CtxText = "" THEN CtxCmd "edit.quickFix"
  CtxSep
  CtxCmdIf "edit.cut", Flag(sel AND ro = 0), TR$("Nothing is selected")
  CtxCmdIf "edit.copy", sel, TR$("Nothing is selected")
  CtxCmdIf "edit.paste", Flag(ro = 0), ""
  CtxSep
  CtxCmd "debug.toggleBreakpoint"
  CtxCmd "debug.runToCursor"
  IF RunState = "paused" THEN
    CtxCmd "debug.setNextStatement"
    CtxCmd "debug.addWatch"
  END IF
  CtxSep
  CtxCmdIf "edit.toggleComment", Flag(ro = 0), ""
  IF Repo.Versioned <> 0 THEN
    CtxSubBegin TR$("&Versions")
    CtxCmd "versions.showChange"
    CtxCmd "versions.undoChange"
    CtxSep
    CtxCmd "versions.localHistory"
    CtxSubEnd
  END IF
  IF sel THEN
    q = "```basic" + CHR$(10) + Sender.SelText + CHR$(10) + "```" + CHR$(10)
  ELSE
    q = TR$("About line {n} of {file}:", "n", LTRIM$(STR$(Sender.CaretLine)), "file", FileTitle(DocFile(d))) + " "
  END IF
  IF CtxText <> "" THEN
    CtxAskRAI TR$("Ask RAI About This Problem..."), TR$("About this problem on line {n} of {file}:", "n", LTRIM$(STR$(Sender.CaretLine)), "file", FileTitle(DocFile(d))) + " " + CtxText + CHR$(10)
  ELSE
    CtxAskRAI TR$("Ask RAI About This Code..."), q
  END IF
  CtxMenu.AccessibleName = TR$("Actions for the code")
  CtxAlign Key
  CtxMenu.PopupAt Sender, X, Y
END SUB

' ---- the form designer --------------------------------------------------------------------

SUB DesignContext(X AS INTEGER, Y AS INTEGER, Key AS INTEGER, Sender AS RDesignSurface)
  DIM d AS INTEGER, n AS INTEGER, nm AS STRING
  d = Sender.Tag
  IF d < 0 OR d >= DocCount THEN EXIT SUB
  IF DocOpen(d) = 0 THEN EXIT SUB
  ' (the commands act on the designer in front: this one)
  IF ActiveDesign <> d THEN Dock.ActiveDocument = DocPane(d)
  CtxBegin
  CtxPath = DocFile(d)
  n = Sender.SelCount
  IF n >= 2 THEN
    CtxSubBegin TR$("&Align")
    CtxCmd "designer.alignLeft"
    CtxCmd "designer.alignRight"
    CtxCmd "designer.alignTop"
    CtxCmd "designer.alignBottom"
    CtxSep
    CtxCmd "designer.alignCenters"
    CtxCmd "designer.alignMiddles"
    CtxSubEnd
    CtxSubBegin TR$("Make Same Si&ze")
    CtxCmd "designer.sameWidth"
    CtxCmd "designer.sameHeight"
    CtxCmd "designer.sameSize"
    CtxSubEnd
    CtxSubBegin TR$("Space &Evenly")
    CtxCmdIf "designer.spaceHorizontally", Flag(n >= 3), TR$("Needs three or more")
    CtxCmdIf "designer.spaceVertically", Flag(n >= 3), TR$("Needs three or more")
    CtxSubEnd
    CtxSep
    CtxCmd "edit.cut"
    CtxCmd "edit.copy"
    CtxCmd "edit.paste"
    CtxCmd "edit.duplicate"
    CtxSep
    CtxCmd "designer.bringToFront"
    CtxCmd "designer.sendToBack"
    CtxSep
    CtxCmd "view.properties"
    CtxAskRAI TR$("Ask RAI About These Components..."), TR$("About the components {names}:", "names", Inspector.Target) + " "
    CtxSep
    CtxCmd "edit.delete"
  ELSEIF n = 1 AND Sender.SelIndex >= 0 THEN
    nm = Sender.GetName(Sender.SelIndex)
    CtxName = nm
    CtxCmd "designer.editHandler"
    CtxDefault
    CtxCmd "designer.editCaption"
    CtxSep
    CtxCmd "edit.cut"
    CtxCmd "edit.copy"
    CtxCmd "edit.paste"
    CtxCmd "edit.duplicate"
    CtxSep
    CtxCmd "designer.bringToFront"
    CtxCmd "designer.sendToBack"
    CtxSubBegin TR$("Center in &Form")
    CtxCmd "designer.centerHorizontally"
    CtxCmd "designer.centerVertically"
    CtxSubEnd
    CtxSep
    CtxCmd "designer.tabOrder"
    CtxCmd "view.properties"
    CtxText = Inspector.TargetType
    IF CtxText <> "" THEN CtxCmdAs "help.component", TR$("&Help: {name}", "name", CtxText), 1, ""
    CtxAskRAI TR$("Ask RAI About This Component..."), TR$("About {name}:", "name", nm) + " "
    CtxSep
    CtxCmd "edit.delete"
  ELSE
    CtxCmd "edit.paste"
    CtxCmd "edit.selectAll"
    CtxSep
    CtxCmd "designer.menuEditor"
    CtxCmd "designer.tabOrder"
    CtxSep
    CtxSubBegin TR$("&Zoom")
    CtxCmd "designer.zoomIn"
    CtxCmd "designer.zoomOut"
    CtxCmd "designer.zoomReset"
    CtxCmd "designer.zoomToFit"
    CtxSubEnd
    CtxSep
    CtxCmd "view.code"
    CtxCmd "view.properties"
    CtxText = Inspector.TargetType
    IF CtxText <> "" THEN CtxCmdAs "help.component", TR$("&Help: {name}", "name", CtxText), 1, ""
    CtxAskRAI TR$("Ask RAI About This Form..."), TR$("About the form in {file}:", "file", FileTitle(DocFile(d))) + " "
  END IF
  CtxMenu.AccessibleName = TR$("Actions for the form")
  CtxAlign Key
  CtxMenu.PopupAt Sender, X, Y
END SUB

' ---- Properties and Events ----------------------------------------------------------------

SUB InspectorContext(X AS INTEGER, Y AS INTEGER, Key AS INTEGER, Sender AS RPropertyInspector)
  DIM prop AS STRING, v AS STRING
  prop = Sender.Selected
  IF prop = "" OR Sender.Target = "" THEN EXIT SUB
  CtxBegin
  CtxName = prop
  v = Sender.Value(prop)
  IF Sender.Page = "events" THEN
    IF v <> "" THEN
      CtxCmd "inspector.goToHandler"
      CtxDefault
      CtxCmdIf "inspector.removeHandler", Flag(Sender.ReadOnly = 0), ""
    ELSE
      CtxCmdAs "inspector.goToHandler", TR$("&Create Handler"), 1, ""
      CtxDefault
    END IF
    CtxSep
    CtxCmd "inspector.copyName"
    CtxCmdIf "inspector.copyCode", Flag(v <> ""), ""
  ELSE
    CtxCmd "inspector.editValue"
    CtxDefault
    CtxCmdIf "inspector.reset", Flag(Sender.IsDefault(prop) = 0 AND Sender.ReadOnly = 0), TR$("Already the default")
    CtxSep
    CtxCmd "inspector.copyValue"
    CtxCmd "inspector.copyName"
    CtxCmd "inspector.copyCode"
  END IF
  CtxSep
  CtxCmdAs "inspector.help", TR$("&Help: {name}", "name", prop), 1, ""
  CtxAskRAI TR$("Ask RAI About {name}...", "name", prop), TR$("About {name} of {target}:", "name", prop, "target", Sender.Target) + " "
  CtxMenu.AccessibleName = TR$("Actions for {what}", "what", prop)
  CtxAlign Key
  CtxMenu.PopupAt Sender, X, Y
END SUB

' ---- RAI's chats ---------------------------------------------------------------------------

SUB ChatsContext(X AS INTEGER, Y AS INTEGER, Key AS INTEGER, Sender AS RListBox)
  IF Sender.ContextIndex < 0 THEN EXIT SUB
  Sender.ItemIndex = Sender.ContextIndex
  CtxBegin
  CtxCmd "ai.chatOpen"
  CtxDefault
  CtxSep
  CtxCmd "ai.chatRename"
  CtxCmd "ai.chatCopyMarkdown"
  CtxSep
  CtxCmd "ai.chatDelete"
  CtxMenu.AccessibleName = TR$("Actions for the chat")
  CtxAlign Key
  CtxMenu.PopupAt Sender, X, Y
END SUB

' ---- Problems, Output, Immediate (2b) ----------------------------------------------------

' The Output pane's pages: the Problems page (a problem's own group first)
' and the output and build pages (the place a line names first).
SUB OutputContext(X AS INTEGER, Y AS INTEGER, Key AS INTEGER, Sender AS ROutputConsole)
  DIM ln AS INTEGER, has AS INTEGER, f AS STRING, n AS INTEGER, on AS INTEGER
  ln = Sender.ClickedLine
  has = Flag(Sender.LineCount > 0)
  CtxBegin
  CtxRow = ln
  IF ln >= 0 THEN
    f = Sender.PlaceFile(ln)
    n = Sender.PlaceLine(ln)
  END IF
  CtxPath = f
  CtxPlaceLine = n
  IF Sender.Page = "problems" THEN
    CtxWhere = "problems"
    on = Flag(f <> "" AND n > 0)
    IF ln >= 0 THEN CtxText = Sender.Line(ln)
    CtxCmdIf "problems.goTo", Flag(f <> ""), TR$("Not on a problem")
    CtxDefault
    CtxCmdIf "edit.quickFix", on, TR$("Not on a problem")
    CtxCmdIf "edit.explainProblem", on, TR$("Not on a problem")
    CtxSep
    CtxCmdIf "edit.copy", Flag(Sender.HasSelection), TR$("Nothing is selected")
    CtxCmdAs "pane.copyAll", TR$("Copy &All Problems"), has, TR$("No problems")
    IF on THEN CtxAskRAI TR$("Ask RAI About This Problem..."), TR$("About this problem:") + " " + CtxText + CHR$(10)
    CtxMenu.AccessibleName = TR$("Actions for the problem")
  ELSE
    CtxWhere = "output"
    IF f <> "" THEN
      CtxCmdAs "output.goTo", TR$("&Go to {file}, Line {n}", "file", FileTitle(f), "n", Num(n)), 1, ""
      CtxDefault
      CtxSep
    END IF
    CtxCmdIf "edit.copy", Flag(Sender.HasSelection), TR$("Nothing is selected")
    CtxCmdIf "pane.copyLine", Flag(ln >= 0), TR$("Not on a line")
    CtxCmdIf "pane.copyAll", has, TR$("Nothing to copy")
    CtxCmdAs "edit.selectAll", TR$("&Select All"), has, TR$("Nothing to copy")
    CtxSep
    CtxCmdAs "view.outputWrap", TR$("&Wrap Long Lines"), 1, ""
    CtxCheck Flag(OutputNoWrap = 0)
    CtxSep
    CtxCmdIf "pane.clear", has, TR$("Nothing to clear")
    CtxMenu.AccessibleName = TR$("Actions for the output")
  END IF
  CtxAlign Key
  CtxMenu.PopupAt Sender, X, Y
END SUB

SUB ImmediateContext(X AS INTEGER, Y AS INTEGER, Key AS INTEGER, Sender AS RRichEdit)
  CtxBegin
  CtxWhere = "immediate"
  CtxCmdIf "edit.copy", Flag(Sender.SelLength > 0), TR$("Nothing is selected")
  CtxCmdIf "edit.paste", 1, ""
  CtxCmdAs "edit.selectAll", TR$("&Select All"), Flag(Sender.Text <> ""), TR$("Nothing to copy")
  CtxSep
  CtxCmdIf "pane.clear", Flag(Sender.Text <> ""), TR$("Nothing to clear")
  CtxMenu.AccessibleName = TR$("Actions for Immediate")
  CtxAlign Key
  CtxMenu.PopupAt Sender, X, Y
END SUB

' ---- the debug panes (2b) ------------------------------------------------------------------

' Variables (w 0) and Watch (1): the item right-clicked selected first.
SUB ValuesContext(W AS INTEGER, X AS INTEGER, Y AS INTEGER, Key AS INTEGER, Sender AS RTreeView)
  DIM e AS STRING, paused AS INTEGER, why AS STRING, i AS INTEGER, top AS INTEGER
  i = Sender.ContextIndex
  IF i < 0 THEN EXIT SUB
  Sender.ItemIndex = i
  CtxBegin
  CtxRow = i
  IF W = 0 THEN CtxWhere = "vars" ELSE CtxWhere = "watch"
  e = PickedExpr(W)
  paused = Flag(RunState = "paused")
  why = TR$("The program isn't paused")
  CtxCmdIf "debug.setValue", Flag(paused AND e <> ""), why
  CtxSep
  CtxCmdIf "debug.copyValue", paused, why
  CtxCmdIf "debug.copyExpression", Flag(e <> ""), TR$("Not a value")
  CtxCmdIf "debug.viewText", Flag(paused AND e <> ""), why
  CtxSep
  IF W = 0 THEN
    CtxCmdIf "debug.addWatch", Flag(e <> ""), TR$("Not a value")
  ELSE
    ' (a watch itself, not a part of one)
    IF i <= MAXVARITEMS THEN top = Flag(LEFT$(WatchItemPath(i), 1) = "W" AND INSTR(WatchItemPath(i), "/") = 0)
    CtxCmdIf "debug.removeWatch", top, TR$("Only a watch itself can be removed")
  END IF
  CtxMenu.AccessibleName = TR$("Actions for {what}", "what", Sender.SelectedItem)
  CtxAlign Key
  CtxMenu.PopupAt Sender, X, Y
END SUB

SUB VarsContext(X AS INTEGER, Y AS INTEGER, Key AS INTEGER, Sender AS RTreeView)
  ValuesContext 0, X, Y, Key, Sender
END SUB

SUB WatchContext(X AS INTEGER, Y AS INTEGER, Key AS INTEGER, Sender AS RTreeView)
  ValuesContext 1, X, Y, Key, Sender
END SUB

SUB StackContext(X AS INTEGER, Y AS INTEGER, Key AS INTEGER, Sender AS RTreeView)
  DIM i AS INTEGER
  i = Sender.ContextIndex
  CtxBegin
  CtxWhere = "stack"
  ' (selected, as a click would — not gone to: Go to This Call does that)
  IF i >= 0 THEN
    TreeFilling = 1
    Sender.ItemIndex = i
    TreeFilling = 0
  END IF
  CtxRow = i
  CtxCmdIf "debug.goToCall", Flag(i >= 0 AND i < FrameCount AND RunState = "paused"), TR$("The program isn't paused")
  CtxDefault
  CtxSep
  CtxCmdIf "edit.copy", Flag(i >= 0), TR$("Nothing is selected")
  CtxCmdIf "pane.copyAll", Flag(Sender.ItemCount > 0), TR$("Nothing to copy")
  CtxMenu.AccessibleName = TR$("Actions for the call")
  CtxAlign Key
  CtxMenu.PopupAt Sender, X, Y
END SUB

SUB BpContext(X AS INTEGER, Y AS INTEGER, Key AS INTEGER, Sender AS RTreeView)
  DIM i AS INTEGER, ok AS INTEGER, why AS STRING
  i = Sender.ContextIndex
  ok = Flag(i >= 0 AND i < BpCount)
  CtxBegin
  CtxWhere = "bp"
  CtxRow = i
  IF ok THEN
    Sender.ItemIndex = i
    BpPicked = i
    ShowPickedBreakpoint
  END IF
  why = TR$("Not on a breakpoint")
  CtxCmdIf "debug.goToBreakpoint", ok, why
  CtxDefault
  CtxCmdIf "debug.editCondition", ok, why
  IF ok THEN
    IF BpOn(i) = 0 THEN CtxCmdAs "debug.disableBreakpoint", TR$("&Enable Breakpoint"), 1, "" ELSE CtxCmd "debug.disableBreakpoint"
  ELSE
    CtxCmdIf "debug.disableBreakpoint", 0, why
  END IF
  CtxSep
  CtxCmdIf "edit.copy", ok, why
  CtxCmdIf "pane.copyAll", Flag(BpCount > 0), TR$("No breakpoints")
  CtxSep
  CtxCmdIf "debug.removeBreakpoint", ok, why
  CtxCmdIf "debug.clearBreakpoints", Flag(BpCount > 0), TR$("No breakpoints")
  CtxMenu.AccessibleName = TR$("Actions for the breakpoint")
  CtxAlign Key
  CtxMenu.PopupAt Sender, X, Y
END SUB

' ---- Help (2b) -----------------------------------------------------------------------------

' A page: Back, Forward | Copy, Select All | Open in the Browser (a page of
' the manual) | Ask RAI. A code block right-clicked: Try It, Copy Code,
' Insert at the Cursor | Ask RAI.
SUB HelpContext(X AS INTEGER, Y AS INTEGER, Key AS INTEGER, Sender AS RMarkdownView)
  DIM code AS STRING
  IF Key = 0 THEN code = Sender.ContextCode
  CtxBegin
  IF code <> "" THEN
    CtxWhere = "helpcode"
    CtxText = code
    CtxCmd "help.tryIt"
    CtxDefault
    CtxCmd "help.copyCode"
    CtxCmdIf "help.insertCode", Flag(ActiveCode >= 0), TR$("Open a code file first")
    CtxAskRAI TR$("Ask RAI About This Example..."), "```basic" + CHR$(10) + code + CHR$(10) + "```" + CHR$(10)
    CtxMenu.AccessibleName = TR$("Actions for the example")
  ELSE
    CtxWhere = "help"
    CtxCmdAs "nav.back", TR$("&Back"), Flag(HelpHistCount > 0), TR$("No page before this one")
    CtxCmdAs "nav.forward", TR$("&Forward"), Flag(HelpFwdCount > 0), TR$("No page after this one")
    CtxSep
    CtxCmdIf "edit.copy", Flag(Sender.SelText <> ""), TR$("Nothing is selected")
    CtxCmdAs "edit.selectAll", TR$("&Select All"), 1, ""
    IF HelpKind = "manual" AND HelpManualPage <> "" AND Proj.FileManager <> "" THEN
      CtxSep
      CtxCmd "help.openBrowser"
    END IF
    CtxAskRAI TR$("Ask RAI About This Page..."), TR$("About the Help page {q}{title}{q}:", "q", CHR$(34), "title", HelpTitleNow) + " "
    CtxMenu.AccessibleName = TR$("Actions for the Help page")
  END IF
  CtxAlign Key
  CtxMenu.PopupAt Sender, X, Y
END SUB

' (the Help page's title: its first heading)
FUNCTION HelpTitleNow AS STRING
  Result = FIELD$(FIELD$(HelpPage.Headings, CHR$(10), 1), CHR$(9), 2)
  IF Result = "" THEN Result = TR$("Help")
END FUNCTION

' ---- the status bar, the panes, the Welcome page, the Toolbox (2b) ------------------------

' (which of the status bar's panels X is in)
FUNCTION StatusPanelAt(X AS INTEGER) AS INTEGER
  DIM k AS INTEGER, at AS INTEGER
  Result = -1
  FOR k = 0 TO Status.PanelCount - 1
    at = at + Status.Panel(k).Width
    IF X < at THEN
      Result = k
      EXIT FUNCTION
    END IF
  NEXT
END FUNCTION

SUB StatusContext(X AS INTEGER, Y AS INTEGER, Key AS INTEGER, Sender AS RStatusBar)
  DIM k AS INTEGER
  k = StatusPanelAt(X)
  CtxBegin
  CtxWhere = "status"
  CtxRow = k
  SELECT CASE k
    CASE 0
      CtxCmdIf "status.copyMessage", Flag(StateText <> ""), TR$("No message")
    CASE 1
      IF ProjectOpen = 0 THEN EXIT SUB
      CtxPath = Proj.Folder
      IF RevealWords <> "" THEN CtxCmdAs "file.reveal", RevealWords, 1, ""
      CtxCmd "project.options"
      CtxSep
      CtxCmd "versions.pane"
    CASE 2
      CtxCmd "edit.goToLine"
    CASE 6
      IF McpStatusWidth = 0 THEN EXIT SUB
      CtxCmd "view.aiApps"
      CtxCmd "mcp.settings"
    CASE ELSE
      EXIT SUB
  END SELECT
  CtxMenu.AccessibleName = TR$("Actions for the status bar")
  CtxMenu.PopupAt Sender, X, Y
END SUB

' (School Lock's chip in the status bar)
SUB LockChipContext(X AS INTEGER, Y AS INTEGER, Key AS INTEGER)
  CtxBegin
  CtxWhere = "status"
  CtxCmd "tools.schoolLock"
  CtxMenu.AccessibleName = TR$("Actions for School Lock")
  CtxMenu.PopupAt LockChipBox, X, Y
END SUB

' A pane's tab or title (the dock manager's): Close Pane | Float or Dock |
' Reset Layout.
SUB PaneContext(X AS INTEGER, Y AS INTEGER, Key AS INTEGER, Sender AS RDockManager)
  DIM p AS STRING
  p = Sender.ContextPane
  IF p = "" THEN EXIT SUB
  CtxBegin
  CtxWhere = "pane"
  CtxPane = p
  CtxCmd "view.closePane"
  CtxSep
  IF Sender.PaneState(p) = "floating" THEN CtxCmdAs "view.floatPane", TR$("&Dock"), 1, "" ELSE CtxCmd "view.floatPane"
  CtxSep
  CtxCmd "view.layout"
  CtxMenu.AccessibleName = TR$("Actions for the pane")
  CtxAlign Key
  CtxMenu.PopupAt Sender, X, Y
END SUB

' A recent project on the Welcome page: Open | Copy Path, Reveal | Remove
' from Recent.
SUB RecentContext(X AS INTEGER, Y AS INTEGER, Key AS INTEGER, Sender AS RLabel)
  CtxBegin
  CtxWhere = "recent"
  CtxRow = Sender.Tag
  CtxPath = Sender.Hint
  IF CtxPath = "" THEN EXIT SUB
  CtxCmd "recent.open"
  CtxDefault
  CtxSep
  CtxCmdIf "file.copyPath", 1, ""
  IF RevealWords <> "" THEN CtxCmdAs "file.reveal", RevealWords, Flag(FILEEXISTS(CtxPath) <> 0), TR$("It isn't there any more")
  CtxSep
  CtxCmd "recent.remove"
  CtxMenu.AccessibleName = TR$("Actions for {what}", "what", Sender.Caption)
  CtxAlign Key
  CtxMenu.PopupAt Sender, X, Y
END SUB

' A component of the Toolbox: Add to the Form | Help, Open an Example | Ask
' RAI How to Use It; a group: Collapse All, Expand All.
SUB ToolboxContext(X AS INTEGER, Y AS INTEGER, Key AS INTEGER, Sender AS RToolbox)
  DIM kind AS STRING, nm AS STRING
  kind = Sender.ContextKind
  IF kind = "" THEN EXIT SUB
  CtxBegin
  CtxWhere = "toolbox"
  IF kind = "group" THEN
    CtxCmd "project.collapseAll"
    CtxCmd "toolbox.expandAll"
    CtxMenu.AccessibleName = TR$("Actions for the Toolbox")
  ELSE
    nm = Sender.Selected
    IF nm = "" THEN EXIT SUB
    CtxName = nm
    CtxCmdIf "toolbox.add", Flag(ActiveDesign >= 0 OR IsFormType(nm) <> 0), TR$("Open a form first")
    CtxDefault
    CtxSep
    CtxCmdAs "help.component", TR$("&Help: {name}", "name", nm), 1, ""
    CtxCmd "help.example"
    CtxAskRAI TR$("Ask RAI How to Use {name}...", "name", nm), TR$("How do I use {name}?", "name", nm) + " "
    CtxMenu.AccessibleName = TR$("Actions for {what}", "what", nm)
  END IF
  CtxAlign Key
  CtxMenu.PopupAt Sender, X, Y
END SUB

' ---- what the 2b commands do -----------------------------------------------------------

' (the surface a command acts on: the menu's; from the palette or a key,
' the pane that has the keyboard)
FUNCTION CtxSurface AS STRING
  IF CtxUse AND CtxWhere <> "" THEN
    Result = CtxWhere
    EXIT FUNCTION
  END IF
  SELECT CASE LCASE$(Dock.ActivePane)
    CASE "outputbox": IF OutputBox.Page = "problems" THEN Result = "problems" ELSE Result = "output"
    CASE "immediatebox": Result = "immediate"
    CASE "varstree": Result = "vars"
    CASE "watchpanel": Result = "watch"
    CASE "stacktree": Result = "stack"
    CASE "bppanel": Result = "bp"
    CASE "helppane": Result = "help"
    CASE ELSE: Result = ""
  END SELECT
END FUNCTION

' Explain This Problem: the problem's own Help page — its message, where it
' is, then the Help of the name it is on.
SUB ExplainProblem
  DIM msg AS STRING, wd AS STRING, where AS STRING, t AS STRING, s AS STRING, d AS INTEGER, nl AS INTEGER, kind AS STRING
  IF CtxUse AND CtxWhere = "problems" THEN
    msg = CtxText
    IF CtxPath <> "" THEN OutputLink CtxPath, CtxPlaceLine
  END IF
  d = ActiveCode
  IF d >= 0 THEN
    IF msg = "" THEN msg = CodeDoc(d).ProblemAt(CodeDoc(d).CaretLine, CodeDoc(d).CaretColumn)
    wd = CaretWord(d)
    where = TR$("Line {n} of {file}", "n", Num(CodeDoc(d).CaretLine), "file", FileTitle(DocFile(d)))
  END IF
  IF msg = "" THEN
    SetState RunState, TR$("No problem here")
    EXIT SUB
  END IF
  t = "# " + TR$("This problem") + CHR$(10) + CHR$(10) + "> " + msg + CHR$(10) + CHR$(10)
  IF where <> "" THEN t = t + where + CHR$(10) + CHR$(10)
  IF wd <> "" AND d >= 0 THEN
    s = Lang.HelpPage(wd, "", DocFile(d), CodeDoc(d).CaretLine, CodeDoc(d).CaretColumn, 0)
    nl = INSTR(s, CHR$(10))
    IF nl > 0 THEN kind = FIELD$(LEFT$(s, nl - 1), CHR$(9), 1)
    IF nl > 0 AND kind <> "" AND kind <> "start" AND kind <> "none" AND kind <> "suggest" THEN t = t + "---" + CHR$(10) + CHR$(10) + MID$(s, nl + 1)
  END IF
  Dock.ShowPane("helppane")
  HelpKind = "problem"
  HelpPage.Text = t
  HelpPage.ScrollTo "#"
  Application.Announce TR$("Help: this problem")
END SUB

' Delete File...: asked first (a question, the safe answer the default);
' then the file to the system's Trash (MOVETOTRASH) and out of the project.
' Never the main file, never a file outside the project's folder.
SUB DeleteFileToTrash(Path AS STRING)
  DIM yes AS INTEGER, d AS INTEGER
  IF Path = "" OR ProjectOpen = 0 THEN EXIT SUB
  IF LCASE$(Path) = LCASE$(ProjectMain) OR CtxInFolder(Path) = 0 THEN EXIT SUB
  ConsentBegin TR$("Move {file} to the Trash?", "file", FileTitle(Path)), TR$("It is taken out of the project too. You can put it back from the Trash.")
  yes = ConsentAsk("", "", TR$("&Move to Trash"), TR$("Cancel"))
  IF yes = 0 THEN EXIT SUB
  ' (its document closes first — a change not saved asks, as Close does)
  d = DocOf(Path)
  IF d >= 0 THEN
    CloseDocument d
    IF DocOpen(d) THEN EXIT SUB
  END IF
  IF MOVETOTRASH(Path) = 0 THEN
    SetState "error", TR$("{file} couldn't be moved to the Trash", "file", FileTitle(Path))
    OutputLine TR$("{file} couldn't be moved to the Trash ({path}): it may be open in another program, or its disk has no Trash.", "file", FileTitle(Path), "path", Path)
    EXIT SUB
  END IF
  Proj.RemoveFile Path
  AddSaved
  IncludeLeaves Path
  SetState RunState, TR$("{file} moved to the Trash", "file", FileTitle(Path))
  Application.Announce TR$("{file} moved to the Trash", "file", FileTitle(Path))
END SUB

' Try It: a Help page's example in a file of its own (Studio's projects
' folder, "Try It"), open in the editor — F5 runs it.
SUB HelpTryIt(Code AS STRING)
  DIM folder AS STRING, p AS STRING, k AS INTEGER, f AS INTEGER
  IF Code = "" THEN EXIT SUB
  folder = DefaultFolder + "/" + TR$("Try It")
  IF DIREXISTS(folder) = 0 THEN MKDIR folder
  k = 1
  p = folder + "/example-1.rr"
  DO WHILE FILEEXISTS(p) AND k < 999
    k = k + 1
    p = folder + "/example-" + LTRIM$(STR$(k)) + ".rr"
  LOOP
  f = FREEFILE
  OPEN p FOR OUTPUT AS #f
  PRINT #f, Code
  CLOSE #f
  IF FILEEXISTS(p) = 0 THEN
    SetState "error", TR$("Can't write {path}", "path", p)
    EXIT SUB
  END IF
  OpenDocument p
  SetState RunState, TR$("{file} is yours to change: F5 runs it", "file", FileTitle(p))
END SUB

' Compare With > Another File...: that file on the left, this one on the right.
SUB CompareWithOther(Path AS STRING)
  OpenDlg.Caption = TR$("Compare With")
  OpenDlg.Filter = TR$("All files") + "|*.*"
  IF OpenDlg.Execute = 0 THEN EXIT SUB
  VerDiffFile = Path
  VerDiffOther = OpenDlg.FileName
  VerShowDiff "other"
END SUB

' ---- the commands --------------------------------------------------------------------------

' (a value as code: numbers, constants and True / False as they are, text quoted)
FUNCTION CodeValue(V AS STRING) AS STRING
  DIM u AS STRING
  u = UCASE$(V)
  IF V = "" THEN
    Result = CHR$(34) + CHR$(34)
  ELSEIF STR$(VAL(V)) = " " + V OR LTRIM$(STR$(VAL(V))) = V OR LEFT$(u, 2) = "&H" OR u = "TRUE" OR u = "FALSE" THEN
    Result = V
  ELSEIF LEN(V) > 2 AND LCASE$(LEFT$(V, 2)) = LEFT$(V, 2) AND UCASE$(MID$(V, 3, 1)) = MID$(V, 3, 1) AND INSTR(V, " ") = 0 THEN
    ' (a constant: alClient, bsSingle, clBtnFace)
    Result = V
  ELSE
    Result = CHR$(34) + REPLACESUBSTR$(V, CHR$(34), CHR$(34) + CHR$(34)) + CHR$(34)
  END IF
END FUNCTION

SUB CopySaid(What AS STRING, Text AS STRING)
  Clipboard.Text = Text
  SetState RunState, What
END SUB

' True when Id was one of the right-click menus' own commands.
FUNCTION ContextCommand(Id AS STRING) AS INTEGER
  DIM f AS STRING, k AS INTEGER, d AS INTEGER, at AS INTEGER, prop AS STRING, n AS STRING, w AS STRING
  Result = 1
  SELECT CASE Id
    CASE "project.openFile"
      IF CtxUse AND CtxTreePath <> "" THEN ProjectNodeOpen CtxTreePath ELSE ProjectNodeOpen ProjectTree.Selected
    CASE "project.rename"
      IF CtxUse AND CtxTreePath <> "" THEN ProjectTree.Rename CtxTreePath ELSE ProjectTree.Rename ProjectTree.Selected
    CASE "project.removeFile"
      Dock.FocusPane("projecttree")
      IF CtxUse AND CtxTreePath <> "" THEN ProjectTree.Delete CtxTreePath ELSE ProjectTree.Delete ProjectTree.Selected
    CASE "project.showFile"
      f = CtxFile
      IF f <> "" THEN
        ProjectTree.Reveal CtxRelative(f)
        Dock.FocusPane("projecttree")
      END IF
    CASE "project.collapseAll"
      IF CtxUse AND CtxWhere = "toolbox" THEN Toolbox.CollapseAll ELSE ProjectTree.CollapseAll
    CASE "file.copyPath"
      f = CtxFile
      IF f <> "" THEN CopySaid TR$("Copied the path"), f
    CASE "file.copyRelativePath"
      f = CtxFile
      IF f <> "" THEN CopySaid TR$("Copied the relative path"), CtxRelative(f)
    CASE "file.reveal"
      f = CtxFile
      IF f <> "" THEN
        IF Proj.Reveal(f) = 0 THEN OutputLine TR$("Can't show it: {error}", "error", Proj.Error)
      END IF
    CASE "file.closeOthers", "file.closeToRight", "file.closeSaved"
      IF CtxUse AND CtxDoc >= 0 THEN d = CtxDoc ELSE d = ActiveDoc
      IF Id = "file.closeSaved" THEN
        FOR k = DocCount - 1 TO 0 STEP -1
          IF DocOpen(k) AND DocDirty(k) = 0 THEN CloseDocument k
        NEXT
      ELSEIF Id = "file.closeOthers" THEN
        FOR k = DocCount - 1 TO 0 STEP -1
          IF DocOpen(k) AND k <> d THEN CloseDocument k
        NEXT
      ELSE
        ' (the tabs after it, in the strip's order)
        at = -1
        FOR k = 0 TO Dock.DocumentCount - 1
          IF DocOfPane(Dock.Document(k)) = d THEN at = k
        NEXT
        IF at >= 0 THEN
          FOR k = Dock.DocumentCount - 1 TO at + 1 STEP -1
            CloseDocument DocOfPane(Dock.Document(k))
          NEXT
        END IF
      END IF
    CASE "designer.editHandler"
      d = ActiveDesign
      IF d >= 0 THEN
        IF DesignDoc(d).SelIndex >= 0 THEN DesignDblClick DesignDoc(d).SelIndex, DesignDoc(d)
      END IF
    CASE "inspector.editValue", "inspector.reset", "inspector.copyValue", "inspector.copyName", "inspector.copyCode", "inspector.goToHandler", "inspector.removeHandler", "inspector.help"
      IF CtxUse AND CtxName <> "" THEN prop = CtxName ELSE prop = Inspector.Selected
      IF prop = "" THEN EXIT FUNCTION
      SELECT CASE Id
        CASE "inspector.editValue": Inspector.EditValue prop
        CASE "inspector.reset": Inspector.ResetValue prop
        CASE "inspector.copyValue": CopySaid TR$("Copied the value"), Inspector.Value(prop)
        CASE "inspector.copyName": CopySaid TR$("Copied the name"), prop
        CASE "inspector.copyCode"
          IF Inspector.Page = "events" THEN
            n = Inspector.Target + "." + prop + " = " + Inspector.Value(prop)
          ELSE
            n = Inspector.Target + "." + prop + " = " + CodeValue(Inspector.Value(prop))
          END IF
          CopySaid TR$("Copied the code"), n
        CASE "inspector.goToHandler": InspectorEvent prop
        CASE "inspector.removeHandler"
          IF Inspector.SetValue(prop, "") = 0 THEN SetState RunState, TR$("The handler couldn't be removed")
        CASE "inspector.help": ShowHelp prop, Inspector.TargetType
      END SELECT
    CASE "ai.askAbout"
      ShowAIPanel
      IF AIReady AND LockOffAI = 0 THEN
        IF CtxUse THEN AIInput.Text = CtxAsk
        AIInput.SetFocus
        AIInput.SelStart = LEN(AIInput.Text)
      END IF
    CASE "ai.chatOpen": AIChatsOpenClick
    CASE "ai.chatRename": AIChatsRenameClick
    CASE "ai.chatDelete": AIChatsDeleteClick
    ' ---- 2b ----
    CASE "ai.chatCopyMarkdown": AIChatsCopyMarkdown
    CASE "edit.copy", "edit.selectAll", "edit.paste"
      w = CtxSurface
      IF CtxUse = 0 OR CtxWhere = "" OR CtxWhere = "code" THEN w = ""
      SELECT CASE w
        CASE "output", "problems"
          IF Id = "edit.copy" THEN OutputCopy
          IF Id = "edit.selectAll" THEN OutputSelectAll
        CASE "immediate"
          IF Id = "edit.copy" THEN ImmediateBox.CopyToClipboard
          IF Id = "edit.selectAll" THEN ImmediateBox.SelectAll
          IF Id = "edit.paste" THEN
            ImmediateBox.SetFocus
            ImmediateBox.PasteFromClipboard
          END IF
        CASE "stack": IF Id = "edit.copy" THEN StackCopy
        CASE "bp": IF Id = "edit.copy" THEN BpCopy
        CASE "help"
          IF Id = "edit.copy" THEN CopySaid TRN$("Copied {n} character", "Copied {n} characters", LEN(HelpPage.SelText)), HelpPage.SelText
          IF Id = "edit.selectAll" THEN HelpPage.SelectAll
        CASE ELSE
          Result = 0
      END SELECT
    CASE "problems.goTo", "output.goTo"
      IF CtxUse AND CtxPath <> "" THEN
        OutputLink CtxPath, CtxPlaceLine
      ELSEIF OutputBox.ClickedLine >= 0 THEN
        f = OutputBox.PlaceFile(OutputBox.ClickedLine)
        IF f <> "" THEN OutputLink f, OutputBox.PlaceLine(OutputBox.ClickedLine)
      END IF
    CASE "edit.quickFix"
      IF CtxUse AND CtxWhere = "problems" THEN
        IF CtxPath <> "" THEN OutputLink CtxPath, CtxPlaceLine
        IF ActiveCode >= 0 THEN CodeDoc(ActiveCode).QuickFix
      ELSE
        Result = 0
      END IF
    CASE "edit.explainProblem": ExplainProblem
    CASE "edit.applyFix"
      d = ActiveCode
      IF d >= 0 THEN
        IF CodeDoc(d).ApplyFixAt(CodeDoc(d).CaretLine, CodeDoc(d).CaretColumn) THEN
          CodeChanged CodeDoc(d)
        ELSE
          SetState RunState, TR$("No fix here")
        END IF
      END IF
    CASE "pane.copyLine"
      IF OutputBox.CopyLine THEN OutputCopied OutputBox.Line(OutputBox.ClickedLine)
    CASE "pane.copyAll"
      SELECT CASE CtxSurface
        CASE "stack": ListCopyAll 0
        CASE "bp": ListCopyAll 1
        CASE ELSE: OutputCopyAll
      END SELECT
    CASE "pane.clear"
      IF CtxSurface = "immediate" THEN ImmediateBox.Clear ELSE ClearOutputPage
    CASE "debug.addWatch"
      IF CtxUse AND CtxWhere = "vars" THEN ValueAction "watch", PickedExpr(0), PickedShown(0) ELSE Result = 0
    CASE "debug.setValue", "debug.copyValue", "debug.copyExpression", "debug.viewText", "debug.removeWatch"
      w = CtxSurface
      IF w = "vars" THEN k = 0 ELSE k = 1
      IF w <> "vars" AND w <> "watch" THEN EXIT FUNCTION
      SELECT CASE Id
        CASE "debug.setValue"
          IF k = 0 THEN BeginSetValue 0, VarsTree.ItemIndex ELSE BeginSetValue 1, WatchTree.ItemIndex
        CASE "debug.copyValue": ValueAction "copy", PickedExpr(k), PickedShown(k)
        CASE "debug.copyExpression": ValueAction "copyexpr", PickedExpr(k), PickedShown(k)
        CASE "debug.viewText": ValueAction "view", PickedExpr(k), PickedShown(k)
        CASE "debug.removeWatch"
          at = 46
          IF k = 1 THEN WatchTreeKey at, 0
      END SELECT
    CASE "debug.goToCall"
      IF CtxUse AND CtxRow >= 0 THEN k = CtxRow ELSE k = StackTree.ItemIndex
      IF k >= 0 AND k < FrameCount THEN
        TreeFilling = 1
        StackTree.ItemIndex = k
        TreeFilling = 0
        StackPicked
      END IF
    CASE "debug.goToBreakpoint": BpTreeOpen
    CASE "debug.editCondition", "debug.disableBreakpoint", "debug.removeBreakpoint"
      k = BpTree.ItemIndex
      IF k < 0 OR k >= BpCount THEN EXIT FUNCTION
      SELECT CASE Id
        CASE "debug.editCondition"
          BpPicked = k
          ShowPickedBreakpoint
          Dock.FocusPane("bppanel")
          BpCondEdit.SetFocus
        CASE "debug.disableBreakpoint": ChangeBreakpoint k, BpCond(k), BpHit(k), BpLog(k), 1 - BpOn(k)
        CASE "debug.removeBreakpoint"
          RemoveBreakpoint k
          FillBreakpoints
      END SELECT
    CASE "nav.back", "nav.forward"
      IF CtxSurface = "help" THEN
        IF Id = "nav.back" THEN HelpGoBack ELSE HelpGoForward
      ELSE
        Result = 0
      END IF
    CASE "help.openBrowser": IF HelpManualPage <> "" THEN OpenWebLink "https://github.com/iBobX/RapidR/blob/main/docs/manual/" + HelpManualPage
    CASE "help.tryIt": HelpTryIt CtxText
    CASE "help.copyCode": IF CtxText <> "" THEN CopySaid TR$("Copied the code"), CtxText
    CASE "help.insertCode"
      d = ActiveCode
      IF d >= 0 AND CtxText <> "" THEN
        Dock.FocusPane(DocPane(d))
        CodeDoc(d).SelText = CtxText
        CodeChanged CodeDoc(d)
      END IF
    CASE "help.component"
      IF CtxUse AND CtxWhere = "toolbox" THEN
        ShowHelp CtxName, ""
      ELSEIF CtxUse AND CtxText <> "" THEN
        ShowHelp CtxText, ""
      ELSEIF CtxUse AND CtxTreePath <> "" THEN
        ProjectNodeOpen CtxTreePath
        IF Inspector.TargetType <> "" THEN ShowHelp Inspector.TargetType, ""
      ELSE
        HelpForFocus
      END IF
    CASE "help.example": ShowPalette TR$("Example") + " " + CtxName
    CASE "status.copyMessage": CopySaid TR$("Copied the message"), TR$(StateText)
    CASE "view.closePane", "view.floatPane"
      IF CtxUse AND CtxPane <> "" THEN f = CtxPane ELSE f = Dock.ActivePane
      IF f = "" THEN EXIT FUNCTION
      IF Id = "view.closePane" THEN
        Dock.HidePane(f)
      ELSEIF Dock.PaneState(f) = "floating" THEN
        Dock.DockBack(f)
      ELSE
        Dock.FloatPane(f)
      END IF
    CASE "recent.open": IF CtxUse AND CtxRow >= 0 THEN OpenRecent CtxRow + 1
    CASE "recent.remove"
      IF CtxUse AND CtxPath <> "" THEN
        ForgetRecent CtxPath
        SaveSettings
        UpdateCommands
        SetState RunState, TR$("{file} taken off Recent", "file", FileTitle(CtxPath))
      END IF
    CASE "toolbox.add": IF Toolbox.Selected <> "" THEN ToolboxPick Toolbox.Selected
    CASE "toolbox.expandAll": Toolbox.ExpandAll
    CASE "file.compareDisk"
      IF CtxUse AND CtxPath <> "" AND DocOf(CtxPath) >= 0 THEN ExtCompareDoc DocOf(CtxPath) ELSE Result = 0
    CASE "file.compareLast"
      f = CtxFile
      IF f <> "" THEN
        VerDiffFile = f
        VerShowDiff "last"
      END IF
    CASE "file.compareOther"
      f = CtxFile
      IF f <> "" THEN CompareWithOther f
    CASE "file.deleteFile"
      IF CtxUse AND CtxPath <> "" THEN DeleteFileToTrash CtxPath
    CASE ELSE
      Result = 0
  END SELECT
END FUNCTION
