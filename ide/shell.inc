' shell.inc: where every command goes (RunCommand), the window's and the
' panes' events, and start-up.

' The active document's code editor (-1: none).
FUNCTION ActiveCode AS INTEGER
  Result = -1
  IF ActiveDoc >= 0 THEN
    IF DocOpen(ActiveDoc) THEN Result = ActiveDoc
  END IF
END FUNCTION

' What Help > About says: the version is the build's own (the language
' service has it from the workspace's), not a number written here.
FUNCTION AboutText AS STRING
  DIM nl AS STRING
  nl = CHR$(13) + CHR$(10)
  Result = TR$("RapidR Studio (RapidR {version})", "version", Lang.Version) + nl + TR$("BASIC is powerful again: desktop and web apps from one source.") + nl + nl + TR$("Copyright (c) 2025-2026 Ruta Internet SRL. MIT licence.")
END FUNCTION

SUB ShowAbout
  SHOWMESSAGE AboutText
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
  SaveDlg.Caption = TR$("Save As")
  SaveDlg.FileName = FileTitle(DocFile(d))
  SaveDlg.Filter = TR$("RapidR sources") + " (*.rr;*.bas;*.rqw;*.rqb;*.rq;*.inc)|*.rr;*.bas;*.rqw;*.rqb;*.rq;*.inc|" + TR$("All files") + "|*.*"
  IF SaveDlg.Execute THEN
    ' (the file it was is no longer this document's: not watched)
    Proj.Unwatch DocFile(d)
    DocFile(d) = SaveDlg.FileName
    SaveDocument d
    Proj.AddFile SaveDlg.FileName
    FillProjectTree
  END IF
END SUB

SUB AddFileToProject
  OpenDlg.Caption = TR$("Add a File to the Project")
  OpenDlg.Filter = TR$("All files") + "|*.*"
  IF OpenDlg.Execute THEN
    IF Proj.AddFile(OpenDlg.FileName) THEN FillProjectTree ELSE OutputLine Proj.Error
  END IF
END SUB

SUB RunCommand(Id AS STRING)
  DIM d AS INTEGER
  ' (what Studio is doing, for a crash or hang report: the command's id)
  Proj.CrashDoing Id
  d = ActiveCode
  ' (Edit's in Variables or Watch, when they have the focus — the selected
  ' value copied, the last value set undone: debug.inc)
  IF DebugPaneCommand(Id) THEN EXIT SUB
  ' (Edit's Copy and Select All in the Output pane, when it has the
  ' focus: panels.inc)
  IF OutputCommand(Id) THEN EXIT SUB
  ' (Edit > Copy in the AI Apps pane's activity: mcp.inc)
  IF McpPaneCommand(Id) THEN EXIT SUB
  ' (View's and Edit's on a Markdown file's Preview | Source: markdown.inc)
  IF MarkdownCommand(Id) THEN EXIT SUB
  ' (the designer's commands, and Edit's on a designer: designer.inc)
  IF DesignCommand(Id) THEN EXIT SUB
  ' (Run's and Debug's: debug.inc)
  IF DebugCommand(Id) THEN EXIT SUB
  ' (the documents' views and groups, Find in Files, Help: workspace.inc, search.inc)
  IF WorkspaceCommand(Id) THEN EXIT SUB
  ' (the AI assistant's: ai.inc)
  IF AICommand(Id) THEN EXIT SUB
  ' (AI apps': mcp.inc)
  IF McpCommand(Id) THEN EXIT SUB
  ' (Versions': versions.inc)
  IF VersionsCommand(Id) THEN EXIT SUB
  ' (files changed outside Studio, Reopen Project: external.inc)
  IF ExternalCommand(Id) THEN EXIT SUB
  SELECT CASE Id
    ' ---- file ----
    CASE "file.newProject": AskNew
    CASE "file.open": AskOpen
    CASE "file.openFolder": AskOpenFolder
    CASE "file.clearRecent": ClearRecent
    CASE "file.openExample": ShowPalette TR$("Example")
    CASE "file.exampleAgain": CopyExampleAgain
    CASE "file.importRapidQ": AskImportRapidQ
    CASE "file.importRapidQFolder": AskImportRapidQFolder
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
    CASE "view.outputWrap": ToggleOutputWrap
    CASE "view.layout"
      ' (every pane where it started, both layouts at their first sizes)
      Dock.ResetLayout
      EditBottom = 0: DebugBottom = 0: DebugWidth = 0
      DebugLayoutLoaded
    CASE "window.resizePanes"
      IF Dock.FocusSplitter("") = 0 THEN SetState RunState, "No splitter to move"
    CASE "view.settings": ShowSettings
    CASE "view.keyboardShortcuts": ShowShortcuts
    CASE "tools.schoolLock": ShowSchoolLock
    ' ---- project ----
    CASE "project.addFile": AddFileToProject
    CASE "project.addForm"
      ' (the file open makes no form: one added to it; else a new form file
      ' of the program, its main file including it — Project >
      ' Add Form)
      d = ActiveCode
      IF d >= 0 AND DesignDoc(d).FormName = "" THEN
        d = DesignCommand("designer.addForm")
      ELSEIF ProjectOpen THEN
        AddProjectFile "form"
      ELSE
        SetState "stopped", "Open or make a project first (File > New Project)"
      END IF
    CASE "project.addModule": AddProjectFile "module"
    CASE "project.options": ShowAppSettings
    ' ---- run / debug ----
    CASE "run.build": BuildProject ""
    CASE "run.buildNative": BuildProject "native"
    CASE "run.buildInterpreted": BuildProject "interpreted"
    CASE "run.buildWeb": BuildProject "web"
    CASE "run.reveal": RevealBuild
    ' ---- window ----
    CASE "window.next": Dock.NextDocument
    CASE "window.previous": Dock.PreviousDocument
    ' ---- help ----
    CASE "help.about": ShowAbout
    CASE ELSE
      ' (a test step: import:<file or folder> — imported without the dialog)
      IF LEFT$(Id, 7) = "import:" THEN
        ImportRapidQ MID$(Id, 8)
      ELSEIF LEFT$(Id, 12) = "file.recent." THEN
        ' (File > Open Recent's items: the list's k-th, newest first)
        OpenRecent VAL(MID$(Id, 13))
      ELSEIF LEFT$(Id, 9) = "includes:" THEN
        ' (a test step: includes:<folder>|<folder> — Settings > Include
        ' folders without the dialog)
        SetIncludeFolders REPLACESUBSTR$(MID$(Id, 10), "|", CHR$(10))
      ELSEIF LEFT$(Id, 3) = "ai:" THEN
        ' (a test step: ai:<what>=<value> — ai.inc's AIStep)
        AIStep MID$(Id, 4)
      ELSEIF LEFT$(Id, 4) = "mcp:" THEN
        ' (a test step: mcp:<what>=<value> — mcp.inc's McpStep)
        McpStep MID$(Id, 5)
      ELSEIF LEFT$(Id, 5) = "lock:" THEN
        ' (a test step: lock:<what>=<value> — schoollock.inc's SchoolLockStep)
        SchoolLockStep MID$(Id, 6)
      ELSEIF LEFT$(Id, 9) = "versions:" THEN
        ' (a test step: versions:<what>=<value> — versions.inc's VersionsStep)
        VersionsStep MID$(Id, 10)
      ELSEIF LEFT$(Id, 4) = "ext:" THEN
        ' (a test step: ext:<what>=<value> — external.inc's ExternalStep)
        ExternalStep MID$(Id, 5)
      ELSEIF LEFT$(Id, 7) = "anchor:" THEN
        ' (a test step: anchor:<a heading's anchor> — the Markdown document
        ' open shows that heading, as its outline row clicked)
        IF ActiveDoc >= 0 THEN
          IF DocIsMd(ActiveDoc) THEN MdDoc(ActiveDoc).ScrollTo MID$(Id, 8)
        END IF
      ELSEIF Id = "about:text" THEN
        ' (a test step: about:text — what Help > About says, in the Output)
        OutputLine AboutText
      ELSEIF EditorCommand(Id) = 0 THEN
        OutputLine TR$("({id}: not there yet)", "id", Id)
      END IF
  END SELECT
END SUB

' ---- events ----------------------------------------------------------------------

SUB StudioResize
  LayOutStatus
  LayOutWelcome
END SUB

' (the keyboard starts in the Project pane, as it always has: the Welcome
' page's cards are Tab stops, and come first in the window)
SUB StudioShown
  IF ActiveDoc < 0 THEN
    ' (S2: the offer to reopen the last project has the keyboard: Enter)
    IF StartShift > 0 THEN ReopenRow.SetFocus ELSE Dock.FocusPane("projecttree")
  END IF
END SUB

SUB StudioClose(Action AS INTEGER)
  ' (changes not saved: asked first; Cancel keeps Studio open)
  IF AskToSaveAll = 0 THEN
    Action = 0
    EXIT SUB
  END IF
  ' (every change saved or let go: the copies of unsaved work go)
  KeepEnd
  StopProgram
  Session.StopBrowser
  SaveSettings
  ' (AI apps: disconnected, the server off)
  McpShutdown
END SUB

' (keys the menus don't hold: F8 stepping beside F11 / F10)
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
  IF ActiveDoc <> was THEN
    ' (another tab: its form's properties — but not the main file the project's
    ' look was just written into)
    IF ProjectInspecting AND ProjectOpening = 0 THEN
      IF ProjectKeepDoc > 0 AND ActiveDoc = ProjectKeepDoc - 1 THEN
        ProjectKeepDoc = 0
      ELSE
        LeaveProjectInspector
      END IF
    END IF
    AnalyzeActive
    DesignNames ActiveDoc
  END IF
  IF ActiveDoc >= 0 THEN EditorFacts ActiveDoc
  ' (its question about a change outside Studio, if it has one: external.inc)
  ExtShowBar
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
      ' (no longer watched for changes outside Studio: external.inc)
      ExtClosed d
      DocOpen(d) = 0
      Lang.Close DocFile(d)
      IF ActiveDoc = d THEN ActiveDoc = -1
    END IF
  NEXT
  UpdateCommands
END SUB

SUB WelcomeLink(Sender AS RLabel)
  StartAction Sender.Tag
END SUB

SUB StartRowPicked(Sender AS RPanel)
  StartAction Sender.Tag
END SUB

' (the Welcome page's Start rows: by mouse, or Tab and Enter / Space)
SUB StartAction(Which AS INTEGER)
  SELECT CASE Which
    CASE 1: RunCommand "file.newProject"
    CASE 2: RunCommand "file.open"
    CASE 3: RunCommand "file.importRapidQ"
    CASE 4: ShowPalette TR$("Example")
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
  IF MarkdownOutlineOpen(d, i) THEN EXIT SUB
  Dock.FocusPane("codedoc(" + LTRIM$(STR$(d)) + ")")
  CodeDoc(d).GotoLine OutlineLine(i)
  EditorFacts d
END SUB

' ---- start-up ----------------------------------------------------------------------
' Arguments: [--home <RapidR's folder>] [--theme <name>] [--fresh]
' [--maximized] [--do id,…] [file]:
' `rapidr ide [file]` on the desktop, the web page's (RAPIDR_ARGS) in the
' browser. --fresh: the default layout and settings (the tests').

SUB StartStudio
  DIM i AS INTEGER, a AS STRING, startFile AS STRING, fresh AS INTEGER
  DIM langArg AS STRING, langGiven AS INTEGER
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
    ELSEIF a = "--lang" THEN
      ' (Studio in a language for this run: a test's, the pseudo-language xx)
      i = i + 1
      langArg = COMMAND$(i)
      langGiven = 1
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
  ' (from here on an error in Studio's own code is said, never the end of
  ' Studio; unsaved work is kept as it is typed: safety.inc)
  SetUpSafety
  ' (RapidR's crash and hang reports: the strip after a Studio that ended
  ' unexpectedly, the hang watchdog — crash.inc)
  SetUpCrash
  ' (an installed Studio: Documents/RapidR/Projects, never inside RapidR)
  DefaultFolder = Proj.ProjectsFolder(HomeDir)
  ' (S2: offered unless the settings turned it off)
  ReopenLast = 1
  IF fresh = 0 THEN LoadSettings
  ' (School Lock: the teacher's lock and the computer's policy — read even
  ' with --fresh, before RAI and AI apps are set up: schoollock.inc)
  LoadSchoolLock
  IF langGiven THEN LanguageChoice = langArg
  ' (Studio's catalogs: ide/lang beside studio.rr on the desktop — the
  ' default — and ide/lang beside the page on the web, apart from the
  ' programs it runs, which have their own lang/)
  IF Proj.FileManager = "" THEN Application.LanguageFolder = HomeDir + "ide/lang"
  IF LanguageChoice <> "" THEN Application.Language = LanguageChoice
  ' (S6: your login shell's PATH for the programs run from here, read in the
  ' background now — a Run waits for it, three seconds at most)
  Session.ReadShellPath
  ' (RapidR's look by default, following the system)
  IF ThemeChoice = "" THEN ThemeChoice = "rapidr"
  Application.Theme = ThemeChoice
  RunState = "stopped"
  StateText = "Ready"
  ActiveDoc = -1
  PreviewDoc = -1
  SetUpBuild
  DefineCommands
  BuildMenus
  BuildToolBar
  SetUpStatus
  SetUpPanels
  SetUpDock
  SetUpDebug
  SetUpAI
  SetUpDocuments
  SetUpSearch
  SetUpVersions
  SetUpExternal
  SetUpMcp
  SetUpSchoolLock
  IF SavedLayout <> "" THEN Dock.LoadLayout(SavedLayout)
  ' (the Properties pane's names column where the user left its divider)
  InspectorNameWidthDefault = Inspector.NameWidth
  IF InspectorNameWidth >= 40 THEN Inspector.NameWidth = InspectorNameWidth
  ' (the editing layout — or the debugging one, Settings' Always —, at its
  ' sizes, whatever layout was kept: one saved while a program ran)
  DebugLayoutLoaded
  FillExamples
  FillPalette
  FillRecent
  ApplyChrome
  ' (from here on, the look's changes: the system's, View > Theme)
  Application.OnThemeChange = StudioThemeChanged
  ' (and the language's: Tools > Settings > General > Language)
  Application.OnLanguageChange = StudioLanguageChanged
  FillProjectTree
  InspectActive
  OutputLine TR$("RapidR Studio is ready.")
  LayOutWelcome
  ApplyPreview
  IF startFile <> "" THEN
    OpenProject startFile
  ELSEIF fresh = 0 THEN
    ' (S2: the last project and its files, offered on the Welcome page)
    ShowReopen
  END IF
  UpdateTitle
  UpdateCommands
  ' (unsaved work a Studio that ended without closing kept: Restore?)
  OfferRestoreSoon
  IF DoList <> "" THEN Macro.Enabled = 1
END SUB

' --do id,id,…: the commands run one after the other once Studio is up
' ("wait" lets a turn go by): `rapidr ide --do run.start file.rr` opens and
' runs; the tests drive Studio this way, through the very commands.
SUB MacroTick
  DIM k AS INTEGER, id AS STRING
  ' (keys a step sent still being typed: the next step waits for them)
  IF Application.KeysPending > 0 THEN EXIT SUB
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
  IF EditorStep(id) THEN EXIT SUB
  ' (lang:<tag> — Studio's language changed as Settings changes it, for
  ' the language tests; lang: alone, the system's)
  IF LEFT$(id, 5) = "lang:" THEN
    SetLanguage MID$(id, 6)
    EXIT SUB
  END IF
  ' (manual:<page> — the Help pane's link to a page of the manual)
  IF LEFT$(id, 7) = "manual:" THEN
    HelpLink id
    EXIT SUB
  END IF
  IF id <> "" AND id <> "wait" THEN
    OutputLine "> " + id
    ' (S-PANELS: the panels' test steps, tool: prop: …, else a command;
    ' safety.inc's safety: steps)
    IF SafetyStep(id) THEN EXIT SUB
    IF CrashStep(id) THEN EXIT SUB
    IF PanelStep(id) = 0 THEN
      IF DebugStep(id) = 0 THEN RunCommand id
    END IF
  END IF
END SUB
