' project.inc: the project (RProject), running it (RProgramSession), and
' Studio's settings (QREGISTRY: the per-user store, on the web the page's).

' ---- the project ---------------------------------------------------------------------

' Opens a project (.rrproj) or a source file (with what it includes) and
' its main file's documents. One of RapidR's examples installed where it
' can't be changed (the macOS app, /usr/lib/rapidr, Program Files) opens as
' the user's copy in Documents/RapidR/Examples/<name> — made the first time
' with its include and data files, as `rapidr examples copy` copies them,
' then opened as the user left it (never copied over): every way in goes
' through here (the Welcome page's cards, File > Open Example, the
' palette, Recent, File > Open, the command line). In a checkout the
' examples open where they are.
SUB OpenProject(Path AS STRING)
  DIM i AS INTEGER, p AS STRING, copied AS STRING
  IF Path = "" THEN EXIT SUB
  p = Proj.ExampleToOpen(Path, HomeDir + "examples")
  IF p = "" THEN
    OutputLine "Can't open the example " + FileTitle(Path) + ": " + Proj.Error
    SetState "error", "Can't open " + FileTitle(Path)
    EXIT SUB
  END IF
  copied = Proj.ExampleCopy
  ' (File > Copy Example Again's copy: said so)
  IF ExampleJustCopied <> "" THEN
    copied = ExampleJustCopied
    ExampleJustCopied = ""
  END IF
  ' (the project open now keeps its breakpoints and watches: debug.inc)
  IF ProjectOpen THEN SaveSettings
  IF Proj.Open(p) = 0 THEN
    ' (a Markdown file — a README, a report — opens on its own, in Preview:
    ' markdown.inc)
    IF OpenMarkdownFile(p) THEN EXIT SUB
    OutputLine "Can't open " + p + ": " + Proj.Error
    SetState "error", "Can't open " + FileTitle(p)
    EXIT SUB
  END IF
  CloseAllDocuments
  ' (S2: the start-up's offer to reopen is done with)
  HideReopen
  ProjectOpen = 1
  ProjectName = Proj.Name
  ProjectMain = Proj.Folder
  IF ProjectMain <> "" THEN ProjectMain = ProjectMain + "/"
  ProjectMain = ProjectMain + Proj.MainFile
  ProjectFile = p
  Lang.CompatMode = Proj.CompatMode
  ' (the project's include files read for the language service: on the web
  ' they are in the page's store, not on a disk it could read)
  FOR i = 0 TO Proj.FileCount - 1
    ' (an .inc that makes a form is a form file: read too)
    IF Proj.FileKind(i) = "include" OR FileExt(Proj.File(i)) = "inc" THEN Lang.Update Proj.FullPath(i), Proj.FileText(Proj.FullPath(i))
  NEXT
  AddRecent p
  FillProjectTree
  ' (the start page makes room; the files open when it was last closed
  ' come back as they were: workspace.inc)
  Dock.ClosePane("welcome")
  OpenDocument ProjectMain
  RestoreSession p
  ' (its breakpoints and watches: debug.inc)
  LoadDebugState
  SetState "stopped", "Ready"
  ' (an example's copy: the Output says where it is, and how to start over)
  ExampleOriginal = Proj.ExampleOf(p, HomeDir + "examples")
  IF ExampleOriginal <> "" THEN SayExampleCopy p, copied
  UpdateTitle
  UpdateCommands
  ' (another project than the assistant's new program: its banner goes)
  AIProjectChanged
END SUB

' The Output's line for an example's copy (Copied: "made" now, else the
' copy that was there): where it is, and "Copy again from the original".
SUB SayExampleCopy(Copy AS STRING, Copied AS STRING)
  DIM folder AS STRING
  folder = LEFT$(Copy, RINSTR(Copy, "/") - 1)
  ' (short lines: the Output pane is narrow; the link on its own)
  IF Copied = "again" THEN
    OutputLine "Copied " + FileTitle(Copy) + " again from the original, in:"
  ELSEIF Copied = "made" THEN
    OutputLine "Made your copy of the example " + FileTitle(Copy) + " (the installed examples can't be changed), in:"
  ELSE
    OutputLine "Your copy of the example " + FileTitle(Copy) + ", as you left it, in:"
  END IF
  OutputLine "  " + folder
  OutputLine "  " + Hyperlink("Copy again from the original", "example-again:")
END SUB

' File > Copy Example Again from the Original: the example's files (its
' program, includes, data) as RapidR has them, over the copy's — asked
' first (the changes made to them go; files added to the copy stay) —
' then the copy opened again.
SUB CopyExampleAgain
  DIM p AS STRING, name AS STRING, d AS INTEGER
  IF ExampleOriginal = "" OR ProjectOpen = 0 THEN EXIT SUB
  name = FileTitle(ExampleOriginal)
  IF AskCopyAgain("Copy " + name + " again from the original?", "Your changes to the example's own files, saved or not, are replaced by RapidR's. Files you added to its folder stay.") = 0 THEN EXIT SUB
  p = Proj.CopyExampleAgain(ProjectFile, HomeDir + "examples")
  IF p = "" THEN
    OutputLine "Can't copy " + name + " again: " + Proj.Error
    SetState "error", "Can't copy " + name + " again"
    EXIT SUB
  END IF
  ' (what was open goes without a question: its changes were asked about)
  StopProgram
  FOR d = 0 TO DocCount - 1
    DocDirty(d) = 0
  NEXT
  UpdateModified
  CloseAllDocuments
  ProjectOpen = 0
  ExampleJustCopied = "again"
  OpenProject p
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
  ExampleOriginal = ""
  FillProjectTree
  ShowWelcome
  UpdateTitle
  UpdateCommands
  AIProjectChanged
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
  DIM i AS INTEGER, f AS STRING, pick AS STRING
  ImportDlg.Caption = "Import RapidQ Project or File"
  ImportDlg.Filter = "RapidQ programs (*.bas;*.rqw;*.rqb;*.rq;*.inc)|*.bas;*.rqw;*.rqb;*.rq;*.inc|RapidR projects (*.rrproj)|*.rrproj|All files|*.*"
  ImportDlg.Mode = 0
  ImportDlg.MultiSelect = 1
  IF ImportDlg.Execute = 0 THEN EXIT SUB
  ' (the program among the files picked — its includes come with it: in a
  ' browser they are read in together, found by their names)
  ' (RapidQ's way: Files(0) is their folder, Files(1) … their names)
  pick = ImportDlg.FileName
  FOR i = 1 TO ImportDlg.SelCount
    f = ImportDlg.Files(i)
    IF ImportDlg.Files(0) <> "" THEN f = ImportDlg.Files(0) + "/" + f
    IF IsProgramFile(f) AND IsProgramFile(pick) = 0 THEN pick = f
  NEXT
  ImportRapidQ pick
END SUB

' (a program to run, not a file it includes: .bas .rqw .rqb .rq .rr .rrproj)
FUNCTION IsProgramFile(Path AS STRING) AS INTEGER
  DIM e AS STRING
  e = LCASE$(MID$(Path, RINSTR(Path, ".") + 1))
  Result = (e = "bas" OR e = "rqw" OR e = "rqb" OR e = "rq" OR e = "rr" OR e = "rrproj")
END FUNCTION

SUB AskImportRapidQFolder
  DIM f AS STRING
  OpenDlg.Caption = "Import RapidQ Folder"
  OpenDlg.PickFolder = 1
  IF OpenDlg.Execute THEN f = OpenDlg.FileName
  OpenDlg.PickFolder = 0
  IF f <> "" THEN ImportRapidQ f
END SUB

SUB ImportRapidQ(Source AS STRING)
  DIM made AS STRING, report AS STRING, summary AS STRING, missing AS STRING, row AS STRING, n AS INTEGER, i AS INTEGER
  OutputLine "Importing " + Source + ": a copy with RapidR's names and every file it includes (the original is only read)"
  made = Proj.ImportRapidQ(Source)
  IF made = "" THEN
    OutputLine "Can't import " + Source + ": " + Proj.Error
    SetState "error", "Can't import " + FileTitle(Source)
    EXIT SUB
  END IF
  summary = Proj.ImportSummary
  report = Proj.ImportReport
  missing = Proj.ImportMissing
  OutputLine summary
  ' (an include found nowhere is never passed over: each one, what to do)
  n = 0
  IF missing <> "" THEN n = TALLY(missing, CHR$(10)) + 1
  FOR i = 1 TO n
    row = FIELD$(missing, CHR$(10), i)
    OutputLine FIELD$(row, CHR$(9), 1) + ": include file not found: " + FIELD$(row, CHR$(9), 2) + " - " + FIELD$(row, CHR$(9), 3)
  NEXT
  OutputLine "The copy: " + made
  OpenProject made
  OpenDocument report
  IF n > 0 THEN
    SetState "error", "Imported " + FileTitle(Source) + ", but " + LTRIM$(STR$(n)) + " include file(s) weren't found: see Output"
  ELSE
    SetState "stopped", "Imported " + FileTitle(Source) + " as a copy with RapidR's names"
  END IF
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
' The saved layout's format (Studio 2026-10: documents as tabs, the debugger's
' and the toolbox's panes): an older one isn't restored
CONST LAYOUT_VERSION = 3
CONST HKCU = &H80000001

SUB LoadSettings
  Reg.RootKey = HKCU
  IF Reg.KeyExists(STUDIO_KEY) THEN
    Reg.OpenKey(STUDIO_KEY, 0)
    SettingsRead = 1
    IF ThemeChoice = "" AND Reg.ValueExists("Theme") THEN ThemeChoice = Reg.ReadString("Theme")
    IF Reg.ValueExists("PreviewClassic") THEN PreviewClassic = Flag(Reg.ReadString("PreviewClassic") = "1")
    ' (a layout saved by an older Studio — before documents were tabs, or
    ' without a panel added since — isn't restored: Studio starts with its
    ' default layout; LAYOUT_VERSION goes up when the layout changes)
    IF Reg.ValueExists("Layout") AND Reg.ValueExists("LayoutVersion") THEN
      IF Reg.ReadString("LayoutVersion") = LTRIM$(STR$(LAYOUT_VERSION)) THEN SavedLayout = Reg.ReadString("Layout")
    END IF
    LoadSession
    IF Reg.ValueExists("Recent") THEN RecentList = Reg.ReadString("Recent")
    ' (S2: on unless turned off)
    IF Reg.ValueExists("ReopenLast") THEN ReopenLast = Flag(Reg.ReadString("ReopenLast") <> "0")
    ' (the code's helpers: helpful unless chosen otherwise)
    IF Reg.ValueExists("Helpers") THEN EditorHelpers = Reg.ReadString("Helpers")
    IF Reg.ValueExists("Folder") THEN DefaultFolder = Reg.ReadString("Folder")
    ' (Tools > Settings > Include folders: settings.inc)
    IF Reg.ValueExists("IncludeFolders") THEN Proj.IncludeFolders = Reg.ReadString("IncludeFolders")
    ' (Tools > Settings > AI: ai.inc)
    LoadAISettings
    Reg.CloseKey
  END IF
END SUB

SUB SaveSettings
  Reg.RootKey = HKCU
  Reg.OpenKey(STUDIO_KEY, 1)
  Reg.WriteString("Theme", ThemeChoice)
  Reg.WriteString("PreviewClassic", LTRIM$(STR$(PreviewClassic)))
  Reg.WriteString("Layout", Dock.SaveLayout)
  Reg.WriteString("LayoutVersion", LTRIM$(STR$(LAYOUT_VERSION)))
  SaveSession
  SaveDebugState
  Reg.WriteString("Recent", RecentList)
  Reg.WriteString("ReopenLast", LTRIM$(STR$(ReopenLast)))
  Reg.WriteString("Helpers", EditorHelpers)
  Reg.WriteString("Folder", DefaultFolder)
  Reg.WriteString("IncludeFolders", Proj.IncludeFolders)
  SaveAISettings
  Reg.CloseKey
END SUB

' (the recent projects and files, newest first, one a line, at most
' MAXRECENT: File > Open Recent, the Welcome page's Recent)
SUB AddRecent(Path AS STRING)
  DIM acc AS STRING, n AS INTEGER, i AS INTEGER, p AS STRING, kept AS INTEGER
  acc = Path + CHR$(10)
  kept = 1
  n = TALLY(RecentList, CHR$(10))
  FOR i = 1 TO n
    p = FIELD$(RecentList, CHR$(10), i)
    IF p <> "" AND LCASE$(p) <> LCASE$(Path) AND kept < MAXRECENT THEN
      acc = acc + p + CHR$(10)
      kept = kept + 1
    END IF
  NEXT
  RecentList = acc
  FillRecent
END SUB

' (a recent item that isn't there any more: off the list)
SUB ForgetRecent(Path AS STRING)
  DIM acc AS STRING, i AS INTEGER, p AS STRING
  FOR i = 1 TO TALLY(RecentList, CHR$(10))
    p = FIELD$(RecentList, CHR$(10), i)
    IF p <> "" AND LCASE$(p) <> LCASE$(Path) THEN acc = acc + p + CHR$(10)
  NEXT
  RecentList = acc
  FillRecent
END SUB

' File > Open Recent's k-th item (the Welcome page's Recent links too): it
' opens — one that isn't there any more is taken off the list, and the
' status bar and the Output say so.
SUB OpenRecent(k AS INTEGER)
  DIM p AS STRING
  IF k < 1 OR k > TALLY(RecentList, CHR$(10)) THEN EXIT SUB
  p = FIELD$(RecentList, CHR$(10), k)
  IF p = "" THEN EXIT SUB
  IF FILEEXISTS(p) = 0 THEN
    ForgetRecent p
    OutputLine FileTitle(p) + " isn't there any more (" + p + "): taken off Open Recent"
    ' (short: the status bar's first part is narrow; the Output says where)
    SetState "error", FileTitle(p) + " isn't there any more"
    SaveSettings
    UpdateCommands
    EXIT SUB
  END IF
  OpenProject p
END SUB

' File > Open Recent > Clear Recent: the list emptied (the files stay).
SUB ClearRecent
  RecentList = ""
  FillRecent
  SaveSettings
  OutputLine "Open Recent cleared"
  SetState RunState, "Open Recent cleared"
  UpdateCommands
END SUB

' (S2) The start-up's offer: the project Studio last had open, with the
' files that were open in it — a row on the Welcome page, focused, so
' Enter reopens it (Tools > Settings > General turns the offer off). One
' that isn't there any more isn't offered; it goes off Recent, said once.
FUNCTION ReopenCandidate AS STRING
  IF ReopenLast = 0 OR SessionProject = "" THEN EXIT FUNCTION
  IF FILEEXISTS(SessionProject) = 0 THEN
    OutputLine "The last project, " + FileTitle(SessionProject) + ", isn't there any more (" + SessionProject + "): taken off Open Recent"
    ForgetRecent SessionProject
    SessionProject = ""
    SessionFiles = ""
    EXIT FUNCTION
  END IF
  Result = SessionProject
END FUNCTION

SUB ReopenLastProject
  DIM p AS STRING
  p = ReopenPath
  HideReopen
  IF p <> "" THEN OpenProject p
END SUB
