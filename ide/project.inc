' project.inc: the project (RProject), running it (RProgramSession), and
' Studio's settings (QREGISTRY: the per-user store, on the web the page's).

' ---- the project ---------------------------------------------------------------------

' Opens a project (.rrproj) or a source file (with what it includes) and
' its main file's documents.
SUB OpenProject(Path AS STRING)
  DIM i AS INTEGER
  IF Path = "" THEN EXIT SUB
  ' (the project open now keeps its breakpoints and watches: debug.inc)
  IF ProjectOpen THEN SaveSettings
  IF Proj.Open(Path) = 0 THEN
    ' (a Markdown file — a README, a report — opens on its own, in Preview:
    ' markdown.inc)
    IF OpenMarkdownFile(Path) THEN EXIT SUB
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
  ' (the project's include files read for the language service: on the web
  ' they are in the page's store, not on a disk it could read)
  FOR i = 0 TO Proj.FileCount - 1
    IF Proj.FileKind(i) = "include" THEN Lang.Update Proj.FullPath(i), Proj.FileText(Proj.FullPath(i))
  NEXT
  AddRecent Path
  FillProjectTree
  ' (the start page makes room; the files open when it was last closed
  ' come back as they were: workspace.inc)
  Dock.ClosePane("welcome")
  OpenDocument ProjectMain
  RestoreSession Path
  ' (its breakpoints and watches: debug.inc)
  LoadDebugState
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
  StopProgram
  CloseAllDocuments
  Proj.Close
  ProjectOpen = 0
  ProjectName = ""
  FillProjectTree
  ShowWelcome
  UpdateTitle
  UpdateCommands
END SUB

' File > Open: a project or a source file.
SUB AskOpen
  OpenDlg.Caption = "Open Project or File"
  OpenDlg.Filter = "RapidR projects and sources (*.rrproj;*.rr;*.bas;*.rqw;*.rqb;*.rq;*.inc)|*.rrproj;*.rr;*.bas;*.rqw;*.rqb;*.rq;*.inc|All files|*.*"
  IF OpenDlg.Execute THEN OpenProject OpenDlg.FileName
END SUB

' File > Open Folder: its project (.rrproj) or main source — on the web the
' folder's files read in, saved back to it (showDirectoryPicker).
SUB AskOpenFolder
  DIM f AS STRING
  OpenDlg.Caption = "Open Folder"
  OpenDlg.PickFolder = 1
  IF OpenDlg.Execute THEN
    f = OpenDlg.FileName
    OpenDlg.PickFolder = 0
    IF Proj.OpenFolder(f) THEN
      IF Proj.FileName <> "" THEN OpenProject Proj.FileName ELSE OpenProject Proj.Folder + "/" + Proj.MainFile
    ELSE
      OutputLine "Can't open " + f + ": " + Proj.Error
      SetState "error", "No RapidR project in " + FileTitle(f)
    END IF
  END IF
  OpenDlg.PickFolder = 0
END SUB

' File > Import RapidQ Project or File / Folder (R-NAMES): a COPY of a
' RapidQ program (a .bas, .rqw, .rqb, .rq or .inc file and what it
' includes, or a folder of them) with RapidR's names, beside it
' (<name>-rapidr), each program proved to compile to the same bytecode as
' its original; the original is only read. The copy opens as a project,
' its report (what changed, line by line, and the proof) beside the code.
SUB AskImportRapidQ
  OpenDlg.Caption = "Import RapidQ Project or File"
  OpenDlg.Filter = "RapidQ programs (*.bas;*.rqw;*.rqb;*.rq;*.inc)|*.bas;*.rqw;*.rqb;*.rq;*.inc|RapidR projects (*.rrproj)|*.rrproj|All files|*.*"
  IF OpenDlg.Execute THEN ImportRapidQ OpenDlg.FileName
END SUB

SUB AskImportRapidQFolder
  DIM f AS STRING
  OpenDlg.Caption = "Import RapidQ Folder"
  OpenDlg.PickFolder = 1
  IF OpenDlg.Execute THEN f = OpenDlg.FileName
  OpenDlg.PickFolder = 0
  IF f <> "" THEN ImportRapidQ f
END SUB

SUB ImportRapidQ(Source AS STRING)
  DIM made AS STRING, report AS STRING, summary AS STRING
  OutputLine "Importing " + Source + ": a copy with RapidR's names (the original is only read)"
  made = Proj.ImportRapidQ(Source)
  IF made = "" THEN
    OutputLine "Can't import " + Source + ": " + Proj.Error
    SetState "error", "Can't import " + FileTitle(Source)
    EXIT SUB
  END IF
  summary = Proj.ImportSummary
  report = Proj.ImportReport
  OutputLine summary
  OutputLine "The copy: " + made
  OpenProject made
  OpenDocument report
  SetState "stopped", "Imported " + FileTitle(Source) + " as a copy with RapidR's names"
END SUB

' File > New Project: a form program (or a console one) where the user says.
SUB AskNew
  NewName.Text = "MyProgram"
  PickTemplate 0
  NewFolder.Text = DefaultFolder
  NewDialog.Center
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
  template = TplId(TplChosen)
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

' (running the program and debugging it: debug.inc)

' ---- settings (the per-user store) -------------------------------------------------------

CONST STUDIO_KEY = "Software\RapidR\Studio"
CONST HKCU = &H80000001

SUB LoadSettings
  Reg.RootKey = HKCU
  IF Reg.KeyExists(STUDIO_KEY) THEN
    Reg.OpenKey(STUDIO_KEY, 0)
    SettingsRead = 1
    IF ThemeChoice = "" AND Reg.ValueExists("Theme") THEN ThemeChoice = Reg.ReadString("Theme")
    IF Reg.ValueExists("PreviewClassic") THEN PreviewClassic = Flag(Reg.ReadString("PreviewClassic") = "1")
    IF Reg.ValueExists("Layout") THEN SavedLayout = Reg.ReadString("Layout")
    LoadSession
    IF Reg.ValueExists("Recent") THEN RecentList = Reg.ReadString("Recent")
    IF Reg.ValueExists("Folder") THEN DefaultFolder = Reg.ReadString("Folder")
    Reg.CloseKey
  END IF
END SUB

SUB SaveSettings
  Reg.RootKey = HKCU
  Reg.OpenKey(STUDIO_KEY, 1)
  Reg.WriteString("Theme", ThemeChoice)
  Reg.WriteString("PreviewClassic", LTRIM$(STR$(PreviewClassic)))
  Reg.WriteString("Layout", Dock.SaveLayout)
  SaveSession
  SaveDebugState
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
