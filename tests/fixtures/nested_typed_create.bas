' A TYPE's instance CREATEd inside a form's CREATE is made where it stands,
' so the rest of the block — a ShowModal at its end (RapidQ's
' richedit/syntax.bas) — finds it. RC.EXE: the form's OnShow sees
' "shown: form [after] note [note] 7 10" (tests/results probe w_nested,
' 2026-10-15). RapidR made such instances after the whole block: after
' ShowModal had returned.
$INCLUDE "RAPIDQ.INC"
DECLARE SUB Shown
TYPE TNote EXTENDS QLABEL
  Tag2 AS INTEGER
  CONSTRUCTOR
    Caption = "note"
    Tag2 = 7
  END CONSTRUCTOR
END TYPE
CREATE Form AS QFORM
  Caption = "before"
  OnShow = Shown
  CREATE N AS TNote
    Left = 10
  END CREATE
  CREATE Lbl AS QLABEL
    Top = 40
    Width = 300
    Caption = "-"
  END CREATE
  Caption = "after"
  ShowModal
END CREATE
SUB Shown
  Lbl.Caption = "shown: form [" + Form.Caption + "] note [" + N.Caption + "] " + STR$(N.Tag2) + " " + STR$(N.Left)
END SUB
