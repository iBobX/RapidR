' commands.inc: the command table — one row per command — and what is made
' from it: the menu bar, the tool bar, the command palette's list; which
' commands are enabled now; RunCommand, where every one of them goes.
'
' Command ids are RapidR's icon inventory's (design/icons/inventory.toml),
' so an id is also its icon's name (Bitmap.LoadIcon takes either).

SUB AddCmd(Id AS STRING, Title AS STRING, Category AS STRING, Shortcut AS STRING, Shown AS STRING, EnabledWhen AS STRING, Icon AS STRING)
  IF CmdCount >= MAXCMDS THEN EXIT SUB
  Cmd(CmdCount).Id = Id
  Cmd(CmdCount).Title = Title
  Cmd(CmdCount).Category = Category
  Cmd(CmdCount).Shortcut = Shortcut
  Cmd(CmdCount).Shown = Shown
  Cmd(CmdCount).EnabledWhen = EnabledWhen
  Cmd(CmdCount).Icon = Icon
  Cmd(CmdCount).Item = -1
  Cmd(CmdCount).Tool = -1
  CmdCount = CmdCount + 1
END SUB

' (a separator in a category's menu: a row without an id)
SUB AddSep(Category AS STRING)
  AddCmd "", "-", Category, "", "", "", ""
END SUB

' (a submenu: Category "View" gets an item Title holding "View/Name")
SUB AddSubmenu(Category AS STRING, Name AS STRING, Title AS STRING)
  AddCmd "", Title, Category, "", "", "", "menu:" + Category + "/" + Name
END SUB

FUNCTION CmdIndex(Id AS STRING) AS INTEGER
  DIM i AS INTEGER
  Result = -1
  FOR i = 0 TO CmdCount - 1
    IF Cmd(i).Id = Id THEN
      Result = i
      EXIT FUNCTION
    END IF
  NEXT
END FUNCTION

' A title as text: no "&", no "...".
FUNCTION PlainTitle(Title AS STRING) AS STRING
  DIM t AS STRING
  t = REPLACESUBSTR$(Title, "&", "")
  IF RIGHT$(t, 3) = "..." THEN t = LEFT$(t, LEN(t) - 3)
  Result = t
END FUNCTION

' The key a command answers to, as the user reads it.
FUNCTION CmdKeys(i AS INTEGER) AS STRING
  IF Cmd(i).Shortcut <> "" THEN Result = Cmd(i).Shortcut ELSE Result = Cmd(i).Shown
END FUNCTION

SUB DefineCommands
  ' ---- File ----
  AddCmd "file.newProject", "&New Project...", "File", "Ctrl+N", "", "", ""
  AddCmd "file.open", "&Open Project or File...", "File", "Ctrl+O", "", "", ""
  AddCmd "file.openFolder", "Open &Folder...", "File", "", "", "", "files/folder-open"
  AddCmd "file.openExample", "Open &Example...", "File", "", "", "", "RFORM"
  AddSep "File"
  AddCmd "file.save", "&Save", "File", "Ctrl+S", "", "document", ""
  AddCmd "file.saveAs", "Save &As...", "File", "", "", "document", "-"
  AddCmd "file.saveAll", "Save A&ll", "File", "Ctrl+Shift+S", "", "project", ""
  AddSep "File"
  AddCmd "file.close", "&Close Document", "File", "Ctrl+F4", "", "document", ""
  AddCmd "file.closeProject", "Close Pro&ject", "File", "", "", "project", "-"
  AddSep "File"
  AddCmd "file.exit", "E&xit", "File", "", "Alt+F4", "", "-"

  ' ---- Edit (the editing keys stay the focused control's) ----
  AddCmd "edit.undo", "&Undo", "Edit", "", "Ctrl+Z", "document", ""
  AddCmd "edit.redo", "&Redo", "Edit", "", "Ctrl+Y", "document", ""
  AddSep "Edit"
  AddCmd "edit.cut", "Cu&t", "Edit", "", "Ctrl+X", "document", ""
  AddCmd "edit.copy", "&Copy", "Edit", "", "Ctrl+C", "document", ""
  AddCmd "edit.paste", "&Paste", "Edit", "", "Ctrl+V", "document", ""
  AddCmd "edit.delete", "&Delete", "Edit", "", "Del", "document", ""
  AddCmd "edit.selectAll", "Select &All", "Edit", "", "Ctrl+A", "document", ""
  AddCmd "edit.duplicate", "D&uplicate", "Edit", "", "Ctrl+D", "design", ""
  AddSep "Edit"
  AddCmd "edit.find", "&Find...", "Edit", "Ctrl+F", "", "code", ""
  AddCmd "edit.replace", "R&eplace...", "Edit", "Ctrl+H", "", "code", ""
  AddCmd "edit.goToLine", "&Go to Line...", "Edit", "Ctrl+G", "", "code", ""

  ' ---- View ----
  AddCmd "view.commandPalette", "Command &Palette...", "View", "Ctrl+Shift+P", "", "", ""
  AddSep "View"
  AddCmd "view.project", "&Project", "View", "", "", "", "files/rrproj"
  AddCmd "view.toolbox", "&Toolbox", "View", "", "", "", "groups/standard"
  AddCmd "view.properties", "P&roperties", "View", "F4", "", "", "symbols/property"
  AddCmd "view.outline", "O&utline", "View", "", "", "", "symbols/module"
  AddCmd "view.output", "&Output", "View", "", "", "", ""
  AddCmd "view.problems", "Pro&blems", "View", "", "", "", ""
  AddCmd "view.immediate", "&Immediate", "View", "", "", "", "terminal"
  AddCmd "view.welcome", "&Welcome Page", "View", "", "", "", "help"
  AddSep "View"
  AddCmd "view.code", "&Code", "View", "F7", "", "document", ""
  AddCmd "view.designer", "&Designer", "View", "Shift+F7", "", "document", "components/designsurface"
  AddSep "View"
  AddSubmenu "View", "Theme", "T&heme"
  AddCmd "view.theme.rapidr", "&RapidR (as the System)", "View/Theme", "", "", "", "-"
  AddCmd "view.theme.light", "RapidR &Light", "View/Theme", "", "", "", "-"
  AddCmd "view.theme.dark", "RapidR &Dark", "View/Theme", "", "", "", "-"
  AddCmd "view.theme.highcontrast", "RapidR &High Contrast", "View/Theme", "", "", "", "-"
  AddSep "View/Theme"
  AddCmd "view.theme.classic", "&Classic (RapidQ's Windows)", "View/Theme", "", "", "", "-"
  AddSubmenu "View", "Documents", "&Documents As"
  AddCmd "view.documents.mdi", "&Windows (MDI)", "View/Documents", "", "", "", "-"
  AddCmd "view.documents.tabs", "&Tabs", "View/Documents", "", "", "", "-"
  AddSep "View"
  AddCmd "view.layout", "&Reset Layout", "View", "", "", "", ""

  ' ---- Project ----
  AddCmd "project.addFile", "Add &File...", "Project", "", "", "project", "file.new"
  AddCmd "project.addForm", "Add F&orm", "Project", "", "", "project", "RFORM"
  AddCmd "project.addModule", "Add &Module", "Project", "", "", "project", "files/rr"
  AddSep "Project"
  AddCmd "project.options", "Project &Options...", "Project", "", "", "project", "settings"

  ' ---- Run ----
  AddCmd "run.start", "&Run", "Run", "F5", "", "idle", ""
  AddCmd "run.withoutDebugging", "Run &Without Debugging", "Run", "Ctrl+F5", "", "idle", ""
  AddCmd "run.stop", "&Stop", "Run", "Shift+F5", "", "session", ""
  AddCmd "run.restart", "R&estart", "Run", "Ctrl+Shift+F5", "", "session", ""
  AddSep "Run"
  AddCmd "run.build", "&Build", "Run", "Ctrl+Shift+B", "", "project", ""
  AddCmd "run.reveal", "Reveal in " + Proj.FileManager, "Run", "", "", "project", ""

  ' ---- Debug ----
  AddCmd "run.continue", "&Continue", "Debug", "F5", "", "paused", ""
  AddCmd "run.pause", "&Pause", "Debug", "", "", "running", ""
  AddSep "Debug"
  AddCmd "debug.stepInto", "Step &Into", "Debug", "F11", "", "project", ""
  AddCmd "debug.stepOver", "Step &Over", "Debug", "F10", "", "project", ""
  AddCmd "debug.stepOut", "Step O&ut", "Debug", "Shift+F11", "", "paused", ""
  AddCmd "debug.runToCursor", "Run to C&ursor", "Debug", "Ctrl+F10", "", "code", ""
  AddSep "Debug"
  AddCmd "debug.toggleBreakpoint", "Toggle &Breakpoint", "Debug", "F9", "", "code", ""
  AddCmd "debug.clearBreakpoints", "&Delete All Breakpoints", "Debug", "Ctrl+Shift+F9", "", "project", "-"
  AddCmd "debug.addWatch", "Add &Watch...", "Debug", "", "", "project", ""

  ' ---- Format (the designer's) ----
  AddCmd "designer.alignLeft", "Align &Lefts", "Format", "", "", "design", ""
  AddCmd "designer.alignRight", "Align &Rights", "Format", "", "", "design", ""
  AddCmd "designer.alignTop", "Align &Tops", "Format", "", "", "design", ""
  AddCmd "designer.alignBottom", "Align &Bottoms", "Format", "", "", "design", ""
  AddCmd "designer.alignCenters", "Align C&enters", "Format", "", "", "design", ""
  AddCmd "designer.alignMiddles", "Align &Middles", "Format", "", "", "design", ""
  AddSep "Format"
  AddCmd "designer.sameWidth", "Make Same &Width", "Format", "", "", "design", ""
  AddCmd "designer.sameHeight", "Make Same &Height", "Format", "", "", "design", ""
  AddCmd "designer.sameSize", "Make Same &Size", "Format", "", "", "design", ""
  AddSep "Format"
  AddCmd "designer.spaceHorizontally", "Space Evenly Acr&oss", "Format", "", "", "design", ""
  AddCmd "designer.spaceVertically", "Space Evenly &Down", "Format", "", "", "design", ""
  AddCmd "designer.centerHorizontally", "Center H&orizontally", "Format", "", "", "design", ""
  AddCmd "designer.centerVertically", "Center &Vertically", "Format", "", "", "design", ""
  AddSep "Format"
  AddCmd "designer.bringToFront", "Bring to &Front", "Format", "", "", "design", ""
  AddCmd "designer.sendToBack", "Send to Bac&k", "Format", "", "", "design", ""

  ' ---- Tools ----
  AddCmd "view.settings", "&Settings...", "Tools", "", "", "", ""
  AddCmd "view.keyboardShortcuts", "&Keyboard Shortcuts", "Tools", "", "", "", ""

  ' ---- Window ----
  AddCmd "window.cascade", "&Cascade", "Window", "", "", "document", ""
  AddCmd "window.tileHorizontally", "Tile &Horizontally", "Window", "", "", "document", ""
  AddCmd "window.tileVertically", "Tile &Vertically", "Window", "", "", "document", ""
  AddSep "Window"
  AddCmd "window.next", "&Next Document", "Window", "", "Ctrl+Tab", "document", "nav.forward"
  AddCmd "window.previous", "&Previous Document", "Window", "", "Ctrl+Shift+Tab", "document", "nav.back"
  AddCmd "window.nextArea", "Next &Area", "Window", "", "F6", "", "-"
  AddSep "Window"
  AddCmd "file.closeAll", "Close &All Documents", "Window", "", "", "document", ""

  ' ---- Help ----
  AddCmd "help.contents", "RapidR &Help", "Help", "F1", "", "", ""
  AddCmd "help.examples", "&Examples", "Help", "", "", "", "RFORM"
  AddSep "Help"
  AddCmd "help.about", "&About RapidR Studio", "Help", "", "", "", "-"
END SUB

' ---- menus ---------------------------------------------------------------------

FUNCTION MenuIndex(Name AS STRING) AS INTEGER
  DIM i AS INTEGER
  Result = -1
  FOR i = 0 TO MenuCount - 1
    IF MenuName(i) = Name THEN
      Result = MenuItemOf(i)
      EXIT FUNCTION
    END IF
  NEXT
END FUNCTION

' A top menu on the bar.
SUB AddTopMenu(Name AS STRING, Title AS STRING)
  MI(MICount).Caption = Title
  MainMenu.AddItems MI(MICount)
  MenuName(MenuCount) = Name
  MenuItemOf(MenuCount) = MICount
  MenuCount = MenuCount + 1
  MICount = MICount + 1
END SUB

' Every command into its menu, in the table's order.
SUB BuildMenus
  DIM i AS INTEGER, parent AS INTEGER, caption AS STRING
  AddTopMenu "File", "&File"
  AddTopMenu "Edit", "&Edit"
  AddTopMenu "View", "&View"
  AddTopMenu "Project", "&Project"
  AddTopMenu "Run", "&Run"
  AddTopMenu "Debug", "&Debug"
  AddTopMenu "Format", "F&ormat"
  AddTopMenu "Tools", "&Tools"
  AddTopMenu "Window", "&Window"
  AddTopMenu "Help", "&Help"
  FOR i = 0 TO CmdCount - 1
    parent = MenuIndex(Cmd(i).Category)
    IF parent >= 0 AND MICount < MAXMENUS THEN
      caption = Cmd(i).Title
      IF Cmd(i).Shown <> "" THEN caption = caption + CHR$(9) + Cmd(i).Shown
      MI(MICount).Caption = caption
      IF Cmd(i).Shortcut <> "" THEN MI(MICount).ShortCut = Cmd(i).Shortcut
      IF LEFT$(Cmd(i).Icon, 5) = "menu:" THEN
        ' (a submenu: its items come under it)
        MenuName(MenuCount) = MID$(Cmd(i).Icon, 6)
        MenuItemOf(MenuCount) = MICount
        MenuCount = MenuCount + 1
      ELSEIF Cmd(i).Id <> "" THEN
        MI(MICount).Tag = i
        MI(MICount).OnClick = MenuPicked
        Cmd(i).Item = MICount
      END IF
      MI(parent).AddItems MI(MICount)
      MICount = MICount + 1
    END IF
  NEXT
END SUB

' ---- the tool bar ---------------------------------------------------------------

' (a command's icon: its own name unless the table names another; "-": none)
FUNCTION CmdIcon(i AS INTEGER) AS STRING
  IF Cmd(i).Icon = "" THEN Result = Cmd(i).Id ELSE Result = Cmd(i).Icon
END FUNCTION

SUB AddTool(Id AS STRING)
  DIM i AS INTEGER, hint AS STRING
  i = CmdIndex(Id)
  IF i < 0 OR TBCount >= MAXTOOLS THEN EXIT SUB
  TB(TBCount).Parent = ToolBar
  TB(TBCount).Left = TBNextX: TB(TBCount).Top = 5
  TB(TBCount).Width = 30: TB(TBCount).Height = 30
  TB(TBCount).Flat = 1
  TB(TBCount).Tag = i
  hint = PlainTitle(Cmd(i).Title)
  TB(TBCount).AccessibleName = hint
  IF CmdKeys(i) <> "" THEN hint = hint + " (" + CmdKeys(i) + ")"
  TB(TBCount).Hint = hint
  TB(TBCount).ShowHint = 1
  TB(TBCount).OnClick = ToolPicked
  TBCmd(TBCount) = i
  Cmd(i).Tool = TBCount
  TBNextX = TBNextX + 32
  TBCount = TBCount + 1
END SUB

SUB AddToolSep
  IF TBSepCount > 12 THEN EXIT SUB
  TBSep(TBSepCount).Parent = ToolBar
  TBSep(TBSepCount).BevelOuter = 0
  TBSep(TBSepCount).Left = TBNextX + 6: TBSep(TBSepCount).Top = 12
  TBSep(TBSepCount).Width = 1: TBSep(TBSepCount).Height = 16
  TBSepCount = TBSepCount + 1
  TBNextX = TBNextX + 14
END SUB

SUB BuildToolBar
  TBNextX = 8
  AddTool "file.newProject": AddTool "file.open": AddTool "file.save": AddTool "file.saveAll"
  AddToolSep
  AddTool "edit.undo": AddTool "edit.redo"
  AddToolSep
  AddTool "edit.cut": AddTool "edit.copy": AddTool "edit.paste": AddTool "edit.find"
  AddToolSep
  AddTool "run.start": AddTool "run.withoutDebugging": AddTool "run.pause": AddTool "run.stop": AddTool "run.build"
  AddToolSep
  AddTool "debug.stepInto": AddTool "debug.stepOver": AddTool "debug.stepOut": AddTool "debug.toggleBreakpoint"
  AddToolSep
  AddTool "designer.alignLeft": AddTool "designer.alignTop": AddTool "designer.sameWidth": AddTool "designer.bringToFront"
  AddToolSep
  AddTool "view.commandPalette": AddTool "view.settings"
END SUB

' The tool bar's icons, drawn in the current theme (again after a switch).
SUB LoadToolIcons
  DIM k AS INTEGER
  FOR k = 0 TO TBCount - 1
    TBIcon(k).LoadIcon CmdIcon(TBCmd(k)), 16
    TB(k).BMPHandle = TBIcon(k)
  NEXT
END SUB

' ---- enabled commands --------------------------------------------------------------

FUNCTION CmdEnabled(i AS INTEGER) AS INTEGER
  DIM w AS STRING, k AS STRING
  w = Cmd(i).EnabledWhen
  k = ""
  IF ActiveDoc >= 0 THEN k = DocKind(ActiveDoc)
  SELECT CASE w
    CASE "": Result = 1
    CASE "project": Result = ProjectOpen
    CASE "document": Result = (k = "code" OR k = "design")
    CASE "code": Result = (k = "code")
    CASE "design": Result = (k = "design")
    CASE "idle": Result = (RunState = "stopped" AND ProjectOpen <> 0)
    CASE "running": Result = (RunState = "running")
    CASE "paused": Result = (RunState = "paused")
    CASE "session": Result = (RunState <> "stopped")
    CASE ELSE: Result = 1
  END SELECT
END FUNCTION

' (1 when x holds, else 0)
FUNCTION Flag(x AS INTEGER) AS INTEGER
  IF x THEN Result = 1 ELSE Result = 0
END FUNCTION

' (a command's menu item checked or not)
SUB CheckCmd(Id AS STRING, Checked AS INTEGER)
  DIM i AS INTEGER
  i = CmdIndex(Id)
  IF i >= 0 THEN
    IF Cmd(i).Item >= 0 THEN MI(Cmd(i).Item).Checked = Flag(Checked)
  END IF
END SUB

SUB UpdateCommands
  DIM i AS INTEGER, en AS INTEGER
  FOR i = 0 TO CmdCount - 1
    IF Cmd(i).Id <> "" THEN
      en = Flag(CmdEnabled(i))
      IF Cmd(i).Item >= 0 THEN MI(Cmd(i).Item).Enabled = en
      IF Cmd(i).Tool >= 0 THEN TB(Cmd(i).Tool).Enabled = en
    END IF
  NEXT
  ' (the radio items say what is chosen)
  CheckCmd "view.theme.rapidr", ThemeChoice = "rapidr"
  CheckCmd "view.theme.light", ThemeChoice = "rapidr light"
  CheckCmd "view.theme.dark", ThemeChoice = "rapidr dark"
  CheckCmd "view.theme.highcontrast", ThemeChoice = "rapidr high contrast"
  CheckCmd "view.theme.classic", ThemeChoice = "classic"
  CheckCmd "view.documents.mdi", DocumentMode = "mdi"
  CheckCmd "view.documents.tabs", DocumentMode = "tabs"
END SUB

SUB MenuPicked(Sender AS QMENUITEM)
  RunCommand Cmd(Sender.Tag).Id
END SUB

SUB ToolPicked(Sender AS QCOOLBTN)
  RunCommand Cmd(Sender.Tag).Id
END SUB

' ---- the command palette ---------------------------------------------------------

DIM PaletteCmd(MAXCMDS + 20) AS INTEGER
DIM PaletteCount AS INTEGER

SUB FillPalette
  DIM i AS INTEGER, f AS STRING, txt AS STRING, cat AS STRING
  f = LCASE$(PaletteEdit.Text)
  PaletteList.Clear
  PaletteCount = 0
  ' (":N": a line of the active document)
  IF LEFT$(f, 1) = ":" THEN
    IF ActiveCode >= 0 THEN
      PaletteList.AddItems "Go to line " + MID$(f, 2) + " (1 to " + LTRIM$(STR$(CodeDoc(ActiveCode).LineCount)) + ")"
      PaletteCmd(0) = -10000
      PaletteCount = 1
      PaletteList.ItemIndex = 0
    END IF
    EXIT SUB
  END IF
  FOR i = 0 TO CmdCount - 1
    IF Cmd(i).Id <> "" AND CmdEnabled(i) <> 0 THEN
      cat = Cmd(i).Category
      IF INSTR(cat, "/") > 0 THEN cat = LEFT$(cat, INSTR(cat, "/") - 1)
      txt = cat + ": " + PlainTitle(Cmd(i).Title)
      IF f = "" OR INSTR(LCASE$(txt), f) > 0 THEN
        IF CmdKeys(i) <> "" THEN txt = txt + "    " + CmdKeys(i)
        PaletteList.AddItems txt
        PaletteCmd(PaletteCount) = i
        PaletteCount = PaletteCount + 1
      END IF
    END IF
  NEXT
  ' (the examples: "Example: hello_form.rr …", opened by a negative entry)
  FOR i = 0 TO ExampleCount - 1
    txt = "Example: " + FileTitle(ExampleFile(i)) + " - " + ExampleWhat(i)
    IF f = "" OR INSTR(LCASE$(txt), f) > 0 THEN
      PaletteList.AddItems txt
      PaletteCmd(PaletteCount) = -1 - i
      PaletteCount = PaletteCount + 1
    END IF
  NEXT
  IF PaletteCount > 0 THEN PaletteList.ItemIndex = 0
END SUB

SUB ShowPalette
  PaletteEdit.Text = ""
  FillPalette
  Palette.Left = Studio.Left + (Studio.Width - Palette.Width) / 2
  Palette.Top = Studio.Top + 80
  Palette.Show
  PaletteEdit.SetFocus
END SUB

SUB PaletteChanged
  FillPalette
END SUB

SUB PalettePicked
  DIM k AS INTEGER
  k = PaletteList.ItemIndex
  IF k < 0 OR k >= PaletteCount THEN EXIT SUB
  Palette.Hide
  IF PaletteCmd(k) = -10000 THEN
    IF VAL(MID$(PaletteEdit.Text, 2)) > 0 AND ActiveCode >= 0 THEN CodeDoc(ActiveCode).GotoLine VAL(MID$(PaletteEdit.Text, 2)) - 1
  ELSEIF PaletteCmd(k) < 0 THEN
    OpenProject HomeDir + "examples/" + ExampleFile(-1 - PaletteCmd(k))
  ELSE
    RunCommand Cmd(PaletteCmd(k)).Id
  END IF
END SUB

' (the palette's keys: Up / Down move in the list, Enter runs, Escape closes)
SUB PaletteKey(Key AS INTEGER, Shift AS INTEGER)
  SELECT CASE Key
    CASE 27
      Palette.Hide
    CASE 13
      PalettePicked
    CASE 38
      IF PaletteList.ItemIndex > 0 THEN PaletteList.ItemIndex = PaletteList.ItemIndex - 1
    CASE 40
      IF PaletteList.ItemIndex < PaletteCount - 1 THEN PaletteList.ItemIndex = PaletteList.ItemIndex + 1
  END SELECT
END SUB

SUB PaletteListKey(Key AS INTEGER, Shift AS INTEGER)
  PaletteKey Key, Shift
END SUB
