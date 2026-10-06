' A form's members RapidQ has for few programs: HideTitleBar / ShowTitleBar
' (the client area keeps its size, the form loses — then gets back — its
' title bar's height; RC.EXE's 300 x 200 form went to 300 x 177), QFORM's
' MDI members (a QFORM has no MDI children: MDIChildCount 0, Cascade, Tile,
' Next, Previous and ArrangeIcons do nothing), a list's OnEnter (the
' keyboard focus comes to it: SetFocus, a click), and OnHint (the long
' hint of what the mouse is over, "" over nothing with a hint) — and the
' file list's tooltip (ShowHint), shown sooner than the VCL's 500 ms here so
' the capture has it.
DECLARE SUB HideIt
DECLARE SUB ShowIt
DECLARE SUB Arrange
DECLARE SUB Entered1
DECLARE SUB Entered2
DECLARE SUB Focus1
DECLARE SUB Hinted (Hint AS STRING)
DIM Log AS STRING, Hints AS STRING
CREATE Form AS QFORM
  Caption = "form members"
  Width = 360
  Height = 300
  Hint = "the form"
  OnHint = Hinted
  CREATE L1 AS QLISTBOX
    Left = 5
    Top = 5
    Width = 120
    Height = 60
    OnEnter = Entered1
  END CREATE
  CREATE L2 AS QFILELISTBOX
    Left = 5
    Top = 75
    Width = 120
    Height = 60
    Mask = "*.nothing-here"
    Hint = "files|the folder's files"
    ShowHint = 1
    OnEnter = Entered2
  END CREATE
  CREATE Ed AS QEDIT
    Left = 140
    Top = 5
    Hint = "Name|Your full name"
  END CREATE
  CREATE Hide AS QBUTTON
    Caption = "hide"
    Left = 140
    Top = 40
    OnClick = HideIt
  END CREATE
  CREATE Show AS QBUTTON
    Caption = "show"
    Left = 220
    Top = 40
    OnClick = ShowIt
  END CREATE
  CREATE Arr AS QBUTTON
    Caption = "arrange"
    Left = 140
    Top = 75
    OnClick = Arrange
  END CREATE
  CREATE Foc AS QBUTTON
    Caption = "focus"
    Left = 220
    Top = 75
    OnClick = Focus1
  END CREATE
  CREATE Lbl AS QLABEL
    Left = 5
    Top = 145
    Width = 340
    Caption = "-"
  END CREATE
  CREATE Lh AS QLABEL
    Left = 5
    Top = 170
    Width = 340
    Caption = "-"
  END CREATE
END CREATE

SUB Note(S AS STRING)
  Log = Log + " " + S
  Lbl.Caption = Log
END SUB

SUB HideIt
  Form.HideTitleBar
  Note "h" + STR$(Form.Height) + "," + STR$(Form.ClientHeight)
  Form.HideTitleBar
  Note "h" + STR$(Form.Height)
END SUB

SUB ShowIt
  Form.ShowTitleBar
  Note "s" + STR$(Form.Height) + "," + STR$(Form.ClientHeight)
END SUB

SUB Arrange
  Form.Cascade
  Form.Tile
  Form.Next
  Form.Previous
  Form.ArrangeIcons
  Note "m" + STR$(Form.MdiChildCount) + STR$(Form.TileMode)
END SUB

SUB Entered1
  Note "e1"
END SUB

SUB Entered2
  Note "e2"
END SUB

SUB Focus1
  L1.SetFocus
END SUB

SUB Hinted (Hint AS STRING)
  Hints = Hints + "[" + Hint + "]"
  Lh.Caption = Hints
END SUB

Application.HintPause = 100
Note "start" + STR$(Form.Height) + "," + STR$(Form.ClientHeight)
Form.ShowModal
