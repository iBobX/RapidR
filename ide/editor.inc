' editor.inc: the code documents' editor (RCODEEDITOR on rapidr-editor and
' RapidR's language service, docs/ide-plan.md I2 / I3, lane S-EDITOR) —
' its Edit commands, settings, the status bar's facts, and the scripted
' flows' typing steps.

' ---- Edit commands on the active code document -------------------------------------
' True when Id is one of the editor's commands (and it ran).
FUNCTION EditorCommand(Id AS STRING) AS INTEGER
  DIM d AS INTEGER
  d = ActiveCode
  Result = 1
  SELECT CASE Id
    CASE "edit.undo": IF d >= 0 THEN CodeDoc(d).Undo
    CASE "edit.redo": IF d >= 0 THEN CodeDoc(d).Redo
    CASE "edit.delete": IF d >= 0 THEN CodeDoc(d).ClearSelection
    CASE "edit.find": IF d >= 0 THEN CodeDoc(d).OpenFind "find"
    CASE "edit.replace": IF d >= 0 THEN CodeDoc(d).OpenFind "replace"
    CASE "edit.toggleComment": IF d >= 0 THEN CodeDoc(d).ToggleComment
    CASE "edit.format": IF d >= 0 THEN CodeDoc(d).FormatDocument
    CASE "edit.goToDefinition": IF d >= 0 THEN CodeDoc(d).GotoDefinition
    CASE "edit.findReferences": IF d >= 0 THEN CodeDoc(d).FindReferences
    CASE "edit.rename": IF d >= 0 THEN CodeDoc(d).OpenFind "rename"
    CASE "edit.triggerCompletion": IF d >= 0 THEN CodeDoc(d).TriggerCompletion
    CASE "edit.quickFix": IF d >= 0 THEN CodeDoc(d).QuickFix
    CASE "edit.findNext": IF d >= 0 THEN CodeDoc(d).FindNext
    CASE "edit.findPrevious": IF d >= 0 THEN CodeDoc(d).FindPrevious
    CASE "edit.indent": IF d >= 0 THEN CodeDoc(d).Indent
    CASE "edit.outdent": IF d >= 0 THEN CodeDoc(d).Outdent
    CASE "edit.selectNextOccurrence": IF d >= 0 THEN CodeDoc(d).SelectNextOccurrence
    CASE "edit.goToBracket": IF d >= 0 THEN CodeDoc(d).GotoMatchingBracket
    CASE "edit.fold": IF d >= 0 THEN CodeDoc(d).Fold CodeDoc(d).CaretLine
    CASE "edit.unfold": IF d >= 0 THEN CodeDoc(d).Unfold CodeDoc(d).CaretLine
    CASE "edit.foldAll": IF d >= 0 THEN CodeDoc(d).FoldAll
    CASE "edit.unfoldAll": IF d >= 0 THEN CodeDoc(d).UnfoldAll
    CASE "edit.parameterInfo": IF d >= 0 THEN CodeDoc(d).TriggerSignature
    CASE "edit.showHover": IF d >= 0 THEN CodeDoc(d).TriggerHover
    CASE "edit.wordWrap": IF d >= 0 THEN CodeDoc(d).WordWrap = NOT CodeDoc(d).WordWrap
    CASE ELSE: Result = 0
  END SELECT
END FUNCTION

' ---- the editor's settings (Studio's: HKCU\Software\RapidR\Studio) ----------------------
' KeywordCase: upper (DIM x AS INTEGER, the default), lower, proper or preserve;
' IdentifierCase: declaration (the program's names as declared, VB's — the
' default) or preserve. VS Code's rapidr.keywordCase / rapidr.identifierCase.
DIM EditorKeywordCase AS STRING
DIM EditorIdentifierCase AS STRING

SUB EditorSettings(d AS INTEGER)
  IF EditorKeywordCase = "" THEN
    EditorKeywordCase = "upper": EditorIdentifierCase = "declaration"
    Reg.RootKey = HKCU
    IF Reg.KeyExists(STUDIO_KEY) THEN
      Reg.OpenKey(STUDIO_KEY, 0)
      IF Reg.ValueExists("KeywordCase") THEN EditorKeywordCase = Reg.ReadString("KeywordCase")
      IF Reg.ValueExists("IdentifierCase") THEN EditorIdentifierCase = Reg.ReadString("IdentifierCase")
      Reg.CloseKey
    END IF
  END IF
  CodeDoc(d).KeywordCase = EditorKeywordCase
  CodeDoc(d).IdentifierCase = EditorIdentifierCase
END SUB

' ---- the status bar: the caret, the file's encoding and line breaks ---------------------
SUB CodeCaretMoved(Line AS INTEGER, Column AS INTEGER)
  Status.Panel(2).Caption = "Ln " + LTRIM$(STR$(Line)) + ", Col " + LTRIM$(STR$(Column))
END SUB

' (document d's facts in the status bar: when it opens or comes to the front)
SUB EditorFacts(d AS INTEGER)
  IF d < 0 THEN EXIT SUB
  Status.Panel(2).Caption = "Ln " + LTRIM$(STR$(CodeDoc(d).CaretLine)) + ", Col " + LTRIM$(STR$(CodeDoc(d).CaretColumn))
  Status.Panel(3).Caption = CodeDoc(d).Encoding
  Status.Panel(4).Caption = CodeDoc(d).LineEnding
END SUB

' (the caret into document d's code once its pane is in the window — a
' pane added this turn isn't laid out yet)
' (and again a turn later: a project tree's name editor that opened the
' file — Project > Add Form — gives the focus back to the tree as it closes)
DIM FocusDoc AS INTEGER
DIM FocusTurns AS INTEGER

SUB FocusCodeSoon(d AS INTEGER)
  FocusDoc = d
  FocusTurns = 2
  EditorFocus.Enabled = 0
  EditorFocus.Interval = 1
  EditorFocus.Enabled = 1
END SUB

SUB EditorFocusTick
  FocusTurns = FocusTurns - 1
  ' (the second time a few frames on: the tree's focus comes back in the
  ' host's next frame)
  IF FocusTurns <= 0 THEN EditorFocus.Enabled = 0 ELSE EditorFocus.Interval = 80
  IF FocusDoc >= 0 AND FocusDoc < DocCount THEN
    ' (a form's file opens on its designer, Delphi's way: the designer
    ' takes the keys then; the code's caret when the code is shown)
    IF DocOpen(FocusDoc) THEN
      IF DocViewNow(FocusDoc) = "Design" THEN
        Dock.FocusPane("designdoc(" + LTRIM$(STR$(FocusDoc)) + ")")
      ELSE
        Dock.FocusPane("codedoc(" + LTRIM$(STR$(FocusDoc)) + ")")
      END IF
    END IF
  END IF
END SUB

' ---- navigation across files: F12, Shift+F12, F2, Ctrl+S --------------------------------
' (the document showing Path's code, opened when it isn't; -1: no such file)
FUNCTION CodeDocOf(Path AS STRING) AS INTEGER
  DIM p AS STRING
  p = Path
  Result = DocOf(p)
  IF Result < 0 THEN
    p = ProjectPath(Path)
    Result = DocOf(p)
  END IF
  IF Result < 0 AND FILEEXISTS(p) THEN
    OpenDocument p
    Result = DocOf(p)
  END IF
END FUNCTION

' F12 found the definition in another file (an $INCLUDE's): there.
SUB CodeNavigate(File AS STRING, LineNo AS INTEGER, ColNo AS INTEGER, Sender AS RCODEEDITOR)
  DIM d AS INTEGER
  d = CodeDocOf(File)
  IF d < 0 THEN EXIT SUB
  Dock.FocusPane("codedoc(" + LTRIM$(STR$(d)) + ")")
  CodeDoc(d).GotoLineColumn LineNo, ColNo
END SUB

' (a JSON string's value after Key in Item: "" when it isn't there)
FUNCTION JsonField(Item AS STRING, Key AS STRING) AS STRING
  DIM k AS INTEGER, e AS INTEGER, c AS STRING
  k = INSTR(Item, CHR$(34) + Key + CHR$(34) + ":")
  IF k = 0 THEN EXIT FUNCTION
  k = k + LEN(Key) + 3
  IF MID$(Item, k, 1) = CHR$(34) THEN
    ' (a string: up to its closing quote, \" and \\ read back)
    e = k + 1
    DO WHILE e <= LEN(Item)
      c = MID$(Item, e, 1)
      IF c = "\" THEN
        Result = Result + MID$(Item, e + 1, 1)
        e = e + 2
      ELSEIF c = CHR$(34) THEN
        EXIT DO
      ELSE
        Result = Result + c
        e = e + 1
      END IF
    LOOP
  ELSE
    e = k
    DO WHILE e <= LEN(Item) AND INSTR(",}]", MID$(Item, e, 1)) = 0
      e = e + 1
    LOOP
    Result = MID$(Item, k, e - k)
  END IF
END FUNCTION

' Shift+F12: every use, listed in Output as places to click (this file's
' are also selected in the editor, as carets).
SUB CodeReferences(Json AS STRING, Sender AS RCODEEDITOR)
  DIM rest AS STRING, item AS STRING, k AS INTEGER, n AS INTEGER, f AS STRING, l AS INTEGER, d AS INTEGER, t AS STRING
  rest = Json
  OutputLine "References:"
  DO
    k = INSTR(rest, "{")
    IF k = 0 THEN EXIT DO
    rest = MID$(rest, k)
    k = INSTR(rest, "}")
    IF k = 0 THEN EXIT DO
    item = LEFT$(rest, k)
    rest = MID$(rest, k + 1)
    f = JsonField(item, "file")
    l = VAL(JsonField(item, "line"))
    t = ""
    d = DocOf(f)
    IF d >= 0 AND l > 0 THEN t = "  " + LTRIM$(CodeDoc(d).Line(l - 1))
    OutputLine "  " + f + ":" + LTRIM$(STR$(l)) + ":" + JsonField(item, "column") + t
    n = n + 1
  LOOP
  IF n = 1 THEN OutputLine "  1 reference" ELSE OutputLine "  " + LTRIM$(STR$(n)) + " references"
  Dock.ShowPane("outputbox")
END SUB

' F2 renamed names another file uses: its edits, applied there (opened).
SUB CodeFileEdits(File AS STRING, Json AS STRING, Sender AS RCODEEDITOR)
  DIM d AS INTEGER
  d = CodeDocOf(File)
  IF d < 0 THEN
    OutputLine "Rename: " + File + " couldn't be opened"
    EXIT SUB
  END IF
  CodeDoc(d).ApplyEdits Json
  IF DocDirty(d) = 0 THEN
    DocDirty(d) = 1
    DocTitles d
  END IF
END SUB

' ---- the scripted flows' steps (docs/studio-wow.md §0) --------------------------------
' `type:<text>`: typed one key at a time into the focused component (the
' kernel's keyboard path: completion, case and auto-close all happen);
' `key:<chord>`: one key, e.g. key:Tab, key:Shift+Tab, key:Ctrl+Space,
' key:F12, key:Enter, key:Comma (Ctrl is Ctrl as on Windows, Cmd's shortcuts on a Mac).
' True when Id was one (the macro waits while the keys are delivered).

' (text as SendKeys types it: its special characters in braces)
FUNCTION KeysOf(Text AS STRING) AS STRING
  DIM i AS INTEGER, c AS STRING
  FOR i = 1 TO LEN(Text)
    c = MID$(Text, i, 1)
    IF INSTR("+^%~(){}", c) > 0 THEN Result = Result + "{" + c + "}" ELSE Result = Result + c
  NEXT
END FUNCTION

' (a chord as SendKeys writes it: Ctrl+Shift+Space → ^+ )
FUNCTION ChordKeys(Chord AS STRING) AS STRING
  DIM rest AS STRING, part AS STRING, k AS INTEGER, mods AS STRING, u AS STRING
  rest = Chord
  DO
    k = INSTR(rest, "+")
    IF k <= 1 OR k = LEN(rest) THEN EXIT DO
    part = UCASE$(LEFT$(rest, k - 1))
    rest = MID$(rest, k + 1)
    SELECT CASE part
      CASE "CTRL", "CMD", "CONTROL": mods = mods + "^"
      CASE "SHIFT": mods = mods + "+"
      CASE "ALT", "OPTION": mods = mods + "%"
    END SELECT
  LOOP
  u = UCASE$(rest)
  IF LEN(rest) = 1 THEN
    Result = mods + KeysOf(LCASE$(rest))
  ELSEIF u = "SPACE" THEN
    Result = mods + " "
  ELSEIF u = "COMMA" THEN
    ' (a flow's steps are split at commas: key:Comma types one)
    Result = mods + ","
  ELSE
    Result = mods + "{" + u + "}"
  END IF
END FUNCTION

FUNCTION EditorStep(Id AS STRING) AS INTEGER
  IF LEFT$(Id, 5) = "type:" OR LEFT$(Id, 4) = "key:" OR LEFT$(Id, 6) = "focus:" THEN OutputLine "> " + Id
  IF LEFT$(Id, 6) = "focus:" THEN
    Dock.FocusPane(MID$(Id, 7))
    Result = 1
  ELSEIF LEFT$(Id, 5) = "type:" THEN
    Application.SendKeys KeysOf(MID$(Id, 6))
    Result = 1
  ELSEIF LEFT$(Id, 4) = "key:" THEN
    Application.SendKeys ChordKeys(MID$(Id, 5))
    Result = 1
  END IF
END FUNCTION
