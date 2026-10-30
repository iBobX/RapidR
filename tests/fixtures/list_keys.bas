' Keys typed in a QLISTBOX reach its form's OnKeyDown / OnKeyPress / OnKeyUp
' only with the form's KeyPreview on (the list has no key events of its own,
' nor a TabStop). Without it the form hears nothing; with it, a letter gives
' OnKeyDown (65), OnKeyPress (97), OnKeyUp (65) and an arrow OnKeyDown (40),
' OnKeyUp (40). RC.EXE sends the same to a QLISTBOX's window (Windows' own
' WM_KEYDOWN, WM_CHAR, WM_KEYUP): w_list_keys in docs/rapidq-ground-truth.md.
$INCLUDE "RAPIDQ.INC"
DECLARE SUB FormDown (Key AS WORD, Shift AS INTEGER)
DECLARE SUB FormPress (Key AS BYTE)
DECLARE SUB FormUp (Key AS WORD, Shift AS INTEGER)
DECLARE SUB PreviewOn
DIM Keys AS STRING

CREATE Form AS QFORM
  Caption = "list keys"
  Width = 320
  Height = 240
  KeyPreview = 0
  OnKeyDown = FormDown
  OnKeyPress = FormPress
  OnKeyUp = FormUp
  CREATE Lst AS QLISTBOX
    Left = 10
    Top = 10
    Width = 150
    Height = 120
    AddItems "apple", "banana", "cherry"
    ItemIndex = 0
  END CREATE
  CREATE Pv AS QBUTTON
    Left = 170
    Top = 10
    Caption = "Preview"
    OnClick = PreviewOn
  END CREATE
  CREATE Lk AS QLABEL
    Left = 10
    Top = 150
    Width = 300
    Caption = "-"
  END CREATE
END CREATE

SUB FormDown (Key AS WORD, Shift AS INTEGER)
  Keys = Keys + "d" + STR$(Key) + " "
  Lk.Caption = Keys
END SUB

SUB FormPress (Key AS BYTE)
  Keys = Keys + "p" + STR$(Key) + " "
  Lk.Caption = Keys
END SUB

SUB FormUp (Key AS WORD, Shift AS INTEGER)
  Keys = Keys + "u" + STR$(Key) + " "
  Lk.Caption = Keys
END SUB

SUB PreviewOn
  Form.KeyPreview = 1
  Keys = Keys + "| "
  Lk.Caption = Keys
END SUB

Form.ShowModal
