' search.inc: Find in Files (Ctrl+Shift+F, docs/studio-wow.md CMD-3) and
' the Help pane (F1, HLP-1).
'
' Find in Files searches every file of the project — an open one as its
' editor has it, saved or not — with the code editor's own search (RPROJECT's
' Find: match case, whole word, regular expressions); the results by file,
' each line a link to its place; Replace All writes the replacement into
' each file's editor (one undo step a file).
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
DIM SearchInfo AS RLabel
DIM SearchTree AS RTreeView
' (each result node's file and line; a file's node: line 0)
DIM HitFile(2000) AS STRING
DIM HitLine(2000) AS INTEGER
DIM HitCol(2000) AS INTEGER
DIM HitLen(2000) AS INTEGER

DIM HelpPane AS RPanel
DIM HelpTitle AS RLabel
DIM HelpWhat AS RLabel
DIM HelpSyntax AS RMemo
DIM HelpInsert AS RButton
DIM HelpBody AS RMemo

DECLARE SUB SearchKey(Key AS INTEGER, Shift AS INTEGER)
DECLARE SUB SearchClicked
DECLARE SUB ReplaceClicked
DECLARE SUB SearchOpen
DECLARE SUB LayOutSearch
DECLARE SUB LayOutHelp
DECLARE SUB HelpInsertClicked

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
  SearchInfo.Parent = SearchPane
  SearchInfo.Caption = "Type what to find in the project's files, then Enter."
  SearchTree.Parent = SearchPane
  SearchTree.AccessibleName = "Search results"
  SearchTree.OnClick = SearchOpen
  SearchTree.OnDblClick = SearchOpen

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
  ' (docked hidden: Search with the project, Help with the properties;
  ' Ctrl+Shift+F and F1 show them)
  Dock.AddPane(SearchPane, "Search", "tab:projecttree", "search")
  Dock.AddPane(HelpPane, "Help", "tab:inspector", "help")
  Dock.HidePane("searchpane")
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
  SearchCase.Font.Color = TC("page.text")
  SearchWord.Font.Color = TC("page.text")
  SearchRegex.Font.Color = TC("page.text")
  SearchPane.Color = TC("page")
  HelpPane.Color = TC("page")
END SUB

SUB LayOutSearch
  DIM w AS INTEGER, y AS INTEGER, fh AS INTEGER, bw AS INTEGER
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

' ---- Find in Files ---------------------------------------------------------------

FUNCTION SearchOptions AS STRING
  DIM o AS STRING
  IF SearchCase.Checked THEN o = o + "case,"
  IF SearchWord.Checked THEN o = o + "word,"
  IF SearchRegex.Checked THEN o = o + "regex,"
  Result = o
END FUNCTION

' (every file to search, one a line: the project's, else the open documents')
FUNCTION SearchFiles AS STRING
  DIM i AS INTEGER, d AS INTEGER, t AS STRING
  FOR i = 0 TO Proj.FileCount - 1
    t = t + Proj.FullPath(i) + CHR$(10)
  NEXT
  FOR d = 0 TO DocCount - 1
    IF DocOpen(d) AND INSTR(LCASE$(CHR$(10) + t), LCASE$(CHR$(10) + DocFile(d) + CHR$(10))) = 0 THEN t = t + DocFile(d) + CHR$(10)
  NEXT
  Result = t
END FUNCTION

' (a file's text: its editor's when it is open)
FUNCTION SearchText(Path AS STRING) AS STRING
  DIM d AS INTEGER
  d = DocOf(Path)
  IF d >= 0 THEN Result = CodeDoc(d).Text ELSE Result = Proj.FileText(Path)
END FUNCTION

' Searches every file for the pattern: the results by file (its name and
' how many), each line under it ("12: the line"), a click opens it there.
SUB FindInFiles(Pattern AS STRING)
  DIM files AS STRING, n AS INTEGER, i AS INTEGER, f AS STRING, hits AS STRING, m AS INTEGER, k AS INTEGER
  DIM row AS STRING, total AS INTEGER, nfiles AS INTEGER, node AS INTEGER, opts AS STRING
  SearchTree.Clear
  IF Pattern = "" THEN
    SearchInfo.Caption = "Type what to find in the project's files, then Enter."
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
    IF m > 0 AND SearchTree.ItemCount < 1900 THEN
      SearchTree.AddItems FileTitle(f) + "  (" + LTRIM$(STR$(m)) + ")"
      node = SearchTree.ItemCount - 1
      HitFile(node) = f: HitLine(node) = 0
      FOR k = 1 TO m
        IF SearchTree.ItemCount < 1990 THEN
          row = FIELD$(hits, CHR$(10), k)
          SearchTree.AddChildItems node, FIELD$(row, CHR$(9), 1) + ":  " + LTRIM$(FIELD$(row, CHR$(9), 4))
          HitFile(SearchTree.ItemCount - 1) = f
          HitLine(SearchTree.ItemCount - 1) = VAL(FIELD$(row, CHR$(9), 1))
          HitCol(SearchTree.ItemCount - 1) = VAL(FIELD$(row, CHR$(9), 2))
          HitLen(SearchTree.ItemCount - 1) = VAL(FIELD$(row, CHR$(9), 3))
        END IF
      NEXT
      total = total + m
      nfiles = nfiles + 1
    END IF
  NEXT
  SearchTree.FullExpand
  IF total = 0 THEN
    SearchInfo.Caption = "No results for " + CHR$(34) + Pattern + CHR$(34) + " in " + LTRIM$(STR$(n)) + " files"
  ELSE
    SearchInfo.Caption = LTRIM$(STR$(total)) + " results in " + LTRIM$(STR$(nfiles)) + " files"
  END IF
END SUB

SUB SearchKey(Key AS INTEGER, Shift AS INTEGER)
  IF Key = 13 THEN FindInFiles SearchEdit.Text
END SUB

SUB SearchClicked
  FindInFiles SearchEdit.Text
END SUB

' A result clicked: its file open at its line, the match selected.
SUB SearchOpen
  DIM i AS INTEGER, d AS INTEGER
  i = SearchTree.ItemIndex
  IF i < 0 OR i > 2000 THEN EXIT SUB
  IF HitFile(i) = "" THEN EXIT SUB
  OpenDocument HitFile(i)
  d = DocOf(HitFile(i))
  IF d < 0 THEN EXIT SUB
  Dock.FocusPane(DocPane(d))
  IF HitLine(i) > 0 THEN
    ' (the code editor's own line and column, from 1: no walk through
    ' the lines before)
    CodeDoc(d).GotoLineColumn HitLine(i), HitCol(i)
    CodeDoc(d).SelLength = HitLen(i)
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

' Ctrl+Shift+F: the Search pane, the selection (or the word at the caret)
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
    CASE "help.contents": HelpForFocus
    CASE ELSE
      IF LEFT$(Id, 5) = "find:" THEN
        ' (a test step: find:<text> — Find in Files for it)
        Dock.ShowPane("searchpane")
        LayOutSearch
        SearchEdit.Text = MID$(Id, 6)
        FindInFiles MID$(Id, 6)
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
