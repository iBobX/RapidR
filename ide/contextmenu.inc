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
  RunCommand id
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
      IF versioned THEN
        CtxSep
        CtxCmdAs "versions.localHistory", TR$("Show &History"), 1, ""
      END IF
      CtxAskRAI TR$("Ask RAI About This File..."), TR$("About {file}:", "file", sel) + " "
      CtxSep
      CtxCmdIf "project.removeFile", Flag(isMain = 0), TR$("The main file stays in the project")
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
  IF doc = "" THEN EXIT SUB
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
  DIM d AS INTEGER, wd AS STRING, sel AS INTEGER, ro AS INTEGER, q AS STRING
  d = Sender.Tag
  IF d < 0 OR d >= DocCount THEN EXIT SUB
  IF DocOpen(d) = 0 THEN EXIT SUB
  CtxBegin
  CtxPath = DocFile(d)
  wd = CaretWord(d)
  sel = Flag(Sender.SelLength > 0)
  ro = Flag(Sender.ReadOnly <> 0)
  CtxCmdIf "edit.goToDefinition", Flag(wd <> ""), TR$("Not on a name")
  CtxCmdIf "edit.findReferences", Flag(wd <> ""), TR$("Not on a name")
  CtxCmdIf "edit.rename", Flag(wd <> "" AND ro = 0), TR$("Not on a name")
  CtxCmd "edit.quickFix"
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
  CtxAskRAI TR$("Ask RAI About This Code..."), q
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
  CtxSep
  CtxCmd "ai.chatDelete"
  CtxMenu.AccessibleName = TR$("Actions for the chat")
  CtxAlign Key
  CtxMenu.PopupAt Sender, X, Y
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
  DIM f AS STRING, k AS INTEGER, d AS INTEGER, at AS INTEGER, prop AS STRING, n AS STRING
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
      ProjectTree.CollapseAll
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
    CASE ELSE
      Result = 0
  END SELECT
END FUNCTION
