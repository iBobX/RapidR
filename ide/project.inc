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
    OutputLine TR$("Can't open the example {file}: {error}", "file", FileTitle(Path), "error", Proj.Error)
    SetState "error", TR$("Can't open {file}", "file", FileTitle(Path))
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
  LeaveProjectInspector
  IF Proj.Open(p) = 0 THEN
    ' (a Markdown file — a README, a report — opens on its own, in Preview:
    ' markdown.inc)
    IF OpenMarkdownFile(p) THEN EXIT SUB
    OutputLine TR$("Can't open {file}: {error}", "file", p, "error", Proj.Error)
    SetState "error", TR$("Can't open {file}", "file", FileTitle(p))
    EXIT SUB
  END IF
  ' (a browser gave Studio the project file alone: its folder has the files)
  IF ProjectFilesElsewhere(p) THEN EXIT SUB
  CloseAllDocuments
  ' (S2: the start-up's offer to reopen is done with)
  HideReopen
  ProjectOpen = 1
  ProjectName = Proj.Name
  ProjectMain = Proj.Folder
  IF ProjectMain <> "" THEN ProjectMain = ProjectMain + "/"
  ProjectMain = ProjectMain + Proj.MainFile
  ProjectFile = p
  ' (the project file watched for changes outside Studio: external.inc)
  ExtProjectOpened
  Lang.CompatMode = Proj.CompatMode
  ' (S5: a module is checked as part of its project, never alone; what
  ' works on one runtime only is noted when the project runs on the other)
  Lang.Project = ProjectMain
  Lang.Targets = Proj.Targets
  Session.Args = Proj.RunArgs
  ' (the look the main program names: the designers draw in it)
  RefreshProgramLook 1
  ApplyDesignerSettings
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
  ' (a main file another file of the project includes: the program likely
  ' starts in that one — said, with where to change it; Robert's
  ' ShopInvoices: main = lang.rr, which main.rr includes)
  SayMainIncluded
  UpdateTitle
  UpdateCommands
  ' (another project than the assistant's new program: its banner goes)
  AIProjectChanged
  ' (its versions: the pane, the badges, the editors' marks — versions.inc)
  VersionsProjectChanged
END SUB

SUB SayMainIncluded
  DIM by AS STRING
  IF ProjectOpen = 0 OR Proj.Kind <> "project" THEN EXIT SUB
  by = Proj.MainIncludedBy
  IF by = "" THEN EXIT SUB
  OutputLine TR$("The main file {main} is included by {by}: the program probably starts in {by}. To run it from there, select the project's name in the Project pane, then MainFile in Properties.", "main", Proj.MainFile, "by", by)
  SetState "stopped", TR$("The main file is {main}, which {by} includes", "main", Proj.MainFile, "by", by)
END SUB

' The Output's line for an example's copy (Copied: "made" now, else the
' copy that was there): where it is, and "Copy again from the original".
SUB SayExampleCopy(Copy AS STRING, Copied AS STRING)
  DIM folder AS STRING
  folder = LEFT$(Copy, RINSTR(Copy, "/") - 1)
  ' (short lines: the Output pane is narrow; the link on its own)
  IF Copied = "again" THEN
    OutputLine TR$("Copied {file} again from the original, in:", "file", FileTitle(Copy))
  ELSEIF Copied = "made" THEN
    OutputLine TR$("Made your copy of the example {file} (the installed examples can't be changed), in:", "file", FileTitle(Copy))
  ELSE
    OutputLine TR$("Your copy of the example {file}, as you left it, in:", "file", FileTitle(Copy))
  END IF
  OutputLine "  " + folder
  OutputLine "  " + Hyperlink(TR$("Copy again from the original"), "example-again:")
END SUB

' File > Copy Example Again from the Original: the example's files (its
' program, includes, data) as RapidR has them, over the copy's — asked
' first (the changes made to them go; files added to the copy stay) —
' then the copy opened again.
SUB CopyExampleAgain
  DIM p AS STRING, name AS STRING, d AS INTEGER
  IF ExampleOriginal = "" OR ProjectOpen = 0 THEN EXIT SUB
  name = FileTitle(ExampleOriginal)
  IF AskCopyAgain(TR$("Copy {name} again from the original?", "name", name), TR$("Your changes to the example's own files, saved or not, are replaced by RapidR's. Files you added to its folder stay.")) = 0 THEN EXIT SUB
  p = Proj.CopyExampleAgain(ProjectFile, HomeDir + "examples")
  IF p = "" THEN
    OutputLine TR$("Can't copy {name} again: {error}", "name", name, "error", Proj.Error)
    SetState "error", TR$("Can't copy {name} again", "name", name)
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
  LeaveProjectInspector
  Proj.Close
  ExtProjectClosed
  Lang.Project = ""
  ProjectOpen = 0
  RefreshProgramLook 0
  ProjectName = ""
  ExampleOriginal = ""
  FillProjectTree
  ShowWelcome
  UpdateTitle
  UpdateCommands
  AIProjectChanged
  VersionsProjectChanged
END SUB

' File > Open: a project or a source file.
SUB AskOpen
  OpenDlg.Caption = TR$("Open Project or File")
  OpenDlg.Filter = TR$("RapidR projects and sources") + " (*.rrproj;*.rr;*.bas;*.rqw;*.rqb;*.rq;*.inc)|*.rrproj;*.rr;*.bas;*.rqw;*.rqb;*.rq;*.inc|" + TR$("All files") + "|*.*"
  IF OpenDlg.Execute THEN OpenProject OpenDlg.FileName
END SUB

' File > Open Folder: its project (.rrproj) or main source — on the web the
' folder's files read in, saved back to it (showDirectoryPicker).
SUB AskOpenFolder
  DIM f AS STRING
  OpenDlg.Caption = TR$("Open Folder")
  OpenDlg.PickFolder = 1
  IF OpenDlg.Execute THEN
    f = OpenDlg.FileName
    OpenDlg.PickFolder = 0
    IF Proj.OpenFolder(f) THEN
      IF Proj.FileName <> "" THEN OpenProject Proj.FileName ELSE OpenProject Proj.Folder + "/" + Proj.MainFile
    ELSE
      OutputLine TR$("Can't open {file}: {error}", "file", f, "error", Proj.Error)
      SetState "error", TR$("No RapidR project in {folder}", "folder", FileTitle(f))
    END IF
  END IF
  OpenDlg.PickFolder = 0
END SUB

' (the web) A project file (.rrproj) the browser gave Studio on its own —
' File > Open picks one file, and a page may read only the files it is
' given — so its main file isn't there to read: rather than the project
' with empty files, the project that was open stays, and Studio offers its
' folder (Open Folder...: the picker opens where the project file is, and
' the folder's files come in with it). True when it said so. The desktop
' reads a project's files from its folder: never.
FUNCTION ProjectFilesElsewhere(Path AS STRING) AS INTEGER
  DIM main AS STRING, back AS INTEGER
  IF Proj.FileManager <> "" OR Proj.Kind <> "project" THEN EXIT FUNCTION
  main = Proj.MainFile
  IF Proj.Folder <> "" THEN main = Proj.Folder + "/" + main
  IF main = "" OR FILEEXISTS(main) THEN EXIT FUNCTION
  Result = 1
  ' (the project that was open before, as it was)
  IF ProjectOpen THEN back = Proj.Open(ProjectFile) ELSE Proj.Close
  OutputLine TR$("{file}: the browser gave RapidR Studio this file only, not the project's files in its folder", "file", FileTitle(Path))
  IF AskOpenItsFolder(TR$("Open the folder {file} is in?", "file", FileTitle(Path)), TR$("The browser let RapidR Studio read only the file you picked. The project's files are in its folder.")) THEN
    AskOpenFolder
  ELSE
    OutputLine "  " + TR$("File > Open Folder... opens the project with its files")
    SetState "error", TR$("Open the folder {file} is in to open the project", "file", FileTitle(Path))
  END IF
END FUNCTION

' File > Import RapidQ Project or File / Folder (R-NAMES): a COPY of a
' RapidQ program (a .bas, .rqw, .rqb, .rq or .inc file and what it
' includes, or a folder of them) with RapidR's names, beside it
' (<name>-rapidr), each program proved to compile to the same bytecode as
' its original; the original is only read. The copy opens as a project,
' its report (what changed, line by line, and the proof) beside the code.
SUB AskImportRapidQ
  DIM i AS INTEGER, f AS STRING, pick AS STRING
  ImportDlg.Caption = TR$("Import RapidQ Project or File")
  ImportDlg.Filter = TR$("RapidQ programs") + " (*.bas;*.rqw;*.rqb;*.rq;*.inc)|*.bas;*.rqw;*.rqb;*.rq;*.inc|" + TR$("RapidR projects") + " (*.rrproj)|*.rrproj|" + TR$("All files") + "|*.*"
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
  OpenDlg.Caption = TR$("Import RapidQ Folder")
  OpenDlg.PickFolder = 1
  IF OpenDlg.Execute THEN f = OpenDlg.FileName
  OpenDlg.PickFolder = 0
  IF f <> "" THEN ImportRapidQ f
END SUB

SUB ImportRapidQ(Source AS STRING)
  DIM made AS STRING, report AS STRING, summary AS STRING, missing AS STRING, row AS STRING, n AS INTEGER, i AS INTEGER
  OutputLine TR$("Importing {source}: a copy with RapidR's names and every file it includes (the original is only read)", "source", Source)
  made = Proj.ImportRapidQ(Source)
  IF made = "" THEN
    OutputLine TR$("Can't import {source}: {error}", "source", Source, "error", Proj.Error)
    SetState "error", TR$("Can't import {file}", "file", FileTitle(Source))
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
    OutputLine TR$("{file}: include file not found: {include} - {what}", "file", FIELD$(row, CHR$(9), 1), "include", FIELD$(row, CHR$(9), 2), "what", FIELD$(row, CHR$(9), 3))
  NEXT
  OutputLine TR$("The copy: {path}", "path", made)
  OpenProject made
  OpenDocument report
  IF n > 0 THEN
    SetState "error", TRN$("Imported {file}, but {n} include file wasn't found: see Output", "Imported {file}, but {n} include files weren't found: see Output", n, "file", FileTitle(Source))
  ELSE
    SetState "stopped", TR$("Imported {file} as a copy with RapidR's names", "file", FileTitle(Source))
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
  SaveDlg.Caption = TR$("Where the new project goes")
  SaveDlg.FileName = NewName.Text + ".rrproj"
  SaveDlg.Filter = TR$("RapidR projects") + " (*.rrproj)|*.rrproj"
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
    SHOWMESSAGE TR$("Can't make the project: {error}", "error", Proj.Error)
  END IF
END SUB

SUB NewCancel
  NewDialog.Close
END SUB

SUB SaveProjectFile
  IF ProjectOpen = 0 THEN EXIT SUB
  IF Proj.Save THEN
    ProjectFile = Proj.FileName
    ' (Studio's own write: no change from outside — external.inc)
    ExtOwnWrite ProjectFile
    AddRecent ProjectFile
    OutputLine TR$("Saved {file}", "file", ProjectFile)
  ELSE
    OutputLine TR$("Can't save the project: {error}", "error", Proj.Error)
  END IF
END SUB

' (running the program and debugging it: debug.inc)

' ---- settings (the per-user store) -------------------------------------------------------

CONST STUDIO_KEY = "Software\RapidR\Studio"
' The saved layout's format (Studio 2026-10: documents as tabs, the debugger's
' and the toolbox's panes; 4: the Search tab beside Project, shown): an older
' one isn't restored
CONST LAYOUT_VERSION = 4
CONST HKCU = &H80000001

SUB LoadSettings
  Reg.RootKey = HKCU
  IF Reg.KeyExists(STUDIO_KEY) THEN
    Reg.OpenKey(STUDIO_KEY, 0)
    SettingsRead = 1
    IF ThemeChoice = "" AND Reg.ValueExists("Theme") THEN ThemeChoice = Reg.ReadString("Theme")
    ' (Studio's language: "" the system's — Settings > General > Language)
    IF Reg.ValueExists("Language") THEN LanguageChoice = Reg.ReadString("Language")
    IF Reg.ValueExists("PreviewClassic") THEN PreviewClassic = Flag(Reg.ReadString("PreviewClassic") = "1")
    ' (the Output and Problems pages wrap long lines unless turned off: View menu)
    IF Reg.ValueExists("OutputWrap") THEN OutputNoWrap = Flag(Reg.ReadString("OutputWrap") = "0")
    ' (a layout saved by an older Studio — before documents were tabs, or
    ' without a panel added since — isn't restored: Studio starts with its
    ' default layout; LAYOUT_VERSION goes up when the layout changes)
    IF Reg.ValueExists("Layout") AND Reg.ValueExists("LayoutVersion") THEN
      IF Reg.ReadString("LayoutVersion") = LTRIM$(STR$(LAYOUT_VERSION)) THEN SavedLayout = Reg.ReadString("Layout")
    END IF
    ' (S9: the debug panes' setting, each layout's sizes — debug.inc)
    IF Reg.ValueExists("DebugPanes") THEN DebugPanesMode = Reg.ReadString("DebugPanes")
    IF DebugPanesMode <> "always" AND DebugPanesMode <> "never" THEN DebugPanesMode = "auto"
    IF Reg.ValueExists("EditBottom") THEN EditBottom = VAL(Reg.ReadString("EditBottom"))
    IF Reg.ValueExists("DebugBottom") THEN DebugBottom = VAL(Reg.ReadString("DebugBottom"))
    IF Reg.ValueExists("DebugWidth") THEN DebugWidth = VAL(Reg.ReadString("DebugWidth"))
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
  Reg.WriteString("Language", LanguageChoice)
  Reg.WriteString("PreviewClassic", LTRIM$(STR$(PreviewClassic)))
  Reg.WriteString("OutputWrap", LTRIM$(STR$(Flag(OutputNoWrap = 0))))
  Reg.WriteString("Layout", Dock.SaveLayout)
  Reg.WriteString("LayoutVersion", LTRIM$(STR$(LAYOUT_VERSION)))
  ' (S9: the layout shown's sizes first)
  RememberLayoutSizes
  Reg.WriteString("DebugPanes", DebugPanesMode)
  Reg.WriteString("EditBottom", LTRIM$(STR$(EditBottom)))
  Reg.WriteString("DebugBottom", LTRIM$(STR$(DebugBottom)))
  Reg.WriteString("DebugWidth", LTRIM$(STR$(DebugWidth)))
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
    OutputLine TR$("{file} isn't there any more ({path}): taken off Open Recent", "file", FileTitle(p), "path", p)
    ' (short: the status bar's first part is narrow; the Output says where)
    SetState "error", TR$("{file} isn't there any more", "file", FileTitle(p))
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
  OutputLine TR$("Open Recent cleared")
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
    OutputLine TR$("The last project, {file}, isn't there any more ({path}): taken off Open Recent", "file", FileTitle(SessionProject), "path", SessionProject)
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
