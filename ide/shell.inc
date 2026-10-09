' shell.inc: where every command goes (RunCommand), the window's and the
' panes' events, and start-up.

' The active document's code editor (-1: none).
FUNCTION ActiveCode AS INTEGER
  Result = -1
  IF ActiveDoc >= 0 THEN
    IF DocOpen(ActiveDoc) THEN Result = ActiveDoc
  END IF
END FUNCTION

SUB ShowAbout
  SHOWMESSAGE "RapidR Studio (RapidR 2.117.0)" + CHR$(13) + CHR$(10) + "RapidQ programs, made modern: desktop and web from one source." + CHR$(13) + CHR$(10) + CHR$(13) + CHR$(10) + "Copyright (c) 2025-2026 Ruta Internet SRL. MIT licence."
END SUB

SUB ShowShortcuts
  DIM i AS INTEGER, t AS STRING
  t = ""
  FOR i = 0 TO CmdCount - 1
    IF Cmd(i).Id <> "" AND CmdKeys(i) <> "" THEN t = t + CmdKeys(i) + "  " + PlainTitle(Cmd(i).Title) + CHR$(13) + CHR$(10)
  NEXT
  SHOWMESSAGE t
END SUB

' (the palette's line mode: ":42" goes to line 42)
SUB GoToLine
  IF ActiveCode < 0 THEN EXIT SUB
  ShowPalette ":"
END SUB

SUB SaveDocumentAs
  DIM d AS INTEGER
  d = ActiveCode
  IF d < 0 THEN EXIT SUB
  SaveDlg.Caption = "Save As"
  SaveDlg.FileName = FileTitle(DocFile(d))
  SaveDlg.Filter = "RapidR sources (*.rr;*.bas;*.inc)|*.rr;*.bas;*.inc|All files|*.*"
  IF SaveDlg.Execute THEN
    DocFile(d) = SaveDlg.FileName
    SaveDocument d
    Proj.AddFile SaveDlg.FileName
    FillProjectTree
  END IF
END SUB

SUB AddFileToProject
  OpenDlg.Caption = "Add a File to the Project"
  OpenDlg.Filter = "All files|*.*"
  IF OpenDlg.Execute THEN
    IF Proj.AddFile(OpenDlg.FileName) THEN FillProjectTree ELSE OutputLine Proj.Error
  END IF
END SUB

SUB RunCommand(Id AS STRING)
  DIM d AS INTEGER
  d = ActiveCode
  ' (the designer's commands, and Edit's on a designer: designer.inc)
  IF DesignCommand(Id) THEN EXIT SUB
  ' (the documents' views and groups, Find in Files, Help: workspace.inc, search.inc)
  IF WorkspaceCommand(Id) THEN EXIT SUB
  SELECT CASE Id
    ' ---- file ----
    CASE "file.newProject": AskNew
    CASE "file.open": AskOpen
    CASE "file.openFolder": AskOpenFolder
    CASE "file.openExample": ShowPalette "Example"
    CASE "file.save": SaveDocument d
    CASE "file.saveAs": SaveDocumentAs
    CASE "file.saveAll": SaveAll: SaveProjectFile
    CASE "file.close": CloseDocument d
    CASE "file.closeAll": CloseAllDocuments
    CASE "file.closeProject": CloseProject
    CASE "file.exit": Studio.Close
    ' ---- edit: on the active document's editor ----
    CASE "edit.cut": IF d >= 0 THEN CodeDoc(d).CutToClipboard
    CASE "edit.copy": IF d >= 0 THEN CodeDoc(d).CopyToClipboard
    CASE "edit.paste": IF d >= 0 THEN CodeDoc(d).PasteFromClipboard
    CASE "edit.selectAll": IF d >= 0 THEN CodeDoc(d).SelectAll
    CASE "edit.goToLine": GoToLine
    ' ---- view ----
    CASE "view.commandPalette": ShowPalette ""
    CASE "view.quickOpen": ShowPalette ""
    CASE "view.project": Dock.FocusPane("projecttree")
    CASE "view.toolbox": Dock.FocusPane("toolbox")
    CASE "view.properties": Dock.FocusPane("inspector")
    CASE "view.outline": Dock.FocusPane("outlinetree")
    CASE "view.output": OutputBox.Page = "output": Dock.FocusPane("outputbox")
    CASE "view.problems": OutputBox.Page = "problems": Dock.FocusPane("outputbox")
    CASE "view.immediate": Dock.FocusPane("immediatebox")
    CASE "view.theme.rapidr": SetTheme "rapidr"
    CASE "view.theme.light": SetTheme "rapidr light"
    CASE "view.theme.dark": SetTheme "rapidr dark"
    CASE "view.theme.highcontrast": SetTheme "rapidr high contrast"
    CASE "view.theme.classic": SetTheme "classic"
    CASE "view.previewClassic": TogglePreviewClassic
    CASE "view.layout": Dock.ResetLayout
    CASE "view.settings": ShowPalette "View: "
    CASE "view.keyboardShortcuts": ShowShortcuts
    ' ---- project ----
    CASE "project.addFile": AddFileToProject
    CASE "project.addForm": AddProjectFile "form"
    CASE "project.addModule": AddProjectFile "module"
    CASE "project.options": ShowAppSettings
    ' ---- run / debug ----
    CASE "run.start": StartProgram 1
    CASE "run.withoutDebugging": StartProgram 0
    CASE "run.stop": StopProgram
    CASE "run.restart": StopProgram: StartProgram 1
    CASE "run.continue": Session.Continue
    CASE "run.pause": Session.Pause
    CASE "debug.stepInto": IF RunState = "paused" THEN Session.StepIn
    CASE "debug.stepOver": IF RunState = "paused" THEN Session.StepOver
    CASE "debug.stepOut": IF RunState = "paused" THEN Session.StepOut
    CASE "run.build": BuildProject ""
    CASE "run.buildNative": BuildProject "native"
    CASE "run.buildInterpreted": BuildProject "interpreted"
    CASE "run.reveal": RevealBuild
    ' ---- window ----
    CASE "window.next": Dock.NextDocument
    CASE "window.previous": Dock.PreviousDocument
    ' ---- help ----
    CASE "help.about": ShowAbout
    CASE ELSE
      OutputLine "(" + Id + ": not there yet)"
  END SELECT
END SUB

' ---- events ----------------------------------------------------------------------

SUB StudioResize
  LayOutStatus
  LayOutWelcome
END SUB

SUB StudioClose(Action AS INTEGER)
  ' (changes not saved: asked first; Cancel keeps Studio open)
  IF AskToSaveAll = 0 THEN
    Action = 0
    EXIT SUB
  END IF
  IF RunState <> "stopped" THEN Session.Stop
  SaveSettings
END SUB

' (keys the menus don't hold: VB6's F8 stepping beside F11 / F10)
SUB StudioKey(Key AS INTEGER, Shift AS INTEGER)
  IF Key = 119 THEN
    IF Shift = 0 THEN RunCommand "debug.stepInto"
    IF Shift = 256 THEN RunCommand "debug.stepOver"
  END IF
END SUB

SUB PaneChanged(Name AS STRING)
END SUB

SUB DocumentActivated(Name AS STRING)
  DIM d AS INTEGER, was AS INTEGER
  was = ActiveDoc
  ' (its view — Design, Code — says DocKind: workspace.inc)
  d = DocOfPane(Name)
  IF d >= 0 THEN ActiveDoc = d
  IF ActiveDoc <> was THEN AnalyzeActive
  UpdateCommands
END SUB

' (a document's window closed by its button: its file's documents go)
SUB DocumentClosing(Name AS STRING, Cancel AS INTEGER)
  DIM d AS INTEGER, k AS STRING
  k = LCASE$(Name)
  FOR d = 0 TO DocCount - 1
    IF DocOpen(d) AND k = "codedoc(" + LTRIM$(STR$(d)) + ")" THEN
      ' (its changes: Save / Don't Save / Cancel keeps it open)
      IF AskToSave(d) = 0 THEN
        Cancel = 1
        EXIT SUB
      END IF
      DocOpen(d) = 0
      Lang.Close DocFile(d)
      IF ActiveDoc = d THEN ActiveDoc = -1
    END IF
  NEXT
  UpdateCommands
END SUB

SUB WelcomeLink(Sender AS QLABEL)
  SELECT CASE Sender.Tag
    CASE 1: RunCommand "file.newProject"
    CASE 2: RunCommand "file.open"
    CASE 3: ShowPalette "Example"
    CASE ELSE: RunCommand "view.commandPalette"
  END SELECT
END SUB

SUB DesignSelected(Index AS INTEGER)
END SUB

SUB OutlineOpen
  DIM i AS INTEGER, d AS INTEGER
  i = OutlineTree.ItemIndex
  d = ActiveCode
  IF i < 0 OR i >= OutlineCount OR d < 0 THEN EXIT SUB
  Dock.FocusPane("codedoc(" + LTRIM$(STR$(d)) + ")")
  CodeDoc(d).GotoLine OutlineLine(i)
END SUB

' ---- start-up ----------------------------------------------------------------------
' Arguments: [--home <RapidR's folder>] [--theme <name>] [--fresh]
' [--maximized] [--do id,…] [file]:
' `rapidr ide [file]` on the desktop, the web page's (RAPIDR_ARGS) in the
' browser. --fresh: the default layout and settings (the tests').

SUB StartStudio
  DIM i AS INTEGER, a AS STRING, startFile AS STRING, fresh AS INTEGER
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
    ELSEIF a = "--fresh" THEN
      fresh = 1
    ELSEIF a = "--maximized" THEN
      Studio.WindowState = 2
    ELSEIF a = "--do" THEN
      i = i + 1
      DoList = COMMAND$(i)
    ELSE
      startFile = a
    END IF
    i = i + 1
  WEND
  DefaultFolder = HomeDir + "projects"
  IF fresh = 0 THEN LoadSettings
  ' (RapidR's look by default, following the system)
  IF ThemeChoice = "" THEN ThemeChoice = "rapidr"
  Application.Theme = ThemeChoice
  RunState = "stopped"
  StateText = "Ready"
  ActiveDoc = -1
  Session.OnOutput = ProgramOutput
  Session.OnStopped = ProgramStopped
  Session.OnContinue = ProgramContinued
  Session.OnExit = ProgramExited
  SetUpBuild
  DefineCommands
  BuildMenus
  BuildToolBar
  SetUpStatus
  SetUpPanels
  SetUpDock
  SetUpDocuments
  SetUpSearch
  IF SavedLayout <> "" THEN Dock.LoadLayout(SavedLayout)
  FillExamples
  FillPalette
  FillRecent
  ApplyChrome
  FillProjectTree
  InspectActive
  OutputLine "RapidR Studio is ready."
  LayOutWelcome
  ApplyPreview
  IF startFile <> "" THEN OpenProject startFile
  UpdateTitle
  UpdateCommands
  IF DoList <> "" THEN Macro.Enabled = 1
END SUB

' --do id,id,…: the commands run one after the other once Studio is up
' ("wait" lets a turn go by): `rapidr ide --do run.start file.rr` opens and
' runs; the tests drive Studio this way, through the very commands.
SUB MacroTick
  DIM k AS INTEGER, id AS STRING
  IF DoList = "" THEN
    Macro.Enabled = 0
    EXIT SUB
  END IF
  k = INSTR(DoList, ",")
  IF k > 0 THEN
    id = LEFT$(DoList, k - 1)
    DoList = MID$(DoList, k + 1)
  ELSE
    id = DoList
    DoList = ""
  END IF
  id = LTRIM$(RTRIM$(id))
  IF id <> "" AND id <> "wait" THEN
    OutputLine "> " + id
    ' (S-PANELS: the panels' test steps, tool: prop: …, else a command)
    IF PanelStep(id) = 0 THEN RunCommand id
  END IF
END SUB
