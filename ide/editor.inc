' editor.inc: the code documents' editor (RCODEEDITOR on rapidr-editor and
' RapidR's language service, docs/ide-plan.md I2 / I3, lane S-EDITOR) —
' its Edit commands, the designer's source patches, and the scripted
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
DIM FocusDoc AS INTEGER

SUB FocusCodeSoon(d AS INTEGER)
  FocusDoc = d
  EditorFocus.Enabled = 0
  EditorFocus.Enabled = 1
END SUB

SUB EditorFocusTick
  EditorFocus.Enabled = 0
  IF FocusDoc >= 0 AND FocusDoc < DocCount THEN
    IF DocOpen(FocusDoc) THEN Dock.FocusPane("codedoc(" + LTRIM$(STR$(FocusDoc)) + ")")
  END IF
END SUB

' ---- the designer's edits to the source -----------------------------------------------
' RDESIGNSURFACE's OnSourceEdit(StartLine, StartCol, EndLine, EndCol, Text)
' — one per patch, in order; lines from 0, columns in characters — applied
' to document d's code with ApplyPatch. The patches of one designer action
' arrive together, before the next turn of the event loop: the first starts
' an undo step, the rest join it (PatchJoin, cleared by a timer that fires
' once they've all been handled), so Edit > Undo takes the whole action back
' in one step — the code's history is the file's only one (the source is
' the form), and the designer follows the text it's given back.
DIM PatchJoin AS INTEGER

SUB DesignerSourceEdit(d AS INTEGER, StartLine AS INTEGER, StartCol AS INTEGER, EndLine AS INTEGER, EndCol AS INTEGER, Text AS STRING)
  IF d < 0 THEN EXIT SUB
  CodeDoc(d).ApplyPatch StartLine, StartCol, EndLine, EndCol, Text, PatchJoin
  PatchJoin = 1
  PatchBurst.Enabled = 0
  PatchBurst.Enabled = 1
END SUB

SUB PatchBurstTick
  PatchBurst.Enabled = 0
  PatchJoin = 0
END SUB

' ---- the scripted flows' steps (docs/studio-wow.md §0) --------------------------------
' `type:<text>`: typed one key at a time into the focused component (the
' kernel's keyboard path: completion, case and auto-close all happen);
' `key:<chord>`: one key, e.g. key:Tab, key:Shift+Tab, key:Ctrl+Space,
' key:F12, key:Enter (Ctrl is Ctrl as on Windows, Cmd's shortcuts on a Mac).
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
