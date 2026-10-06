' shell.inc: where every command goes (RunCommand), the window's and the
' panes' events, and start-up.

' The active document's code editor (-1: none).
FUNCTION ActiveCode AS INTEGER
  Result = -1
  IF ActiveDoc >= 0 THEN
    IF DocOpen(ActiveDoc) THEN Result = ActiveDoc
  END IF
END FUNCTION

' The outline and properties of the active document again.
SUB ScanActive
  IF ActiveCode >= 0 THEN
    ScanSource ActiveCode
    ShowProperties FormName
  ELSE
    OutlineTree.Clear
    PropCount = 0
    ShowProperties ""
  END IF
END SUB

SUB ShowAbout
  SHOWMESSAGE "RapidR Studio " + "(RapidR 2.117.0)" + CHR$(13) + CHR$(10) + "RapidQ programs, made modern: desktop and web from one source." + CHR$(13) + CHR$(10) + "Copyright (c) 2025-2026 Ruta Internet SRL. MIT licence."
END SUB

SUB RunCommand(Id AS STRING)
  DIM d AS INTEGER
  d = ActiveCode
  SELECT CASE Id
    ' ---- view ----
    CASE "view.commandPalette": ShowPalette
    CASE "view.project": Dock.FocusPane("projecttree")
    CASE "view.toolbox": Dock.FocusPane("toolboxtree")
    CASE "view.properties": Dock.FocusPane("propgrid")
    CASE "view.outline": Dock.FocusPane("outlinetree")
    CASE "view.output": Dock.FocusPane("outputbox")
    CASE "view.problems": Dock.FocusPane("problemsview")
    CASE "view.immediate": Dock.FocusPane("immediatebox")
    CASE "view.welcome": Dock.FocusPane("welcome")
    CASE "view.theme.rapidr": SetTheme "rapidr"
    CASE "view.theme.light": SetTheme "rapidr light"
    CASE "view.theme.dark": SetTheme "rapidr dark"
    CASE "view.theme.highcontrast": SetTheme "rapidr high contrast"
    CASE "view.theme.classic": SetTheme "classic"
    CASE "view.documents.mdi"
      DocumentMode = "mdi": Dock.DocumentMode = "mdi": UpdateCommands
    CASE "view.documents.tabs"
      DocumentMode = "tabs": Dock.DocumentMode = "tabs": UpdateCommands
    CASE "view.layout": Dock.ResetLayout
    ' ---- window ----
    CASE "window.cascade": Dock.Cascade
    CASE "window.tileHorizontally": Dock.TileHorizontal
    CASE "window.tileVertically": Dock.TileVertical
    CASE "window.next": Dock.NextDocument
    CASE "window.previous": Dock.PreviousDocument
    ' ---- edit: on the active document's editor ----
    CASE "edit.cut": IF d >= 0 THEN CodeDoc(d).CutToClipboard
    CASE "edit.copy": IF d >= 0 THEN CodeDoc(d).CopyToClipboard
    CASE "edit.paste": IF d >= 0 THEN CodeDoc(d).PasteFromClipboard
    CASE "edit.selectAll": IF d >= 0 THEN CodeDoc(d).SelectAll
    ' ---- help ----
    CASE "help.about": ShowAbout
    CASE "help.examples": Dock.FocusPane("welcome")
    CASE "file.openExample": Dock.FocusPane("welcome")
    CASE "file.exit": Studio.Close
    CASE ELSE
      OutputLine "(" + Id + ": not there yet)"
  END SELECT
END SUB

' ---- events ----------------------------------------------------------------------

SUB StudioResize
  LayOutStatus
END SUB

SUB StudioClose(Action AS INTEGER)
END SUB

SUB PaneChanged(Name AS STRING)
END SUB

SUB DocumentActivated(Name AS STRING)
  DIM d AS INTEGER, k AS STRING
  k = LCASE$(Name)
  ActiveDoc = -1
  FOR d = 0 TO DocCount - 1
    IF DocOpen(d) THEN
      IF k = "codedoc(" + LTRIM$(STR$(d)) + ")" THEN ActiveDoc = d: DocKind(d) = "code"
      IF k = "designdoc(" + LTRIM$(STR$(d)) + ")" THEN ActiveDoc = d: DocKind(d) = "design"
    END IF
  NEXT
  UpdateCommands
END SUB

SUB DocumentClosing(Name AS STRING, Cancel AS INTEGER)
END SUB

SUB WelcomeLink(Sender AS QLABEL)
  SELECT CASE Sender.Caption
    CASE "New Project...": RunCommand "file.newProject"
    CASE "Open Project or File...": RunCommand "file.open"
    CASE "Open an Example...": ExamplesView.SetFocus
    CASE ELSE: RunCommand "view.commandPalette"
  END SELECT
END SUB

SUB ExampleOpened
  DIM i AS INTEGER
  i = ExamplesView.ItemIndex
  IF i < 0 OR i >= ExampleCount THEN EXIT SUB
  OpenSource HomeDir + "examples/" + ExampleFile(i)
END SUB

SUB ProjectTreeOpen
END SUB

SUB DesignSelected(Index AS INTEGER)
END SUB

SUB OutlineOpen
  DIM i AS INTEGER, d AS INTEGER
  i = OutlineTree.ItemIndex
  d = ActiveCode
  IF i < 0 OR i >= OutlineCount OR d < 0 THEN EXIT SUB
  CodeDoc(d).GotoLine OutlineLine(i)
END SUB

SUB ProblemOpen
END SUB

' ---- start-up ----------------------------------------------------------------------
' Arguments: [--home <RapidR's folder>] [--theme <name>] [file]: `rapidr ide [file]` on the
' desktop, the web page's (RAPIDR_ARGS) in the browser.

SUB StartStudio
  DIM i AS INTEGER, a AS STRING, startFile AS STRING
  HomeDir = ""
  i = 1
  WHILE i <= COMMANDCOUNT
    a = COMMAND$(i)
    IF a = "--home" THEN
      i = i + 1
      HomeDir = COMMAND$(i)
      IF HomeDir <> "" AND RIGHT$(HomeDir, 1) <> "/" THEN HomeDir = HomeDir + "/"
    ELSEIF a = "--theme" THEN
      i = i + 1
      ThemeChoice = COMMAND$(i)
    ELSE
      startFile = a
    END IF
    i = i + 1
  WEND
  ' (RapidR's look by default, following the system)
  IF ThemeChoice = "" THEN ThemeChoice = "rapidr"
  Application.Theme = ThemeChoice
  RunState = "stopped"
  StateText = "Ready"
  DocumentMode = "mdi"
  ActiveDoc = -1
  DefineCommands
  BuildMenus
  BuildToolBar
  SetUpStatus
  SetUpProblems
  SetUpDock
  FillToolbox
  FillExamples
  ApplyChrome
  FillProjectTree
  ShowProperties ""
  OutputLine "RapidR Studio is ready."
  AddProblem 3, "No problems", "", 0
  ' (alone, the start page fills the documents area)
  Dock.DocumentState("welcome", 2)
  IF startFile <> "" THEN OpenSource startFile
  UpdateTitle
  UpdateCommands
END SUB
