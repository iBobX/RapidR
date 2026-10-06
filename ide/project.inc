' project.inc: the project (RProject), running it (RProgramSession), and
' Studio's settings (QREGISTRY: the per-user store, on the web the page's).

' ---- the project ---------------------------------------------------------------------

SUB FillProjectTree
  DIM i AS INTEGER, k AS INTEGER, p AS STRING, ext AS STRING
  ProjectTree.Clear
  FileIcons.Clear
  IF ProjectOpen = 0 THEN
    ProjectTree.AddItems "No project open"
    TreeIcon ProjectTree, 0, FileIcon("files/folder")
    EXIT SUB
  END IF
  ProjectTree.AddItems Proj.Name
  TreeIcon ProjectTree, 0, FileIcon("files/rrproj")
  TreePath(0) = ""
  FOR i = 0 TO Proj.FileCount - 1
    p = Proj.File(i)
    ProjectTree.AddChildItems 0, p
    k = ProjectTree.ItemCount - 1
    ext = FileExt(p)
    IF Proj.FileKind(i) = "form" THEN
      TreeIcon ProjectTree, k, FileIcon("components/RFORM")
    ELSE
      TreeIcon ProjectTree, k, FileIcon("files/" + ext)
    END IF
    IF k < 200 THEN TreePath(k) = Proj.FullPath(i)
  NEXT
  ProjectTree.FullExpand
END SUB

' Opens a project (.rrproj) or a source file (with what it includes) and
' its main file's documents.
SUB OpenProject(Path AS STRING)
  IF Path = "" THEN EXIT SUB
  IF Proj.Open(Path) = 0 THEN
    OutputLine "Can't open " + Path + ": " + Proj.Error
    SetState "error", "Can't open " + FileTitle(Path)
    EXIT SUB
  END IF
  CloseAllDocuments
  ProjectOpen = 1
  ProjectName = Proj.Name
  ProjectMain = Proj.Folder
  IF ProjectMain <> "" THEN ProjectMain = ProjectMain + "/"
  ProjectMain = ProjectMain + Proj.MainFile
  ProjectFile = Path
  Lang.CompatMode = Proj.CompatMode
  AddRecent Path
  FillProjectTree
  ' (the start page makes room: the form and its code side by side)
  Dock.ClosePane("welcome")
  OpenDocument ProjectMain
  IF DocumentMode = "mdi" THEN Dock.TileVertical
  SetState "stopped", "Ready"
  UpdateTitle
  UpdateCommands
END SUB

SUB CloseAllDocuments
  DIM d AS INTEGER
  FOR d = 0 TO DocCount - 1
    IF DocOpen(d) THEN CloseDocument d
  NEXT
END SUB

SUB CloseProject
  IF RunState <> "stopped" THEN Session.Stop
  CloseAllDocuments
  Proj.Close
  ProjectOpen = 0
  ProjectName = ""
  FillProjectTree
  Dock.ShowPane("welcome")
  Dock.DocumentState("welcome", 2)
  UpdateTitle
  UpdateCommands
END SUB

' File > Open: a project or a source file.
SUB AskOpen
  OpenDlg.Caption = "Open Project or File"
  OpenDlg.Filter = "RapidR projects and sources (*.rrproj;*.rr;*.bas;*.inc)|*.rrproj;*.rr;*.bas;*.inc|All files|*.*"
  IF OpenDlg.Execute THEN OpenProject OpenDlg.FileName
END SUB

' File > New Project: a form program (or a console one) where the user says.
SUB AskNew
  NewName.Text = "MyProgram"
  NewKind.ItemIndex = 0
  NewFolder.Text = DefaultFolder
  NewDialog.ShowModal
END SUB

SUB NewBrowse
  SaveDlg.Caption = "Where the new project goes"
  SaveDlg.FileName = NewName.Text + ".rrproj"
  SaveDlg.Filter = "RapidR projects (*.rrproj)|*.rrproj"
  IF SaveDlg.Execute THEN
    NewFolder.Text = LEFT$(SaveDlg.FileName, RINSTR(REPLACESUBSTR$(SaveDlg.FileName, "\", "/"), "/") - 1)
    NewName.Text = BaseName(SaveDlg.FileName)
  END IF
END SUB

SUB NewCreate
  DIM template AS STRING, folder AS STRING
  IF NewKind.ItemIndex = 1 THEN template = "console" ELSE template = "gui"
  folder = NewFolder.Text
  IF RIGHT$(folder, 1) <> "/" THEN folder = folder + "/"
  folder = folder + NewName.Text
  MKDIR folder
  IF Proj.New(template, NewName.Text, folder) THEN
    NewDialog.Close
    OpenProject Proj.FileName
  ELSE
    SHOWMESSAGE "Can't make the project: " + Proj.Error
  END IF
END SUB

SUB NewCancel
  NewDialog.Close
END SUB

SUB SaveProjectFile
  IF ProjectOpen = 0 THEN EXIT SUB
  IF Proj.Save THEN
    ProjectFile = Proj.FileName
    AddRecent ProjectFile
    OutputLine "Saved " + ProjectFile
  ELSE
    OutputLine "Can't save the project: " + Proj.Error
  END IF
END SUB

SUB ProjectTreeOpen
  DIM i AS INTEGER
  i = ProjectTree.ItemIndex
  IF i <= 0 OR i >= 200 THEN EXIT SUB
  IF TreePath(i) <> "" THEN OpenDocument TreePath(i)
END SUB

' ---- running the program (RProgramSession) -----------------------------------------------

SUB StartProgram(WithDebugger AS INTEGER)
  IF ProjectOpen = 0 OR RunState <> "stopped" THEN EXIT SUB
  ' (VB's way: what is being run is saved first)
  SaveAll
  AnalyzeActive
  IF Lang.ErrorCount > 0 THEN
    SetState "error", "Can't run: the program has errors"
    Dock.FocusPane("problemsview")
    EXIT SUB
  END IF
  OutputBox.Clear
  Dock.ShowPane("outputbox")
  Session.Program = ProjectMain
  Session.Debug = WithDebugger
  IF Session.Start THEN
    SetState "running", "Running " + FileTitle(ProjectMain)
    RunStarted = TIMER
  ELSE
    OutputLine "Can't run " + ProjectMain + ": " + Session.Error
    SetState "error", "Can't run " + FileTitle(ProjectMain)
  END IF
END SUB

SUB StopProgram
  IF RunState <> "stopped" THEN Session.Stop
END SUB

SUB ProgramOutput(Text AS STRING)
  OutputBox.SelStart = LEN(OutputBox.Text)
  OutputBox.SelText = Text
END SUB

SUB ProgramStopped(Reason AS STRING, InFile AS STRING, LineNo AS INTEGER)
  SetState "paused", "Paused (" + Reason + ") at " + FileTitle(InFile) + ", line " + LTRIM$(STR$(LineNo))
END SUB

SUB ProgramContinued
  SetState "running", "Running " + FileTitle(ProjectMain)
END SUB

SUB ProgramExited(Code AS INTEGER)
  OutputLine "[" + FileTitle(ProjectMain) + " ended, exit code " + LTRIM$(STR$(Code)) + "]"
  IF Code = 0 THEN SetState "stopped", "Ready" ELSE SetState "error", "Ended with exit code " + LTRIM$(STR$(Code))
END SUB

' The Immediate window: a line typed and Enter — "? x" prints x in the paused
' program, a statement runs there.
SUB ImmediateKey(Key AS INTEGER, Shift AS INTEGER)
  DIM n AS INTEGER, typed AS STRING
  IF Key <> 13 THEN EXIT SUB
  n = ImmediateBox.LineCount
  IF n < 1 THEN EXIT SUB
  typed = ImmediateBox.Line(n - 1)
  IF LTRIM$(typed) = "" THEN EXIT SUB
  IF RunState = "paused" THEN
    ImmediateBox.AddStrings Session.Evaluate(typed)
  ELSE
    ImmediateBox.AddStrings "(the program isn't paused)"
  END IF
END SUB

' ---- settings (the per-user store) -------------------------------------------------------

CONST STUDIO_KEY = "Software\RapidR\Studio"
CONST HKCU = &H80000001

SUB LoadSettings
  Reg.RootKey = HKCU
  IF Reg.KeyExists(STUDIO_KEY) THEN
    Reg.OpenKey(STUDIO_KEY, 0)
    IF ThemeChoice = "" AND Reg.ValueExists("Theme") THEN ThemeChoice = Reg.ReadString("Theme")
    IF Reg.ValueExists("DocumentMode") THEN DocumentMode = Reg.ReadString("DocumentMode")
    IF Reg.ValueExists("Layout") THEN SavedLayout = Reg.ReadString("Layout")
    IF Reg.ValueExists("Recent") THEN RecentList = Reg.ReadString("Recent")
    IF Reg.ValueExists("Folder") THEN DefaultFolder = Reg.ReadString("Folder")
    Reg.CloseKey
  END IF
END SUB

SUB SaveSettings
  Reg.RootKey = HKCU
  Reg.OpenKey(STUDIO_KEY, 1)
  Reg.WriteString("Theme", ThemeChoice)
  Reg.WriteString("DocumentMode", DocumentMode)
  Reg.WriteString("Layout", Dock.SaveLayout)
  Reg.WriteString("Recent", RecentList)
  Reg.WriteString("Folder", DefaultFolder)
  Reg.CloseKey
END SUB

' (the recent projects, newest first, one a line, at most 8)
SUB AddRecent(Path AS STRING)
  DIM acc AS STRING, n AS INTEGER, i AS INTEGER, p AS STRING, kept AS INTEGER
  acc = Path + CHR$(10)
  kept = 1
  n = TALLY(RecentList, CHR$(10))
  FOR i = 1 TO n
    p = FIELD$(RecentList, CHR$(10), i)
    IF p <> "" AND LCASE$(p) <> LCASE$(Path) AND kept < 8 THEN
      acc = acc + p + CHR$(10)
      kept = kept + 1
    END IF
  NEXT
  RecentList = acc
  FillRecent
END SUB
