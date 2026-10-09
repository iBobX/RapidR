' panels.inc: RapidR Studio's panels (docs/ide-plan.md I1, L-PANELS /
' S-PANELS) — public components (docs/ide-components.md §3.4–3.8):
'   ToolBar      RTOOLBAR            the command table's tool buttons
'   Palette      RCOMMANDPALETTE     every command, the examples, ":line"
'   ProjectTree  RPROJECTTREE        the project's files, forms' components
'   Toolbox      RTOOLBOX            the registry's components, by group
'   Inspector    RPROPERTYINSPECTOR  the active designer's selection
'   OutputBox    ROUTPUTCONSOLE      the program's output, the build log,
'                                    the problems (the compiler's messages)
'
' The designer document of a file is DesignDoc(d) (S-DESIGN, documents.inc /
' designer.inc); the panels reach it only through its public members:
' AddComponent(Type$, X, Y) (-1, -1: in the selection's container), PlaceType
' (click a tool, then click or draw on the form), DragComponent (a tool
' dragged onto the form), CreateHandler(Name$, Event$) / HandlerLine (an
' event's SUB, written or found), Undo / Redo. The inspector edits the
' designer's model itself (Inspector.Designer = DesignDoc(d)); the designer
' writes each change into the code (OnSourceEdit).

' ---- the tool bar ------------------------------------------------------------

SUB AddTool(Id AS STRING)
  DIM i AS INTEGER, hint AS STRING, icon AS STRING
  i = CmdIndex(Id)
  IF i < 0 THEN EXIT SUB
  hint = PlainTitle(Cmd(i).Title)
  IF CmdKeys(i) <> "" THEN hint = hint + " (" + CmdKeys(i) + ")"
  icon = CmdIcon(i)
  IF icon = "-" THEN icon = ""
  ToolBar.AddButton Id, icon, hint, Id
  Cmd(i).Tool = 1
END SUB

SUB BuildToolBar
  ToolBar.Clear
  AddTool "file.newProject": AddTool "file.open": AddTool "file.save": AddTool "file.saveAll"
  ToolBar.AddSeparator
  AddTool "edit.undo": AddTool "edit.redo"
  ToolBar.AddSeparator
  AddTool "edit.cut": AddTool "edit.copy": AddTool "edit.paste": AddTool "edit.find"
  ToolBar.AddSeparator
  AddTool "run.start": AddTool "run.withoutDebugging": AddTool "run.pause": AddTool "run.stop": AddTool "run.build"
  ' (the web's Build is Build Web App: the desktop's Build row isn't there)
  IF Proj.FileManager = "" THEN AddTool "run.buildWeb"
  ToolBar.AddSeparator
  AddTool "debug.stepInto": AddTool "debug.stepOver": AddTool "debug.stepOut": AddTool "debug.toggleBreakpoint"
  ToolBar.AddSeparator
  AddTool "designer.alignLeft": AddTool "designer.alignTop": AddTool "designer.sameWidth": AddTool "designer.bringToFront"
  ToolBar.AddSeparator
  AddPreviewTool: AddTool "view.commandPalette": AddTool "view.settings"
END SUB

SUB ToolClicked(Name AS STRING, Command AS STRING)
  RunCommand Command
END SUB

' ---- the command palette (Ctrl+Shift+P) ------------------------------------------
' Every command of the table (its category, its keys, its icon; the disabled
' ones dimmed: UpdateCommands), the examples ("Example: …"), and ":N" for a
' line of the active document.

SUB FillPalette
  DIM i AS INTEGER, cat AS STRING, icon AS STRING
  Palette.Clear
  FOR i = 0 TO CmdCount - 1
    IF Cmd(i).Id <> "" THEN
      cat = Cmd(i).Category
      IF INSTR(cat, "/") > 0 THEN cat = LEFT$(cat, INSTR(cat, "/") - 1)
      icon = CmdIcon(i)
      IF icon = "-" THEN icon = ""
      Palette.AddCommand Cmd(i).Id, PlainTitle(Cmd(i).Title), CmdKeys(i), cat, icon
    END IF
  NEXT
  FOR i = 0 TO ExampleCount - 1
    Palette.AddCommand "example:" + LTRIM$(STR$(i)), FileTitle(ExampleFile(i)) + " - " + ExampleWhat(i), "", "Example", "RFORM"
  NEXT
  Palette.AddPrefix ":", "Go to line {}"
END SUB

' One search for everything (docs/studio-wow.md CMD-2): beside the
' commands, the project's files ("Open"), the active file's SUBs, TYPEs and
' globals ("Symbol") and its form's components ("Component"), kept up to
' date as the code changes (InspectActive).
DIM QuickId(800) AS STRING
DIM QuickCount AS INTEGER

SUB AddQuick(Id AS STRING, Title AS STRING, Category AS STRING, Icon AS STRING)
  IF QuickCount > 800 THEN EXIT SUB
  Palette.AddCommand Id, Title, "", Category, Icon
  QuickId(QuickCount) = Id
  QuickCount = QuickCount + 1
END SUB

SUB RefreshQuickOpen
  DIM i AS INTEGER, n AS INTEGER, d AS INTEGER, text AS STRING, row AS STRING, kind AS STRING, nm AS STRING, detail AS STRING
  FOR i = 0 TO QuickCount - 1
    Palette.RemoveCommand QuickId(i)
  NEXT
  QuickCount = 0
  IF ProjectOpen THEN
    FOR i = 0 TO Proj.FileCount - 1
      AddQuick "open:" + Proj.FullPath(i), Proj.File(i), "Open", "files/" + FileExt(Proj.File(i))
    NEXT
  END IF
  d = ActiveCode
  IF d < 0 THEN EXIT SUB
  text = Lang.Outline(DocFile(d))
  n = TALLY(text, CHR$(10))
  FOR i = 1 TO n
    row = FIELD$(text, CHR$(10), i)
    kind = FIELD$(row, CHR$(9), 2)
    nm = FIELD$(row, CHR$(9), 3)
    detail = FIELD$(row, CHR$(9), 4)
    IF kind = "component" THEN
      AddQuick "comp:" + nm, nm + ": " + detail, "Component", UCASE$(MID$(detail, RINSTR(detail, " ") + 1))
    ELSE
      AddQuick "line:" + FIELD$(row, CHR$(9), 5) + ":" + nm, nm, "Symbol", "symbols/" + kind
    END IF
  NEXT
END SUB

' (Filter: what the box starts with — "Example", ":" for a line)
SUB ShowPalette(Filter AS STRING)
  Palette.Show
  IF Filter <> "" THEN Palette.Filter = Filter
END SUB

SUB PaletteCommand(Id AS STRING)
  DIM d AS INTEGER
  IF LEFT$(Id, 8) = "example:" THEN
    OpenProject HomeDir + "examples/" + ExampleFile(VAL(MID$(Id, 9)))
  ELSEIF LEFT$(Id, 5) = "open:" THEN
    OpenDocument MID$(Id, 6)
  ELSEIF LEFT$(Id, 5) = "line:" THEN
    d = ActiveCode
    IF d >= 0 THEN
      Dock.FocusPane("codedoc(" + LTRIM$(STR$(d)) + ")")
      CodeDoc(d).GotoLine VAL(MID$(Id, 6)) - 1
    END IF
  ELSEIF LEFT$(Id, 5) = "comp:" THEN
    d = ActiveDesign
    IF d >= 0 THEN
      Dock.FocusPane("designdoc(" + LTRIM$(STR$(d)) + ")")
      DesignDoc(d).SelectName MID$(Id, 6)
    END IF
  ELSEIF LEFT$(Id, 1) = ":" THEN
    d = ActiveCode
    IF d >= 0 AND VAL(MID$(Id, 2)) > 0 THEN
      Dock.FocusPane("codedoc(" + LTRIM$(STR$(d)) + ")")
      CodeDoc(d).GotoLine VAL(MID$(Id, 2)) - 1
    END IF
  ELSE
    RunCommand Id
  END IF
END SUB

' ---- the project tree -------------------------------------------------------------
' The project's files by kind (forms with the components their CREATE
' blocks make), as RPROJECTTREE reads the project (.rrproj) or the source
' file and what it includes.

SUB FillProjectTree
  IF ProjectOpen = 0 THEN
    ProjectTree.Project = ""
    EXIT SUB
  END IF
  ProjectTree.Project = ProjectFile
END SUB

' (a project path, relative or not, as a file's path)
FUNCTION ProjectPath(Path AS STRING) AS STRING
  DIM p AS STRING
  p = Path
  IF INSTR(p, "#") > 0 THEN p = LEFT$(p, INSTR(p, "#") - 1)
  IF LEFT$(p, 1) = "/" OR MID$(p, 2, 1) = ":" OR Proj.Folder = "" THEN
    Result = p
  ELSE
    Result = Proj.Folder + "/" + p
  END IF
END FUNCTION

' Opened (double click, Enter): a file — its documents; a form's component —
' its form in the designer, that component selected.
SUB ProjectNodeOpen(Path AS STRING)
  DIM d AS INTEGER, comp AS STRING
  IF Path = "" OR RIGHT$(Path, 1) = "/" THEN EXIT SUB
  OpenDocument ProjectPath(Path)
  IF INSTR(Path, "#") = 0 THEN EXIT SUB
  comp = MID$(Path, INSTR(Path, "#") + 1)
  d = DocOf(ProjectPath(Path))
  IF d < 0 THEN EXIT SUB
  IF DocHasForm(d) = 0 THEN EXIT SUB
  Dock.FocusPane("designdoc(" + LTRIM$(STR$(d)) + ")")
  DesignDoc(d).SelectName comp
END SUB

' Selected: a component of the open form is selected in its designer too.
SUB ProjectNodeSelect(Path AS STRING)
  DIM d AS INTEGER
  IF INSTR(Path, "#") = 0 THEN EXIT SUB
  d = DocOf(ProjectPath(Path))
  IF d < 0 THEN EXIT SUB
  IF DocHasForm(d) = 0 THEN EXIT SUB
  DesignDoc(d).SelectName MID$(Path, INSTR(Path, "#") + 1)
END SUB

' Project > Add Form / Add Module: a new file in the tree, its name typed
' in place (Enter: ProjectNewFile writes it, adds it, opens it).
SUB AddProjectFile(Kind AS STRING)
  IF ProjectOpen = 0 THEN EXIT SUB
  Dock.ShowPane("projecttree")
  ProjectTree.NewFile Kind
END SUB

' A new project file (Project > Add Form / Add Module, named in the tree):
' written with its first text — a form in the program's own names
' (`CREATE Form2 AS RForm`), a module — added to the project, brought into
' the program by the main file's `$INCLUDE` (one undo step in its editor:
' RapidQ's way of making a program of several files), and opened: a form on
' its designer.
SUB ProjectNewFile(Path AS STRING, Kind AS STRING)
  DIM full AS STRING, f AS RFileStream, dm AS INTEGER, mainText AS STRING, t AS STRING, patch AS STRING
  full = ProjectPath(Path)
  ' (the main program as its editor has it)
  dm = DocOf(ProjectMain)
  IF dm >= 0 THEN mainText = CodeDoc(dm).Text ELSE mainText = Proj.FileText(ProjectMain)
  IF FILEEXISTS(full) = 0 THEN
    ' (the toolbox's RFormMDI: an MDI main window)
    IF Kind = "form" AND PendingFormKind = "formmdi" THEN t = Proj.NewFileText(Path, "formmdi", mainText) ELSE t = Proj.NewFileText(Path, Kind, mainText)
    f.Open(full, 65535)
    f.WriteStr(t, LEN(t))
    f.Close
  END IF
  IF Proj.AddFile(full, Kind) THEN
    IF Proj.Kind = "project" THEN SaveProjectFile
  ELSE
    OutputLine "Can't add " + Path + ": " + Proj.Error
  END IF
  IF (Kind = "form" OR Kind = "module") AND LCASE$(full) <> LCASE$(ProjectMain) THEN
    patch = Proj.IncludeEdit(mainText, full)
    IF patch <> "" THEN
      IF dm < 0 THEN
        OpenDocument ProjectMain
        dm = DocOf(ProjectMain)
      END IF
      IF dm >= 0 THEN
        CodeDoc(dm).ApplyPatches patch, 0
        ProjectFileChanged dm
        OutputLine FileTitle(ProjectMain) + " includes " + FileTitle(full) + " ($INCLUDE): its forms are made when the program starts"
      END IF
    END IF
  END IF
  OpenDocument full
  IF Kind = "form" THEN
    IF PendingFormHint <> "" THEN
      SetState RunState, REPLACESUBSTR$(PendingFormHint, "{form}", BaseName(full))
      OutputLine REPLACESUBSTR$(PendingFormHint, "{form}", BaseName(full))
    ELSE
      SetState RunState, BaseName(full) + " added: drop components on it; show it with " + BaseName(full) + ".Show"
    END IF
  END IF
  PendingFormKind = ""
  PendingFormHint = ""
END SUB

' Renamed in place (F2): the file on the disk, the project, its documents.
SUB ProjectRename(OldPath AS STRING, NewPath AS STRING, Cancel AS INTEGER)
  DIM o AS STRING, n AS STRING, d AS INTEGER
  o = ProjectPath(OldPath): n = ProjectPath(NewPath)
  IF FILEEXISTS(n) THEN
    SetState "error", FileTitle(n) + " is there already"
    Cancel = 1
    EXIT SUB
  END IF
  d = DocOf(o)
  IF d >= 0 THEN
    IF DocDirty(d) THEN SaveDocument d
  END IF
  IF FILEEXISTS(o) THEN RENAME o, n
  Proj.RemoveFile o
  Proj.AddFile n
  SaveProjectFile
  IF d >= 0 THEN
    DocFile(d) = n
    DocTitles d
  END IF
  IF LCASE$(o) = LCASE$(ProjectMain) THEN
    ProjectMain = n
  ELSE
    IncludeFollows o, n
  END IF
END SUB

' (a file of the program renamed or moved: the main file's $INCLUDE names
' it again — one undo step in its editor)
SUB IncludeFollows(OldPath AS STRING, NewPath AS STRING)
  DIM dm AS INTEGER, mainText AS STRING, patch AS STRING
  dm = DocOf(ProjectMain)
  IF dm >= 0 THEN mainText = CodeDoc(dm).Text ELSE mainText = Proj.FileText(ProjectMain)
  patch = Proj.IncludeRenameEdit(mainText, OldPath, NewPath)
  IF patch = "" THEN EXIT SUB
  IF dm < 0 THEN
    OpenDocument ProjectMain
    dm = DocOf(ProjectMain)
  END IF
  IF dm < 0 THEN EXIT SUB
  CodeDoc(dm).ApplyPatches patch, 0
  ProjectFileChanged dm
  OutputLine FileTitle(ProjectMain) + " includes " + FileTitle(NewPath) + " now"
END SUB

' Taken out of the project (confirmed in the tree): its documents close; the
' file stays on the disk.
SUB ProjectDelete(Path AS STRING, Cancel AS INTEGER)
  DIM p AS STRING
  p = ProjectPath(Path)
  IF LCASE$(p) = LCASE$(ProjectMain) THEN
    SetState "error", "The main file stays in the project"
    Cancel = 1
    EXIT SUB
  END IF
  CloseDocument DocOf(p)
  Proj.RemoveFile p
  SaveProjectFile
  IncludeLeaves p
  SetState RunState, FileTitle(p) + " taken out of the project (still on the disk)"
END SUB

' (a file taken out of the project: the main file no longer includes it —
' one undo step in its editor)
SUB IncludeLeaves(Path AS STRING)
  DIM dm AS INTEGER, patch AS STRING
  dm = DocOf(ProjectMain)
  IF dm < 0 THEN
    IF Proj.IncludeRemoveEdit(Proj.FileText(ProjectMain), Path) = "" THEN EXIT SUB
    OpenDocument ProjectMain
    dm = DocOf(ProjectMain)
  END IF
  IF dm < 0 THEN EXIT SUB
  patch = Proj.IncludeRemoveEdit(CodeDoc(dm).Text, Path)
  IF patch = "" THEN EXIT SUB
  CodeDoc(dm).ApplyPatches patch, 0
  ProjectFileChanged dm
  OutputLine FileTitle(ProjectMain) + " no longer includes " + FileTitle(Path)
END SUB

' Dragged to another place (or folder): the project keeps the tree's order.
SUB ProjectMove(Path AS STRING, NewPath AS STRING, Index AS INTEGER)
  IF Proj.Kind <> "project" THEN EXIT SUB
  IF ProjectTree.Save THEN
    IF Proj.Open(ProjectFile) = 0 THEN OutputLine "Can't read the project again: " + Proj.Error
  ELSE
    OutputLine "Can't save the project's order"
  END IF
END SUB

' (the file's text as the editor has it: its components follow the code)
SUB ProjectFileChanged(d AS INTEGER)
  IF ProjectOpen = 0 OR d < 0 THEN EXIT SUB
  ProjectTree.SetFileText DocFile(d), CodeDoc(d).Text
  DesignNames d
END SUB

' (the names the program's other files give their components: document d's
' designer never names a new one so — a RapidQ program's names are global)
SUB DesignNames(d AS INTEGER)
  IF ProjectOpen = 0 OR d < 0 THEN EXIT SUB
  IF DocOpen(d) = 0 THEN EXIT SUB
  DesignDoc(d).ReservedNames = ProjectTree.ComponentNames(DocFile(d))
END SUB

' ---- the toolbox ----------------------------------------------------------------
' The components under RapidR's names, grouped by purpose, with RapidR's
' icons, found by typing; each adds itself to the active form (written in
' that file's own style: the designer).

' (ActiveDesign, the active document's designer: designer.inc)

' ---- a form from the toolbox: a document, not a component (Delphi, Xojo) --
' RForm (or RFormMDI) clicked, double-clicked or dragged adds a form FILE
' to the program — Project > Add Form — never a form inside a form.

FUNCTION IsFormType(TypeName AS STRING) AS INTEGER
  DIM t AS STRING
  t = UCASE$(TypeName)
  Result = Flag(t = "RFORM" OR t = "QFORM" OR t = "RFORMMDI" OR t = "QFORMMDI")
END FUNCTION

' (Hint: what the status bar says once it is added — "" the usual)
SUB AddFormOfType(TypeName AS STRING, Hint AS STRING)
  IF ProjectOpen = 0 THEN
    SetState RunState, "Open or make a project first (File > New Project)"
    EXIT SUB
  END IF
  ' (a click then a double click: one form)
  IF PendingFormKind <> "" AND ABS(TIMER - PendingFormAt) < 1.5 THEN EXIT SUB
  IF UCASE$(TypeName) = "RFORMMDI" OR UCASE$(TypeName) = "QFORMMDI" THEN PendingFormKind = "formmdi" ELSE PendingFormKind = "form"
  PendingFormHint = Hint
  PendingFormAt = TIMER
  AddProjectFile "form"
END SUB

' Double click or Enter: added in the selected container, at a free place.
SUB ToolboxPick(TypeName AS STRING)
  DIM d AS INTEGER
  IF IsFormType(TypeName) THEN
    AddFormOfType TypeName, ""
    EXIT SUB
  END IF
  d = ActiveDesign
  IF d < 0 THEN
    SetState "stopped", "Open a form to add a " + TypeName
    EXIT SUB
  END IF
  DesignDoc(d).AddComponent TypeName, -1, -1
  Dock.FocusPane("designdoc(" + LTRIM$(STR$(d)) + ")")
END SUB

' A click: the designer places one where the form is clicked (or drawn).
SUB ToolboxSelect(TypeName AS STRING)
  DIM d AS INTEGER
  IF IsFormType(TypeName) THEN
    AddFormOfType TypeName, ""
    EXIT SUB
  END IF
  d = ActiveDesign
  IF d >= 0 THEN DesignDoc(d).PlaceType = TypeName
END SUB

' A drag out of the toolbox: the designer follows it (its guides) and drops it.
SUB ToolboxDragStart(TypeName AS STRING)
  DIM d AS INTEGER
  ' (a form isn't dropped into the designer: ToolboxDrop adds its file)
  IF IsFormType(TypeName) THEN EXIT SUB
  d = ActiveDesign
  IF d >= 0 THEN DesignDoc(d).DragComponent TypeName
END SUB

' (let go over a designer that didn't take the drag itself: added there)
SUB ToolboxDrop(TypeName AS STRING, Target AS STRING, X AS INTEGER, Y AS INTEGER)
  DIM d AS INTEGER, ft AS STRING
  d = ActiveDesign
  IF IsFormType(TypeName) THEN
    ' (let go on a designed form: a new window all the same, and why)
    IF d >= 0 THEN
      IF LCASE$(Target) = "designdoc(" + LTRIM$(STR$(d)) + ")" THEN
        ft = UCASE$(DesignDoc(d).FormType)
        IF ft = "RFORMMDI" OR ft = "QFORMMDI" THEN
          ' (a child window, RapidQ's way: a panel on the MDI form, AddChild)
          ft = DesignDoc(d).AddMdiChild()
          IF ft <> "" THEN
            SetState RunState, DesignDoc(d).StatusText
            OutputLine DesignDoc(d).StatusText
          END IF
        ELSE
          AddFormOfType TypeName, "A form can't go inside a form: {form} was added as a new window. For child windows, make " + DesignDoc(d).FormName + " an RFormMDI."
        END IF
        EXIT SUB
      END IF
    END IF
    AddFormOfType TypeName, ""
    EXIT SUB
  END IF
  IF d < 0 THEN EXIT SUB
  IF LCASE$(Target) = "designdoc(" + LTRIM$(STR$(d)) + ")" THEN DesignDoc(d).AddComponent TypeName, X, Y
END SUB

' ---- the property inspector --------------------------------------------------------
' It follows the active form's designer: what is selected there (several
' components: what they share), values as the CREATE block writes them,
' the registry's docs; a change is one undo step in the designer, which
' writes it into the code.

SUB InspectActive
  DIM d AS INTEGER
  RefreshQuickOpen
  d = ActiveDesign
  IF d >= 0 THEN
    Inspector.Designer = DesignDoc(d)
    Dock.PaneTitle("inspector", "Properties")
  ELSE
    Inspector.Designer = ""
    Inspector.Target = ""
    Dock.PaneTitle("inspector", "Properties")
  END IF
END SUB

SUB InspectorChanged(Prop AS STRING, Value AS STRING)
  DIM d AS INTEGER
  d = ActiveDesign
  IF d < 0 THEN EXIT SUB
  SetState RunState, Inspector.Target + "." + Prop + " = " + Value
END SUB

' An event's row double-clicked: its SUB — found, or written with the
' event's parameters and bound — and the caret inside it.
SUB InspectorEvent(EventName AS STRING)
  DIM d AS INTEGER, comp AS STRING, sb AS STRING
  d = ActiveDesign
  IF d < 0 THEN EXIT SUB
  comp = Inspector.Target
  IF comp = "" OR INSTR(comp, ",") > 0 THEN EXIT SUB
  sb = DesignDoc(d).CreateHandler(comp, EventName)
  IF sb = "" THEN
    OutputLine "Designer: " + DesignDoc(d).StatusText
    EXIT SUB
  END IF
  Dock.FocusPane("codedoc(" + LTRIM$(STR$(d)) + ")")
  IF DesignDoc(d).HandlerLine >= 0 THEN CaretAtHandler d, DesignDoc(d).HandlerLine
END SUB

' (the caret at the end of line L of document d's code — a new handler's
' indented body line: typing starts there)
SUB CaretAtHandler(d AS INTEGER, L AS INTEGER)
  CodeDoc(d).GotoLine L
  CodeDoc(d).GotoLineColumn L + 1, LEN(CodeDoc(d).Line(L)) + 1
END SUB

' ---- the output console --------------------------------------------------------------

SUB OutputLine(Text AS STRING)
  OutputBox.WriteLine Text
END SUB

SUB ClearProblems
  OutputBox.ClearProblems
END SUB

' (Kind: 0 error, 1 warning, 2 information)
SUB AddProblem(Kind AS INTEGER, Message AS STRING, InFile AS STRING, LineNo AS INTEGER, ColNo AS INTEGER)
  DIM sev AS STRING
  sev = "info"
  IF Kind = 0 THEN sev = "error"
  IF Kind = 1 THEN sev = "warning"
  OutputBox.AddProblem InFile, LineNo, ColNo, sev, Message
END SUB

' A `file:line` place or a problem clicked: that line of that file. (The
' build log's links — "Reveal in Finder", the app's path — are reveal:<path>:
' shown in the file manager.)
SUB OutputLink(InFile AS STRING, LineNo AS INTEGER)
  DIM d AS INTEGER, p AS STRING
  IF LEFT$(InFile, 7) = "reveal:" THEN
    IF Proj.Reveal(MID$(InFile, 8)) = 0 THEN OutputLine "Can't show it: " + Proj.Error
    EXIT SUB
  END IF
  p = InFile
  d = DocOf(p)
  IF d < 0 THEN
    p = ProjectPath(InFile)
    d = DocOf(p)
  END IF
  IF d < 0 THEN
    IF FILEEXISTS(p) THEN OpenDocument p
    d = DocOf(p)
  END IF
  IF d < 0 THEN d = ActiveCode
  IF d < 0 THEN EXIT SUB
  Dock.FocusPane("codedoc(" + LTRIM$(STR$(d)) + ")")
  IF LineNo > 0 THEN CodeDoc(d).GotoLine LineNo - 1
END SUB

' ---- the look ----------------------------------------------------------------------------

' (what each panel says while it has nothing to show)
SUB SetUpPanels
  ProjectTree.EmptyText = "No project open. File > New Project (Ctrl+N), or Open (Ctrl+O)."
  Inspector.EmptyText = "Select a component on a form to see its properties."
  OutputBox.EmptyText = "Press F5 to run the program; its output shows here."
  Toolbox.Hint = "Double-click or press Enter to add a component to the form; drag it to place it."
  ' (RapidR's names: RButton, RLabel …; the designer writes each file in
  ' its own — a form written with RapidQ's names gets QBUTTON)
  Toolbox.ShowNames = "rapidr"
END SUB

SUB ApplyPanelFonts
  ProjectTree.Font = UIFont
  Toolbox.Font = UIFont
  Inspector.Font = UIFont
  OutputBox.Font = MonoFont
  ToolBar.Font = UIFont
  Palette.Font = UIFont
END SUB

' ---- test steps (tests/studio_wow.mjs; the --do list, beside the commands) ----
'   tool:<TYPE>          the toolbox's Enter on that component
'   prop:<Prop>=<value>  the inspector's row set as a user sets it
'   page:<properties|events>, filter:<text>  the inspector's page, its search
'   reset:<Prop>         the inspector's row put back to its default
'   event:<OnX>          an event's row double-clicked
'   focus:<pane>         Dock.FocusPane
'   search:<text>        the toolbox's search box
'   palette:<text>       the command palette, opened with that typed
'   pick:<Name>          a component selected in the active designer
'   newproject:<template>|<folder>|<name>   File > New Project's Create
'   rename:<file>|<new name>   F2 in the project tree, the name typed
'   remove:<file>        Delete in the project tree (it asks first)
'   drop:<TYPE>|<target> a toolbox item let go over a pane
' True when Stp was one of these.
FUNCTION PanelStep(Stp AS STRING) AS INTEGER
  DIM k AS INTEGER, verb AS STRING, arg AS STRING
  k = INSTR(Stp, ":")
  IF k < 2 THEN EXIT FUNCTION
  verb = LEFT$(Stp, k - 1)
  arg = MID$(Stp, k + 1)
  Result = 1
  SELECT CASE verb
    CASE "tool"
      ' (selected in the toolbox, which names it as it shows it: RButton)
      Toolbox.Selected = arg
      IF Toolbox.Selected <> "" THEN ToolboxPick Toolbox.Selected ELSE ToolboxPick UCASE$(arg)
    CASE "prop"
      k = INSTR(arg, "=")
      IF k > 1 THEN
        IF Inspector.SetValue(LEFT$(arg, k - 1), MID$(arg, k + 1)) = 0 THEN OutputLine "(" + Stp + ": refused)"
      END IF
    CASE "page": Inspector.Page = arg
    CASE "reset": Inspector.ResetValue arg
    CASE "filter": Inspector.Filter = arg
    CASE "event": InspectorEvent arg
    CASE "focus": Dock.FocusPane(arg)
    CASE "search": Toolbox.Filter = arg
    CASE "code"
      ' (code:<old>=><new>: the first <old> in the active code replaced, as
      ' typed over a selection)
      k = INSTR(arg, "=>")
      IF k > 1 AND ActiveCode >= 0 THEN
        verb = LEFT$(arg, k - 1)
        IF INSTR(CodeDoc(ActiveCode).Text, verb) > 0 THEN
          CodeDoc(ActiveCode).SelStart = INSTR(CodeDoc(ActiveCode).Text, verb) - 1
          CodeDoc(ActiveCode).SelLength = LEN(verb)
          CodeDoc(ActiveCode).SelText = MID$(arg, k + 2)
          ' (as the editor's OnChange after a user's typing)
          CodeChanged CodeDoc(ActiveCode)
        END IF
      END IF
    CASE "pick"
      ' (a component selected in the active designer, by its name)
      IF ActiveDesign >= 0 THEN DesignDoc(ActiveDesign).SelectName arg
    CASE "palette": ShowPalette arg
    CASE "drop"
      ' (drop:<TYPE>|<target>: a toolbox item let go over a pane — designdoc(0) …)
      ToolboxDrop UCASE$(FIELD$(arg, "|", 1)), FIELD$(arg, "|", 2), 40, 40
    CASE "remove"
      ' (remove:<file>: Delete in the project tree — it asks first)
      ProjectTree.Delete arg
    CASE "rename"
      ' (rename:<file>|<new name>: F2 in the project tree, the name typed)
      ProjectTree.Rename FIELD$(arg, "|", 1), FIELD$(arg, "|", 2)
    CASE "newproject"
      ' (newproject:<template>|<folder>|<name>: File > New Project's Create)
      NewName.Text = FIELD$(arg, "|", 3)
      NewFolder.Text = FIELD$(arg, "|", 2)
      FOR k = 0 TO TPLS - 1
        IF TplId(k) = FIELD$(arg, "|", 1) THEN PickTemplate k
      NEXT
      NewCreate
    CASE ELSE: Result = 0
  END SELECT
END FUNCTION
