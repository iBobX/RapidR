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
  ToolBar.AddSeparator
  AddTool "debug.stepInto": AddTool "debug.stepOver": AddTool "debug.stepOut": AddTool "debug.toggleBreakpoint"
  ToolBar.AddSeparator
  AddTool "designer.alignLeft": AddTool "designer.alignTop": AddTool "designer.sameWidth": AddTool "designer.bringToFront"
  ToolBar.AddSeparator
  AddTool "view.commandPalette": AddTool "view.settings"
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

' (Filter: what the box starts with — "Example", ":" for a line)
SUB ShowPalette(Filter AS STRING)
  Palette.Show
  IF Filter <> "" THEN Palette.Filter = Filter
END SUB

SUB PaletteCommand(Id AS STRING)
  DIM d AS INTEGER
  IF LEFT$(Id, 8) = "example:" THEN
    OpenProject HomeDir + "examples/" + ExampleFile(VAL(MID$(Id, 9)))
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

' (the designer component of document d named Name: its index, -1 if none)
FUNCTION DesignIndex(d AS INTEGER, Name AS STRING) AS INTEGER
  DIM i AS INTEGER
  Result = -1
  FOR i = 0 TO DesignDoc(d).Count - 1
    IF LCASE$(DesignDoc(d).GetName(i)) = LCASE$(Name) THEN
      Result = i
      EXIT FUNCTION
    END IF
  NEXT
END FUNCTION

' Opened (double click, Enter): a file — its documents; a form's component —
' its form in the designer, that component selected.
SUB ProjectNodeOpen(Path AS STRING)
  DIM d AS INTEGER, comp AS STRING, k AS INTEGER
  IF Path = "" OR RIGHT$(Path, 1) = "/" THEN EXIT SUB
  OpenDocument ProjectPath(Path)
  IF INSTR(Path, "#") = 0 THEN EXIT SUB
  comp = MID$(Path, INSTR(Path, "#") + 1)
  d = DocOf(ProjectPath(Path))
  IF d < 0 THEN EXIT SUB
  IF DocHasForm(d) = 0 THEN EXIT SUB
  Dock.FocusPane("designdoc(" + LTRIM$(STR$(d)) + ")")
  k = DesignIndex(d, comp)
  IF k >= 0 THEN DesignDoc(d).SelectComp k
END SUB

' Selected: a component of the open form is selected in its designer too.
SUB ProjectNodeSelect(Path AS STRING)
  DIM d AS INTEGER, k AS INTEGER
  IF INSTR(Path, "#") = 0 THEN EXIT SUB
  d = DocOf(ProjectPath(Path))
  IF d < 0 THEN EXIT SUB
  IF DocHasForm(d) = 0 THEN EXIT SUB
  k = DesignIndex(d, MID$(Path, INSTR(Path, "#") + 1))
  IF k >= 0 THEN DesignDoc(d).SelectComp k
END SUB

' (the file's text as the editor has it: its components follow the code)
SUB ProjectFileChanged(d AS INTEGER)
  IF ProjectOpen = 0 OR d < 0 THEN EXIT SUB
  ProjectTree.SetFileText DocFile(d), CodeDoc(d).Text
END SUB

' ---- the toolbox ----------------------------------------------------------------
' RapidQ's and RapidR's components as the registry groups them, with
' RapidR's icons, found by typing; each adds itself to the active form.

' (the document whose form the designer shows now, -1 if none)
FUNCTION ActiveDesign AS INTEGER
  DIM d AS INTEGER
  Result = -1
  d = ActiveCode
  IF d >= 0 THEN
    IF DocHasForm(d) THEN Result = d
  END IF
END FUNCTION

' Double click or Enter: added in the selected container, at a free place.
SUB ToolboxPick(TypeName AS STRING)
  DIM d AS INTEGER
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
  d = ActiveDesign
  IF d >= 0 THEN DesignDoc(d).PlaceType = TypeName
END SUB

' A drag out of the toolbox: the designer follows it (its guides) and drops it.
SUB ToolboxDragStart(TypeName AS STRING)
  DIM d AS INTEGER
  d = ActiveDesign
  IF d >= 0 THEN DesignDoc(d).DragComponent TypeName
END SUB

' (let go over a designer that didn't take the drag itself: added there)
SUB ToolboxDrop(TypeName AS STRING, Target AS STRING, X AS INTEGER, Y AS INTEGER)
  DIM d AS INTEGER
  d = ActiveDesign
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
  IF sb = "" THEN EXIT SUB
  Dock.FocusPane("codedoc(" + LTRIM$(STR$(d)) + ")")
  CodeDoc(d).GotoLine DesignDoc(d).HandlerLine
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

' A `file:line` place or a problem clicked: that line of that file.
SUB OutputLink(InFile AS STRING, LineNo AS INTEGER)
  DIM d AS INTEGER, p AS STRING
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
'   event:<OnX>          an event's row double-clicked
'   focus:<pane>         Dock.FocusPane
'   search:<text>        the toolbox's search box
'   palette:<text>       the command palette, opened with that typed
' True when Stp was one of these.
FUNCTION PanelStep(Stp AS STRING) AS INTEGER
  DIM k AS INTEGER, verb AS STRING, arg AS STRING
  k = INSTR(Stp, ":")
  IF k < 2 THEN EXIT FUNCTION
  verb = LEFT$(Stp, k - 1)
  arg = MID$(Stp, k + 1)
  Result = 1
  SELECT CASE verb
    CASE "tool": ToolboxPick UCASE$(arg)
    CASE "prop"
      k = INSTR(arg, "=")
      IF k > 1 THEN
        IF Inspector.SetValue(LEFT$(arg, k - 1), MID$(arg, k + 1)) = 0 THEN OutputLine "(" + Stp + ": refused)"
      END IF
    CASE "page": Inspector.Page = arg
    CASE "filter": Inspector.Filter = arg
    CASE "event": InspectorEvent arg
    CASE "focus": Dock.FocusPane(arg)
    CASE "search": Toolbox.Filter = arg
    CASE "palette": ShowPalette arg
    CASE ELSE: Result = 0
  END SELECT
END FUNCTION
