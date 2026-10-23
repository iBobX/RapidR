' search.inc: Search (Ctrl+Shift+F, docs/studio-wow.md CMD-3, Preview 4's S3),
' Find References and Go to Definition's places, Back and Forward, the code
' editor's context menu, and the Help pane (F1, HLP-1).
'
' Search is a tab beside Project, a magnifier on it: what to find, where
' (this file, the open files, the modules, the forms, the resources, the
' whole project, a folder), the code editor's own search (RPROJECT's Find:
' match case, whole word, regular expressions) over each file — an open one
' as its editor has it, saved or not. The results by file, each line
' file:line:col with the match marked in it; Enter or a double click opens
' it there. Replace All writes the replacement into each file's editor (one
' undo step a file).
'
' Find References (Shift+F12) lists in the same tab every use of the name
' at the caret — a variable, a SUB or FUNCTION, a component — in every file
' of the program, as the language service knows them (its symbols, the
' include order: never a text search). Go to Definition (F12) jumps to
' where it is declared, in any file; Navigate Back (Alt+Left) returns, and
' Forward goes again.
'
' F1 opens the language's entry (RLANGUAGESERVICE's Help, from the registry
' that also gives the editor's hovers, the inspector's and the toolbox's
' docs) for what is under the caret: the word in the code (a member of a
' component when it follows "Name."), the inspector's selected row, the
' toolbox's component, the designer's selected component — with an Insert
' button that types its syntax into the code.

DIM SearchPane AS RPanel
DIM SearchEdit AS REdit
DIM ReplaceEdit AS REdit
DIM SearchCase AS RCheckBox
DIM SearchWord AS RCheckBox
DIM SearchRegex AS RCheckBox
DIM SearchGo AS RButton
DIM ReplaceGo AS RButton
DIM SearchScopeLabel AS RLabel
DIM SearchScope AS RComboBox
DIM SearchInfo AS RLabel
DIM SearchTree AS RTreeView
' (each result node's file and place; a file's node: line 0)
DIM HitFile(2000) AS STRING
DIM HitLine(2000) AS INTEGER
DIM HitCol(2000) AS INTEGER
DIM HitLen(2000) AS INTEGER
' (the folder "A folder…" searches; the scope before it was picked — a
' cancelled pick goes back to it)
DIM SearchFolder AS STRING
DIM SearchScopeWas AS INTEGER
' (what the tab shows: "find" — a search's results — or "refs" — Find
' References' places)
DIM SearchShows AS STRING

' The scopes, as the list shows them (SearchScope.ItemIndex)
CONST SCOPE_FILE = 0
CONST SCOPE_OPEN = 1
CONST SCOPE_MODULES = 2
CONST SCOPE_FORMS = 3
CONST SCOPE_RESOURCES = 4
CONST SCOPE_PROJECT = 5
CONST SCOPE_FOLDER = 6

' Navigate Back / Forward (Alt+Left / Alt+Right): the places Go to
' Definition, a search's result or a reference left from
DIM NavFile(40) AS STRING
DIM NavLine(40) AS INTEGER
DIM NavCol(40) AS INTEGER
DIM NavCount AS INTEGER
DIM FwdFile(40) AS STRING
DIM FwdLine(40) AS INTEGER
DIM FwdCol(40) AS INTEGER
DIM FwdCount AS INTEGER
' (where Go to Definition was asked from: kept until the jump — in the
' file, at once; to another file, when OnNavigate opens it)
DIM NavFromFile AS STRING
DIM NavFromLine AS INTEGER
DIM NavFromCol AS INTEGER
' (when it was asked: a jump in the same file comes as the caret's move
' just after — the editor answers between the program's turns)
DIM NavAskedAt AS DOUBLE

' The code editors' context menu: the language's places first, then the
' clipboard
DIM CodeMenu AS RPopupMenu
DIM CodeMenuDef AS RMenuItem
DIM CodeMenuRefs AS RMenuItem
DIM CodeMenuRename AS RMenuItem
DIM CodeMenuSep1 AS RMenuItem
DIM CodeMenuCut AS RMenuItem
DIM CodeMenuCopy AS RMenuItem
DIM CodeMenuPaste AS RMenuItem
DIM CodeMenuSep2 AS RMenuItem
DIM CodeMenuSelAll AS RMenuItem
DIM CodeMenuFind AS RMenuItem
' (each item's command, by its Tag)
DIM CodeMenuCmd(10) AS STRING
' (Find References' places, as the editor gave them)
DIM RefFile(400) AS STRING
DIM RefLine(400) AS INTEGER
DIM RefCol(400) AS INTEGER

DIM HelpPane AS RPanel
DIM HelpTitle AS RLabel
DIM HelpWhat AS RLabel
DIM HelpSyntax AS RMemo
DIM HelpInsert AS RButton
DIM HelpBody AS RMemo

DECLARE SUB SearchKey(Key AS INTEGER, Shift AS INTEGER)
DECLARE SUB SearchTreeKey(Key AS INTEGER, Shift AS INTEGER)
DECLARE SUB SearchClicked
DECLARE SUB ReplaceClicked
DECLARE SUB SearchOpen
DECLARE SUB SearchPeek
DECLARE SUB SearchScopeChanged
DECLARE SUB LayOutSearch
DECLARE SUB LayOutHelp
DECLARE SUB HelpInsertClicked
DECLARE SUB CodeMenuPicked(Sender AS RMenuItem)
DECLARE SUB CodeMenuOpening
DECLARE SUB NavPush(f AS STRING, L AS INTEGER, C AS INTEGER)
DECLARE SUB NavPushHere
DECLARE FUNCTION SearchLine(Path AS STRING, L AS INTEGER) AS STRING
DECLARE FUNCTION WordChar(c AS STRING) AS INTEGER

SUB SetUpSearch
  SearchPane.Parent = Studio
  SearchPane.BevelOuter = 0
  SearchPane.AccessibleName = "Search"
  SearchPane.OnResize = LayOutSearch
  SearchEdit.Parent = SearchPane
  SearchEdit.AccessibleName = "Search for"
  SearchEdit.Hint = "Search (Enter)"
  SearchEdit.OnKeyDown = SearchKey
  ReplaceEdit.Parent = SearchPane
  ReplaceEdit.AccessibleName = "Replace with"
  SearchCase.Parent = SearchPane: SearchCase.Caption = "Case"
  SearchCase.Hint = "Match case"
  SearchWord.Parent = SearchPane: SearchWord.Caption = "Word"
  SearchWord.Hint = "Whole words only"
  SearchRegex.Parent = SearchPane: SearchRegex.Caption = "Regex"
  SearchRegex.Hint = "A regular expression"
  SearchGo.Parent = SearchPane: SearchGo.Caption = "Find"
  SearchGo.OnClick = SearchClicked
  ReplaceGo.Parent = SearchPane: ReplaceGo.Caption = "Replace All"
  ReplaceGo.OnClick = ReplaceClicked
  SearchScopeLabel.Parent = SearchPane
  SearchScopeLabel.Caption = "In:"
  SearchScope.Parent = SearchPane
  SearchScope.AccessibleName = "Search in"
  SearchScope.AddItems "This file", "Open files", "Modules", "Forms", "Resources", "Whole project", "A folder..."
  SearchScope.ItemIndex = SCOPE_PROJECT
  SearchScopeWas = SCOPE_PROJECT
  SearchScope.OnChange = SearchScopeChanged
  SearchInfo.Parent = SearchPane
  SearchInfo.Caption = "Type what to find, then Enter."
  SearchTree.Parent = SearchPane
  SearchTree.AccessibleName = "Search results"
  ' (a click shows the place and keeps the keys here; Enter or a double
  ' click opens it, the caret on the match)
  SearchTree.OnClick = SearchPeek
  SearchTree.OnDblClick = SearchOpen
  SearchTree.OnKeyDown = SearchTreeKey
  SearchShows = "find"

  ' (the code editors' context menu: OpenDocument gives it to each)
  CodeMenuDef.Caption = "Go to &Definition": CodeMenuDef.ShortCut = "F12"
  CodeMenuRefs.Caption = "Find &References": CodeMenuRefs.ShortCut = "Shift+F12"
  CodeMenuRename.Caption = "Re&name Symbol...": CodeMenuRename.ShortCut = "F2"
  CodeMenuSep1.Caption = "-"
  CodeMenuCut.Caption = "Cu&t": CodeMenuCut.ShortCut = "Ctrl+X"
  CodeMenuCopy.Caption = "&Copy": CodeMenuCopy.ShortCut = "Ctrl+C"
  CodeMenuPaste.Caption = "&Paste": CodeMenuPaste.ShortCut = "Ctrl+V"
  CodeMenuSep2.Caption = "-"
  CodeMenuSelAll.Caption = "Select &All": CodeMenuSelAll.ShortCut = "Ctrl+A"
  CodeMenuFind.Caption = "Find in F&iles...": CodeMenuFind.ShortCut = "Ctrl+Shift+F"
  CodeMenuCmd(1) = "edit.goToDefinition": CodeMenuDef.Tag = 1
  CodeMenuCmd(2) = "edit.findReferences": CodeMenuRefs.Tag = 2
  CodeMenuCmd(3) = "edit.rename": CodeMenuRename.Tag = 3
  CodeMenuCmd(4) = "edit.cut": CodeMenuCut.Tag = 4
  CodeMenuCmd(5) = "edit.copy": CodeMenuCopy.Tag = 5
  CodeMenuCmd(6) = "edit.paste": CodeMenuPaste.Tag = 6
  CodeMenuCmd(7) = "edit.selectAll": CodeMenuSelAll.Tag = 7
  CodeMenuCmd(8) = "edit.findInFiles": CodeMenuFind.Tag = 8
  CodeMenuDef.OnClick = CodeMenuPicked
  CodeMenuRefs.OnClick = CodeMenuPicked
  CodeMenuRename.OnClick = CodeMenuPicked
  CodeMenuCut.OnClick = CodeMenuPicked
  CodeMenuCopy.OnClick = CodeMenuPicked
  CodeMenuPaste.OnClick = CodeMenuPicked
  CodeMenuSelAll.OnClick = CodeMenuPicked
  CodeMenuFind.OnClick = CodeMenuPicked
  CodeMenu.AddItems CodeMenuDef
  CodeMenu.AddItems CodeMenuRefs
  CodeMenu.AddItems CodeMenuRename
  CodeMenu.AddItems CodeMenuSep1
  CodeMenu.AddItems CodeMenuCut
  CodeMenu.AddItems CodeMenuCopy
  CodeMenu.AddItems CodeMenuPaste
  CodeMenu.AddItems CodeMenuSep2
  CodeMenu.AddItems CodeMenuSelAll
  CodeMenu.AddItems CodeMenuFind
  CodeMenu.OnPopup = CodeMenuOpening

  HelpPane.Parent = Studio
  HelpPane.BevelOuter = 0
  HelpPane.AccessibleName = "Help"
  HelpPane.OnResize = LayOutHelp
  HelpTitle.Parent = HelpPane
  HelpTitle.Caption = "Help"
  HelpWhat.Parent = HelpPane
  HelpWhat.WordWrap = 1
  HelpWhat.Caption = "Press F1 on a word in the code, a property, a toolbox item or a component on the form."
  HelpSyntax.Parent = HelpPane
  HelpSyntax.ReadOnly = 1
  HelpSyntax.WordWrap = 1
  HelpSyntax.AccessibleName = "Syntax"
  HelpInsert.Parent = HelpPane
  HelpInsert.Caption = "Insert"
  HelpInsert.Hint = "Type the syntax into the code at the caret"
  HelpInsert.OnClick = HelpInsertClicked
  HelpInsert.Enabled = 0
  HelpBody.Parent = HelpPane
  HelpBody.ReadOnly = 1
  HelpBody.WordWrap = 1
  HelpBody.ScrollBars = 2
  HelpBody.AccessibleName = "Help text"
  ' (Search: a tab beside Project, always there — Ctrl+Shift+F brings it
  ' to the front; Help docked hidden with the properties, F1 shows it)
  Dock.AddPane(SearchPane, "Search", "tab:projecttree", "search")
  Dock.AddPane(HelpPane, "Help", "tab:inspector", "help")
  Dock.HidePane("helppane")
  Dock.ShowPane("projecttree")
  Dock.ShowPane("inspector")
END SUB

SUB ApplySearchChrome
  HelpTitle.Font = SubFont
  HelpTitle.Font.Color = TC("page.heading")
  HelpWhat.Font.Color = TC("page.dim")
  HelpSyntax.Font = MonoFont
  SearchInfo.Font.Color = TC("page.dim")
  SearchScopeLabel.Font.Color = TC("page.text")
  SearchCase.Font.Color = TC("page.text")
  SearchWord.Font.Color = TC("page.text")
  SearchRegex.Font.Color = TC("page.text")
  SearchPane.Color = TC("page")
  HelpPane.Color = TC("page")
  ' (RapidR's looks: chevrons and indents, as the Project pane; the classic
  ' look keeps Windows' dotted lines)
  SearchTree.ShowLines = Flag(ThemeName = "classic")
END SUB

SUB LayOutSearch
  DIM w AS INTEGER, y AS INTEGER, fh AS INTEGER, bw AS INTEGER, lw AS INTEGER
  w = SearchPane.Width - 16
  IF w < 60 THEN EXIT SUB
  fh = 26: bw = 92
  IF ThemeName = "classic" THEN fh = 21: bw = 76
  y = 8
  SearchEdit.Left = 8: SearchEdit.Top = y: SearchEdit.Width = w - bw - 6: SearchEdit.Height = fh
  SearchGo.Left = 8 + w - bw: SearchGo.Top = y: SearchGo.Width = bw: SearchGo.Height = fh
  y = y + fh + 6
  ReplaceEdit.Left = 8: ReplaceEdit.Top = y: ReplaceEdit.Width = w - bw - 6: ReplaceEdit.Height = fh
  ReplaceGo.Left = 8 + w - bw: ReplaceGo.Top = y: ReplaceGo.Width = bw: ReplaceGo.Height = fh
  y = y + fh + 6
  ' (three options sharing the row)
  bw = w \ 3
  SearchCase.Left = 8: SearchCase.Top = y: SearchCase.Width = bw: SearchCase.Height = 20
  SearchWord.Left = 8 + bw: SearchWord.Top = y: SearchWord.Width = bw: SearchWord.Height = 20
  SearchRegex.Left = 8 + 2 * bw: SearchRegex.Top = y: SearchRegex.Width = w - 2 * bw: SearchRegex.Height = 20
  y = y + 26
  ' (where: "In:" and the scope's list)
  lw = 28
  SearchScopeLabel.Left = 8: SearchScopeLabel.Top = y + (fh - 16) \ 2: SearchScopeLabel.Width = lw: SearchScopeLabel.Height = 18
  SearchScope.Left = 8 + lw + 4: SearchScope.Top = y: SearchScope.Width = w - lw - 4
  y = y + fh + 6
  SearchInfo.Left = 8: SearchInfo.Top = y: SearchInfo.Width = w: SearchInfo.Height = 20
  y = y + 24
  SearchTree.Left = 0: SearchTree.Top = y: SearchTree.Width = SearchPane.Width
  SearchTree.Height = SearchPane.Height - y
END SUB

SUB LayOutHelp
  DIM w AS INTEGER, y AS INTEGER, bw AS INTEGER
  w = HelpPane.Width - 24
  IF w < 60 THEN EXIT SUB
  bw = 76
  HelpTitle.Left = 12: HelpTitle.Top = 10: HelpTitle.Width = w: HelpTitle.Height = 24
  HelpWhat.Left = 12: HelpWhat.Top = 38: HelpWhat.Width = w: HelpWhat.Height = 36
  y = 80
  HelpSyntax.Left = 12: HelpSyntax.Top = y: HelpSyntax.Width = w - bw - 8: HelpSyntax.Height = 52
  HelpInsert.Left = 12 + w - bw: HelpInsert.Top = y: HelpInsert.Width = bw: HelpInsert.Height = 28
  y = y + 60
  HelpBody.Left = 12: HelpBody.Top = y: HelpBody.Width = w
  HelpBody.Height = HelpPane.Height - y - 12
END SUB

' ---- Search ------------------------------------------------------------------------

FUNCTION SearchOptions AS STRING
  DIM o AS STRING
  IF SearchCase.Checked THEN o = o + "case,"
  IF SearchWord.Checked THEN o = o + "word,"
  IF SearchRegex.Checked THEN o = o + "regex,"
  Result = o
END FUNCTION

' (a file Search reads: not a picture, a sound, a video, a font, an archive,
' a program or a database)
FUNCTION SearchableFile(Path AS STRING) AS INTEGER
  SELECT CASE FileExt(Path)
    CASE "png", "jpg", "jpeg", "gif", "bmp", "ico", "icns", "webp", "tif", "tiff", "wav", "mp3", "ogg", "flac", "mid", "midi", "mp4", "mov", "avi", "webm"
      Result = 0
    CASE "ttf", "otf", "woff", "woff2", "zip", "gz", "tar", "7z", "exe", "dll", "dylib", "so", "pdf", "db", "sqlite", "parquet", "arrow", "xlsx", "rrb"
      Result = 0
    CASE ELSE
      Result = 1
  END SELECT
END FUNCTION

' (Path, a line, unless the list has it)
FUNCTION AddSearchFile(List AS STRING, Path AS STRING) AS STRING
  Result = List
  IF Path = "" THEN EXIT FUNCTION
  IF INSTR(LCASE$(CHR$(10) + List), LCASE$(CHR$(10) + Path + CHR$(10))) > 0 THEN EXIT FUNCTION
  Result = List + Path + CHR$(10)
END FUNCTION

' (n things: "1 file", "3 files")
FUNCTION Counted(n AS INTEGER, Thing AS STRING) AS STRING
  IF n = 1 THEN Result = "1 " + Thing ELSE Result = LTRIM$(STR$(n)) + " " + Thing + "s"
END FUNCTION

' (the open documents' files)
FUNCTION OpenSearchFiles AS STRING
  DIM d AS INTEGER, t AS STRING
  FOR d = 0 TO DocCount - 1
    IF DocOpen(d) THEN t = AddSearchFile(t, DocFile(d))
  NEXT
  Result = t
END FUNCTION

' Every file to search, one a line, as the scope says: this file, the open
' ones, the project's modules (and include files), its forms, its
' resources (the ones with text), all of it (and the open files), or a
' folder's.
FUNCTION SearchFiles AS STRING
  DIM i AS INTEGER, t AS STRING, k AS STRING, d AS INTEGER, scope AS INTEGER
  scope = SearchScope.ItemIndex
  ' (no project: its scopes are the open files)
  IF ProjectOpen = 0 AND scope >= SCOPE_MODULES AND scope <= SCOPE_PROJECT THEN scope = SCOPE_OPEN
  SELECT CASE scope
    CASE SCOPE_FILE
      d = ActiveCode
      IF d >= 0 THEN t = AddSearchFile(t, DocFile(d))
    CASE SCOPE_OPEN
      t = OpenSearchFiles
    CASE SCOPE_FOLDER
      IF SearchFolder <> "" THEN t = Proj.FilesIn(SearchFolder)
    CASE ELSE
      FOR i = 0 TO Proj.FileCount - 1
        k = Proj.FileKind(i)
        IF SearchableFile(Proj.File(i)) THEN
          IF scope = SCOPE_PROJECT THEN t = AddSearchFile(t, Proj.FullPath(i))
          IF scope = SCOPE_MODULES AND (k = "module" OR k = "include") THEN t = AddSearchFile(t, Proj.FullPath(i))
          IF scope = SCOPE_FORMS AND k = "form" THEN t = AddSearchFile(t, Proj.FullPath(i))
          IF scope = SCOPE_RESOURCES AND (k = "resource" OR k = "asset" OR k = "data") THEN t = AddSearchFile(t, Proj.FullPath(i))
        END IF
      NEXT
      IF scope = SCOPE_PROJECT THEN
        FOR d = 0 TO DocCount - 1
          IF DocOpen(d) THEN t = AddSearchFile(t, DocFile(d))
        NEXT
      END IF
  END SELECT
  Result = t
END FUNCTION

' (where the scope is, in words, for "No results … in …")
FUNCTION ScopeWords AS STRING
  SELECT CASE SearchScope.ItemIndex
    CASE SCOPE_FILE: Result = "this file"
    CASE SCOPE_OPEN: Result = "the open files"
    CASE SCOPE_MODULES: Result = "the modules"
    CASE SCOPE_FORMS: Result = "the forms"
    CASE SCOPE_RESOURCES: Result = "the resources"
    CASE SCOPE_FOLDER: Result = ShortPath(SearchFolder)
    CASE ELSE: Result = "the project"
  END SELECT
  IF ProjectOpen = 0 AND SearchScope.ItemIndex >= SCOPE_MODULES AND SearchScope.ItemIndex <= SCOPE_PROJECT THEN Result = "the open files"
END FUNCTION

' (a file's text: its editor's when it is open)
FUNCTION SearchText(Path AS STRING) AS STRING
  DIM d AS INTEGER
  d = DocOf(Path)
  IF d >= 0 THEN Result = CodeDoc(d).Text ELSE Result = Proj.FileText(Path)
END FUNCTION

' A result's row under its file's node: "line:col  the line" (the place
' file:line:col, the file the node above — the pane is narrow: a file's name
' in every row would hide the matches), the match marked in it (MarkStart /
' MarkLength); a long line shown from a little before the match ("…").
SUB PlaceRow(Node AS INTEGER, f AS STRING, LineNo AS INTEGER, ColNo AS INTEGER, Length AS INTEGER, Text AS STRING)
  DIM t AS STRING, lead AS INTEGER
  t = REPLACESUBSTR$(Text, CHR$(9), " ")
  lead = LEN(t) - LEN(LTRIM$(t))
  SearchTree.Item(Node).Text = RTRIM$(LTRIM$(t))
  SearchTree.Item(Node).Column = LTRIM$(STR$(LineNo)) + ":" + LTRIM$(STR$(ColNo))
  SearchTree.Item(Node).MarkStart = ColNo - 1 - lead
  SearchTree.Item(Node).MarkLength = Length
  HitFile(Node) = f
  HitLine(Node) = LineNo
  HitCol(Node) = ColNo
  HitLen(Node) = Length
END SUB

' (a path as the project shows it: from its folder — lib/units.inc —,
' else as it is)
FUNCTION ProjectRel(Path AS STRING) AS STRING
  DIM f AS STRING
  f = REPLACESUBSTR$(Proj.Folder, "\", "/")
  Result = REPLACESUBSTR$(Path, "\", "/")
  IF ProjectOpen AND f <> "" AND LCASE$(LEFT$(Result, LEN(f) + 1)) = LCASE$(f + "/") THEN Result = MID$(Result, LEN(f) + 2)
END FUNCTION

' (the icon the Project pane gives a file: a form's file its kind's, any
' other its type's — Item(i).Icon's and AddProblem's names)
FUNCTION FileIconName(Path AS STRING) AS STRING
  DIM i AS INTEGER, p AS STRING
  p = LCASE$(REPLACESUBSTR$(Path, "\", "/"))
  Result = "file:" + Path
  IF ProjectOpen = 0 THEN EXIT FUNCTION
  FOR i = 0 TO Proj.FileCount - 1
    IF LCASE$(REPLACESUBSTR$(Proj.FullPath(i), "\", "/")) = p THEN
      IF Proj.FileKind(i) = "form" THEN Result = "kind:form"
      EXIT FUNCTION
    END IF
  NEXT
END FUNCTION

' A file's group in the results: its icon, its name in bold, its folder
' dimmed, how many in a pill; its results under it.
SUB AddFileNode(f AS STRING, Count AS INTEGER)
  DIM node AS INTEGER, rel AS STRING, folder AS STRING
  SearchTree.AddItems FileTitle(f)
  node = SearchTree.ItemCount - 1
  rel = ProjectRel(f)
  IF RINSTR(rel, "/") > 0 THEN
    folder = LEFT$(rel, RINSTR(rel, "/"))
    IF LEFT$(folder, 1) = "/" OR MID$(folder, 2, 1) = ":" THEN folder = ShortPath(LEFT$(folder, LEN(folder) - 1)) + "/"
  END IF
  SearchTree.Item(node).Icon = FileIconName(f)
  SearchTree.Item(node).Bold = 1
  SearchTree.Item(node).Detail = folder
  SearchTree.Item(node).Badge = LTRIM$(STR$(Count))
  HitFile(node) = f: HitLine(node) = 0: HitCol(node) = 0: HitLen(node) = 0
END SUB

' The Search tab in front, laid out.
SUB ShowSearchTab
  Dock.ShowPane("searchpane")
  LayOutSearch
END SUB

' Searches every file of the scope for the pattern: the results by file
' (its name and how many), each match under it as file:line:col and its
' line, the match marked; Enter or a double click opens it there.
SUB FindInFiles(Pattern AS STRING)
  DIM files AS STRING, n AS INTEGER, i AS INTEGER, f AS STRING, hits AS STRING, m AS INTEGER, k AS INTEGER
  DIM row AS STRING, total AS INTEGER, nfiles AS INTEGER, node AS INTEGER, opts AS STRING, more AS INTEGER
  SearchTree.Clear
  SearchShows = "find"
  IF Pattern = "" THEN
    SearchInfo.Caption = "Type what to find, then Enter."
    EXIT SUB
  END IF
  IF SearchScope.ItemIndex = SCOPE_FOLDER AND SearchFolder = "" THEN
    SearchInfo.Caption = "Choose a folder: In, A folder..."
    EXIT SUB
  END IF
  opts = SearchOptions
  files = SearchFiles
  n = TALLY(files, CHR$(10))
  FOR i = 1 TO n
    f = FIELD$(files, CHR$(10), i)
    hits = Proj.Find(Pattern, opts, SearchText(f))
    IF Proj.Error <> "" THEN
      SearchInfo.Caption = "Not a regular expression: " + Proj.Error
      EXIT SUB
    END IF
    m = TALLY(hits, CHR$(10))
    IF m > 0 THEN
      IF SearchTree.ItemCount < 1900 THEN
        AddFileNode f, m
        node = SearchTree.ItemCount - 1
        FOR k = 1 TO m
          IF SearchTree.ItemCount < 1990 THEN
            row = FIELD$(hits, CHR$(10), k)
            SearchTree.AddChildItems node, ""
            PlaceRow SearchTree.ItemCount - 1, f, VAL(FIELD$(row, CHR$(9), 1)), VAL(FIELD$(row, CHR$(9), 2)), VAL(FIELD$(row, CHR$(9), 3)), FIELD$(row, CHR$(9), 4)
          ELSE
            more = 1
          END IF
        NEXT
      ELSE
        more = 1
      END IF
      total = total + m
      nfiles = nfiles + 1
    END IF
  NEXT
  SearchTree.FullExpand
  IF total = 0 THEN
    SearchInfo.Caption = "No results for " + CHR$(34) + Pattern + CHR$(34) + " in " + ScopeWords
  ELSE
    SearchInfo.Caption = Counted(total, "result") + " in " + Counted(nfiles, "file")
    IF more THEN SearchInfo.Caption = SearchInfo.Caption + " (the first 1,990 shown)"
  END IF
  Application.Announce SearchInfo.Caption
END SUB

SUB SearchKey(Key AS INTEGER, Shift AS INTEGER)
  IF Key = 13 THEN FindInFiles SearchEdit.Text
END SUB

SUB SearchClicked
  FindInFiles SearchEdit.Text
END SUB

' The scope changed: "A folder..." asks which (cancelled: the scope it was);
' the search again in the new scope.
SUB SearchScopeChanged
  IF SearchScope.ItemIndex = SCOPE_FOLDER THEN
    OpenDlg.Caption = "Search in a Folder"
    OpenDlg.PickFolder = 1
    IF OpenDlg.Execute THEN
      SearchFolder = OpenDlg.FileName
      SearchScope.Hint = SearchFolder
    ELSEIF SearchFolder = "" THEN
      SearchScope.ItemIndex = SearchScopeWas
    END IF
    OpenDlg.PickFolder = 0
  END IF
  SearchScopeWas = SearchScope.ItemIndex
  IF SearchShows = "find" AND SearchEdit.Text <> "" THEN FindInFiles SearchEdit.Text
END SUB

' Opens result i at its match (Focus: the editor takes the keys); a file's
' node opens the file.
SUB OpenResult(i AS INTEGER, Focus AS INTEGER)
  DIM d AS INTEGER
  IF i < 0 OR i > 2000 THEN EXIT SUB
  IF HitFile(i) = "" THEN EXIT SUB
  ' (opened: the place left kept for Back — a click only shows it)
  IF Focus THEN NavPushHere
  OpenDocument HitFile(i)
  d = DocOf(HitFile(i))
  IF d < 0 THEN EXIT SUB
  ' (a click: the keys stay in the results — a file just opened doesn't
  ' take them)
  IF Focus = 0 THEN EditorFocus.Enabled = 0
  IF Focus THEN Dock.FocusPane(DocPane(d)) ELSE Dock.ActiveDocument = DocPane(d)
  IF HitLine(i) > 0 THEN
    ' (the code editor's own line and column, from 1: no walk through
    ' the lines before)
    CodeDoc(d).GotoLineColumn HitLine(i), HitCol(i)
    CodeDoc(d).SelLength = HitLen(i)
  END IF
  ' (a click keeps the keys in the results: the next one with the arrows)
  IF Focus = 0 THEN SearchTree.SetFocus
END SUB

' A result double-clicked, or Enter on it: its file open at its line, the
' match selected, the caret there.
SUB SearchOpen
  OpenResult SearchTree.ItemIndex, 1
END SUB

' A result clicked: shown, the keys kept in the results.
SUB SearchPeek
  DIM i AS INTEGER
  i = SearchTree.ItemIndex
  IF i < 0 OR i > 2000 THEN EXIT SUB
  ' (a file's heading clicked: its results shown or hidden)
  IF HitLine(i) = 0 THEN
    SearchTree.Item(i).Expanded = NOT SearchTree.Item(i).Expanded
    EXIT SUB
  END IF
  OpenResult i, 0
END SUB

' Enter: a result opened; on a file's heading, its results shown or hidden
' (Left and Right do that too, Up and Down go through the results: the
' tree's own keys).
SUB SearchTreeKey(Key AS INTEGER, Shift AS INTEGER)
  DIM i AS INTEGER
  IF Key <> 13 THEN EXIT SUB
  i = SearchTree.ItemIndex
  IF i < 0 OR i > 2000 THEN EXIT SUB
  IF HitLine(i) = 0 THEN
    SearchTree.Item(i).Expanded = NOT SearchTree.Item(i).Expanded
  ELSE
    SearchOpen
  END IF
END SUB

' Replace All: each file with matches opened, its text replaced in its
' editor as one edit (one undo step a file), then searched again.
SUB ReplaceClicked
  DIM files AS STRING, n AS INTEGER, i AS INTEGER, f AS STRING, d AS INTEGER, t AS STRING, done AS INTEGER, opts AS STRING
  IF SearchEdit.Text = "" THEN EXIT SUB
  opts = SearchOptions
  files = SearchFiles
  n = TALLY(files, CHR$(10))
  FOR i = 1 TO n
    f = FIELD$(files, CHR$(10), i)
    IF Proj.Find(SearchEdit.Text, opts, SearchText(f)) <> "" THEN
      OpenDocument f
      d = DocOf(f)
      IF d >= 0 THEN
        t = Proj.Replace(SearchEdit.Text, opts, CodeDoc(d).Text, ReplaceEdit.Text)
        CodeDoc(d).SelStart = 0
        CodeDoc(d).SelLength = LEN(CodeDoc(d).Text)
        CodeDoc(d).SelText = t
        CodeChanged CodeDoc(d)
        done = done + 1
      END IF
    END IF
  NEXT
  FindInFiles SearchEdit.Text
  SearchInfo.Caption = "Replaced in " + LTRIM$(STR$(done)) + " files. " + SearchInfo.Caption
END SUB

' Ctrl+Shift+F: the Search tab, the selection (or the word at the caret)
' to find.
SUB ShowSearch
  DIM d AS INTEGER
  d = ActiveCode
  Dock.FocusPane("searchpane")
  LayOutSearch
  IF d >= 0 THEN
    IF CodeDoc(d).SelLength > 0 AND CodeDoc(d).SelLength < 200 THEN SearchEdit.Text = CodeDoc(d).SelText
  END IF
  SearchEdit.SetFocus
END SUB

' ---- Find References (Shift+F12) ----------------------------------------------------

' (the name at line L, column C of a file's text: the word there)
FUNCTION NameAt(Path AS STRING, L AS INTEGER, C AS INTEGER) AS STRING
  DIM t AS STRING, a AS INTEGER, b AS INTEGER
  t = SearchLine(Path, L)
  a = C
  b = a
  WHILE b <= LEN(t) AND WordChar(MID$(t, b, 1))
    b = b + 1
  WEND
  Result = MID$(t, a, b - a)
END FUNCTION

' (line L, from 1, of a file's text: its editor's when open)
FUNCTION SearchLine(Path AS STRING, L AS INTEGER) AS STRING
  DIM d AS INTEGER, t AS STRING, k AS INTEGER, p AS INTEGER, q AS INTEGER
  d = DocOf(Path)
  IF d >= 0 THEN
    Result = CodeDoc(d).Line(L - 1)
    EXIT FUNCTION
  END IF
  t = Proj.FileText(Path)
  p = 1
  FOR k = 2 TO L
    q = INSTR(p, t, CHR$(10))
    IF q = 0 THEN EXIT FUNCTION
    p = q + 1
  NEXT
  q = INSTR(p, t, CHR$(10))
  IF q = 0 THEN q = LEN(t) + 1
  Result = REPLACESUBSTR$(MID$(t, p, q - p), CHR$(13), "")
END FUNCTION

' Shift+F12: every use of the name at the caret, in every file of the
' program (the language service's references, as the editor gives them in
' Json: file, line, column), in the Search tab by file — "References to
' X: 7 in 3 files" —, the name marked in each line.
SUB ShowReferences(Json AS STRING, Name AS STRING)
  DIM rest AS STRING, item AS STRING, k AS INTEGER, n AS INTEGER, f AS STRING, l AS INTEGER, c AS INTEGER
  DIM nm AS STRING, node AS INTEGER, j AS INTEGER, nfiles AS INTEGER, last AS STRING, w AS INTEGER
  ' (the places, in the order given)
  rest = Json
  DO
    k = INSTR(rest, "{")
    IF k = 0 THEN EXIT DO
    rest = MID$(rest, k)
    k = INSTR(rest, "}")
    IF k = 0 THEN EXIT DO
    item = LEFT$(rest, k)
    rest = MID$(rest, k + 1)
    IF n < 400 THEN
      RefFile(n) = JsonField(item, "file")
      RefLine(n) = VAL(JsonField(item, "line"))
      RefCol(n) = VAL(JsonField(item, "column"))
      n = n + 1
    END IF
  LOOP
  ShowSearchTab
  SearchTree.Clear
  SearchShows = "refs"
  IF n = 0 THEN
    SearchInfo.Caption = "No references: put the caret on a name of the program (a variable, a SUB, a component)."
    Application.Announce SearchInfo.Caption
    EXIT SUB
  END IF
  ' (the name asked about: the word at the caret, else the first use's)
  nm = Name
  IF nm = "" THEN nm = NameAt(RefFile(0), RefLine(0), RefCol(0))
  ' (grouped by file, in the order the files come)
  FOR j = 0 TO n - 1
    IF INSTR(CHR$(10) + last, CHR$(10) + RefFile(j) + CHR$(10)) = 0 THEN
      last = last + RefFile(j) + CHR$(10)
      nfiles = nfiles + 1
    END IF
  NEXT
  FOR k = 1 TO nfiles
    f = FIELD$(last, CHR$(10), k)
    c = 0
    FOR j = 0 TO n - 1
      IF RefFile(j) = f THEN c = c + 1
    NEXT
    AddFileNode f, c
    node = SearchTree.ItemCount - 1
    FOR j = 0 TO n - 1
      IF RefFile(j) = f AND SearchTree.ItemCount < 1990 THEN
        SearchTree.AddChildItems node, ""
        ' (each use marked as long as it is written there: a FUNCTION's
        ' Result is a use of it too)
        w = LEN(NameAt(f, RefLine(j), RefCol(j)))
        IF w = 0 THEN w = LEN(nm)
        PlaceRow SearchTree.ItemCount - 1, f, RefLine(j), RefCol(j), w, SearchLine(f, RefLine(j))
      END IF
    NEXT
  NEXT
  SearchTree.FullExpand
  SearchInfo.Caption = "References to " + nm + ": " + LTRIM$(STR$(n)) + " in " + Counted(nfiles, "file")
  Application.Announce SearchInfo.Caption
END SUB

' ---- Go to Definition (F12), Navigate Back / Forward (Alt+Left / Alt+Right) -------------

' (a place kept: the active code document's caret)
SUB NavPushHere
  DIM d AS INTEGER
  d = ActiveCode
  IF d < 0 THEN EXIT SUB
  NavPush DocFile(d), CodeDoc(d).CaretLine, CodeDoc(d).CaretColumn
END SUB

' (a place to go back to: the newest last, the oldest dropped when full; a
' new place ends what Forward had)
SUB NavPush(f AS STRING, L AS INTEGER, C AS INTEGER)
  DIM i AS INTEGER
  IF f = "" THEN EXIT SUB
  IF NavCount > 0 THEN
    IF NavFile(NavCount - 1) = f AND NavLine(NavCount - 1) = L THEN EXIT SUB
  END IF
  IF NavCount >= 40 THEN
    FOR i = 1 TO 39
      NavFile(i - 1) = NavFile(i): NavLine(i - 1) = NavLine(i): NavCol(i - 1) = NavCol(i)
    NEXT
    NavCount = 39
  END IF
  NavFile(NavCount) = f: NavLine(NavCount) = L: NavCol(NavCount) = C
  NavCount = NavCount + 1
  FwdCount = 0
END SUB

' (a file open at line L, column C — both from 1 —, the keys in its code)
SUB NavGo(f AS STRING, L AS INTEGER, C AS INTEGER)
  DIM d AS INTEGER
  d = CodeDocOf(f)
  IF d < 0 THEN EXIT SUB
  Dock.FocusPane(DocPane(d))
  CodeDoc(d).GotoLineColumn L, C
END SUB

' Go to Definition (F12, Edit, the code's context menu): where the name at
' the caret is declared — in this file at once, in another through
' OnNavigate (CodeNavigate) — the place left kept for Back.
SUB GoToDefinition(d AS INTEGER)
  DIM l AS INTEGER, c AS INTEGER
  IF d < 0 THEN EXIT SUB
  l = CodeDoc(d).CaretLine: c = CodeDoc(d).CaretColumn
  NavFromFile = DocFile(d): NavFromLine = l: NavFromCol = c
  NavAskedAt = TIMER
  ' (the editor answers after this turn: in this file the caret moves —
  ' NavCaretMoved —, to another file OnNavigate comes — NavigateTo)
  CodeDoc(d).GotoDefinition
END SUB

' The caret moved (the active code's, line and column from 1): just after
' Go to Definition, to another line of the same file — its jump. The place
' it left is kept for Back.
SUB NavCaretMoved(L AS INTEGER, C AS INTEGER)
  DIM d AS INTEGER
  IF NavFromFile = "" THEN EXIT SUB
  IF ABS(TIMER - NavAskedAt) > 2 THEN
    NavFromFile = ""
    EXIT SUB
  END IF
  d = ActiveCode
  IF d < 0 THEN EXIT SUB
  IF DocFile(d) = NavFromFile AND L <> NavFromLine THEN
    NavPush NavFromFile, NavFromLine, NavFromCol
    NavFromFile = ""
  END IF
END SUB

' F12 in another file (OnNavigate): the place it was asked from kept for
' Back, then the file open there.
SUB NavigateTo(File AS STRING, LineNo AS INTEGER, ColNo AS INTEGER, Sender AS RCodeEditor)
  IF NavFromFile <> "" THEN
    NavPush NavFromFile, NavFromLine, NavFromCol
    NavFromFile = ""
  ELSE
    ' (Ctrl+click: where the click was)
    NavPush DocFile(Sender.Tag), Sender.CaretLine, Sender.CaretColumn
  END IF
  NavGo File, LineNo, ColNo
END SUB

' Alt+Left: the place before the last jump (the place now kept for Forward).
SUB NavBack
  DIM d AS INTEGER
  IF NavCount = 0 THEN
    SetState "stopped", "Nothing to go back to"
    EXIT SUB
  END IF
  d = ActiveCode
  IF d >= 0 AND FwdCount < 40 THEN
    FwdFile(FwdCount) = DocFile(d): FwdLine(FwdCount) = CodeDoc(d).CaretLine: FwdCol(FwdCount) = CodeDoc(d).CaretColumn
    FwdCount = FwdCount + 1
  END IF
  NavCount = NavCount - 1
  NavGo NavFile(NavCount), NavLine(NavCount), NavCol(NavCount)
END SUB

' Alt+Right: the place Back left.
SUB NavForward
  DIM d AS INTEGER, keep AS INTEGER
  IF FwdCount = 0 THEN
    SetState "stopped", "Nothing to go forward to"
    EXIT SUB
  END IF
  d = ActiveCode
  FwdCount = FwdCount - 1
  keep = FwdCount
  IF d >= 0 THEN NavPush DocFile(d), CodeDoc(d).CaretLine, CodeDoc(d).CaretColumn
  FwdCount = keep
  NavGo FwdFile(FwdCount), FwdLine(FwdCount), FwdCol(FwdCount)
END SUB

' ---- the code editors' context menu ---------------------------------------------------

' (what can be done now: the clipboard's items as the selection allows)
SUB CodeMenuOpening
  DIM d AS INTEGER
  d = ActiveCode
  IF d < 0 THEN EXIT SUB
  CodeMenuCut.Enabled = Flag(CodeDoc(d).SelLength > 0 AND CodeDoc(d).ReadOnly = 0)
  CodeMenuCopy.Enabled = Flag(CodeDoc(d).SelLength > 0)
  CodeMenuPaste.Enabled = Flag(CodeDoc(d).ReadOnly = 0)
  CodeMenuRename.Enabled = Flag(CodeDoc(d).ReadOnly = 0)
END SUB

' (document d's code editor gets it: OpenDocument)
SUB CodeMenuFor(d AS INTEGER)
  CodeDoc(d).PopupMenu = CodeMenu
END SUB

SUB CodeMenuPicked(Sender AS RMenuItem)
  IF Sender.Tag >= 1 AND Sender.Tag <= 8 THEN RunCommand CodeMenuCmd(Sender.Tag)
END SUB

' ---- Help (F1) ----------------------------------------------------------------------

DIM HelpSyntaxText AS STRING
DIM CaretOfType AS STRING

' (a word's character)
FUNCTION WordChar(c AS STRING) AS INTEGER
  Result = Flag((c >= "A" AND c <= "Z") OR (c >= "a" AND c <= "z") OR (c >= "0" AND c <= "9") OR c = "_" OR c = "$")
END FUNCTION

' The word at the caret of document d's code and, when it follows "Name.",
' Name's type (from the outline's components) in CaretOfType.
FUNCTION CaretWord(d AS INTEGER) AS STRING
  DIM t AS STRING, p AS INTEGER, a AS INTEGER, b AS INTEGER, obj AS STRING, o AS STRING, n AS INTEGER, i AS INTEGER, row AS STRING, OfType AS STRING
  CaretOfType = ""
  t = CodeDoc(d).Text
  p = CodeDoc(d).SelStart + 1
  a = p: b = p
  WHILE a > 1 AND WordChar(MID$(t, a - 1, 1))
    a = a - 1
  WEND
  WHILE b <= LEN(t) AND WordChar(MID$(t, b, 1))
    b = b + 1
  WEND
  Result = MID$(t, a, b - a)
  IF a > 2 AND MID$(t, a - 1, 1) = "." THEN
    i = a - 1
    WHILE i > 1 AND WordChar(MID$(t, i - 1, 1))
      i = i - 1
    WEND
    obj = LCASE$(MID$(t, i, a - 1 - i))
    ' (Name's type: the outline's component line "Name … QTYPE")
    o = Lang.Outline(DocFile(d))
    n = TALLY(o, CHR$(10))
    FOR i = 1 TO n
      row = FIELD$(o, CHR$(10), i)
      IF FIELD$(row, CHR$(9), 2) = "component" AND LCASE$(FIELD$(row, CHR$(9), 3)) = obj THEN
        OfType = UCASE$(FIELD$(row, CHR$(9), 4))
        IF INSTR(OfType, " ") > 0 THEN OfType = MID$(OfType, RINSTR(OfType, " ") + 1)
      END IF
    NEXT
    IF OfType = "" THEN OfType = obj
    CaretOfType = OfType
  END IF
END FUNCTION

' Shows the language's entry for Word (a member of OfType when given).
SUB ShowHelp(Word AS STRING, OfType AS STRING)
  DIM t AS STRING, n AS INTEGER, i AS INTEGER, body AS STRING
  Dock.ShowPane("helppane")
  LayOutHelp
  t = Lang.Help(Word, OfType)
  IF t = "" AND OfType <> "" THEN t = Lang.Help(Word, "")
  IF t = "" THEN
    HelpTitle.Caption = Word
    HelpWhat.Caption = "Nothing in the language is called " + CHR$(34) + Word + CHR$(34) + "."
    HelpSyntax.Text = ""
    HelpBody.Text = ""
    HelpSyntaxText = ""
    HelpInsert.Enabled = 0
    EXIT SUB
  END IF
  HelpTitle.Caption = FIELD$(t, CHR$(10), 1)
  HelpSyntaxText = REPLACESUBSTR$(FIELD$(t, CHR$(10), 2), CHR$(13), CHR$(10))
  HelpSyntax.Text = HelpSyntaxText
  HelpWhat.Caption = FIELD$(t, CHR$(10), 3)
  n = TALLY(t, CHR$(10)) + 1
  FOR i = 4 TO n
    body = body + FIELD$(t, CHR$(10), i) + CHR$(13) + CHR$(10) + CHR$(13) + CHR$(10)
  NEXT
  HelpBody.Text = body
  HelpInsert.Enabled = Flag(ActiveCode >= 0)
END SUB

' F1: help for what is under the caret or selected where the focus is.
SUB HelpForFocus
  DIM pane AS STRING, d AS INTEGER, w AS STRING, of AS STRING, sel AS INTEGER
  pane = LCASE$(Dock.ActivePane)
  d = ActiveCode
  IF pane = "inspector" AND Inspector.Selected <> "" THEN
    ShowHelp Inspector.Selected, Inspector.TargetType
    EXIT SUB
  END IF
  IF pane = "toolbox" AND Toolbox.Selected <> "" THEN
    ShowHelp Toolbox.Selected, ""
    EXIT SUB
  END IF
  IF d >= 0 THEN
    IF DocKind(d) = "design" THEN
      ' (the designer's selected component's type, else the form's)
      sel = DesignDoc(d).SelIndex
      w = Inspector.TargetType
      IF w <> "" THEN
        ShowHelp w, ""
        EXIT SUB
      END IF
    END IF
    w = CaretWord(d)
    of = CaretOfType
    IF w <> "" THEN
      ShowHelp w, of
      EXIT SUB
    END IF
  END IF
  ShowHelp "RapidR", ""
END SUB

SUB HelpInsertClicked
  DIM d AS INTEGER
  d = ActiveCode
  IF d < 0 OR HelpSyntaxText = "" THEN EXIT SUB
  Dock.FocusPane(DocPane(d))
  CodeDoc(d).SelText = HelpSyntaxText
  CodeChanged CodeDoc(d)
END SUB

' The search and help commands; True when Id was one.
FUNCTION SearchCommand(Id AS STRING) AS INTEGER
  Result = 1
  SELECT CASE Id
    CASE "edit.findInFiles": ShowSearch
    CASE "edit.goToDefinition": GoToDefinition ActiveCode
    CASE "nav.back": NavBack
    CASE "nav.forward": NavForward
    CASE "help.contents": HelpForFocus
    CASE ELSE
      IF LEFT$(Id, 5) = "find:" THEN
        ' (a test step: find:<text> — Find in Files for it)
        ShowSearchTab
        SearchEdit.Text = MID$(Id, 6)
        FindInFiles MID$(Id, 6)
      ELSEIF LEFT$(Id, 6) = "scope:" THEN
        ' (a test step: scope:<n> — the scope list's item n, as picked)
        SearchScope.ItemIndex = VAL(MID$(Id, 7))
        SearchScopeChanged
      ELSEIF LEFT$(Id, 7) = "folder:" THEN
        ' (a test step: folder:<path> — "A folder..." with that folder chosen)
        SearchFolder = MID$(Id, 8)
        SearchScope.ItemIndex = SCOPE_FOLDER
        SearchScopeWas = SCOPE_FOLDER
        IF SearchEdit.Text <> "" THEN FindInFiles SearchEdit.Text
      ELSEIF LEFT$(Id, 6) = "caret:" THEN
        ' (a test step: caret:<text> — the caret just after the first <text>
        ' of the active code)
        IF ActiveCode >= 0 THEN
          IF INSTR(CodeDoc(ActiveCode).Text, MID$(Id, 7)) > 0 THEN
            CodeDoc(ActiveCode).SelStart = INSTR(CodeDoc(ActiveCode).Text, MID$(Id, 7)) - 1 + LEN(MID$(Id, 7))
            CodeDoc(ActiveCode).SelLength = 0
          END IF
        END IF
      ELSEIF LEFT$(Id, 8) = "targets:" THEN
        ' (a test step: targets:<desktop|web|both> — Project Options' Targets, as its OK sets it)
        Proj.Targets = MID$(Id, 9)
        Lang.Targets = Proj.Targets
        AnalyzeActive
      ELSEIF LEFT$(Id, 7) = "result:" THEN
        ' (a test step: result:<n> — Enter on the results' node n)
        SearchTree.ItemIndex = VAL(MID$(Id, 8))
        SearchOpen
      ELSEIF LEFT$(Id, 5) = "help:" THEN
        ' (help:<word>[@<type>])
        IF INSTR(Id, "@") > 0 THEN
          ShowHelp MID$(Id, 6, INSTR(Id, "@") - 6), MID$(Id, INSTR(Id, "@") + 1)
        ELSE
          ShowHelp MID$(Id, 6), ""
        END IF
      ELSE
        Result = 0
      END IF
  END SELECT
END FUNCTION
