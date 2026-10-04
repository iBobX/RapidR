' A QFORM's Visible on both runtimes: False until the form shows (RapidQ's
' default); `Visible = 1` is its Show — inside its own CREATE too, where it
' shows once the program waits — with OnShow; reading it tells whether it shows.
DECLARE SUB Shown2
DECLARE SUB Shown3
DECLARE SUB Clicked
DIM n2 AS INTEGER, n3 AS INTEGER
CREATE f2 AS QFORM
  Visible = 1
  OnShow = Shown2
  Left = 600 : Width = 150 : Height = 100
END CREATE
CREATE f3 AS QFORM
  OnShow = Shown3
  Left = 650 : Width = 150 : Height = 100
END CREATE
CREATE f AS QFORM
  CREATE btn AS QBUTTON
    OnClick = Clicked
  END CREATE
  CREATE lbl AS QLABEL
    Top = 40 : Width = 300
  END CREATE
END CREATE
lbl.Caption = "start " + STR$(f.Visible) + STR$(f2.Visible) + STR$(f3.Visible) + ";"
f.ShowModal

SUB Shown2: n2 = n2 + 1: END SUB
SUB Shown3: n3 = n3 + 1: END SUB

SUB Clicked
  f3.Visible = 1
  lbl.Caption = lbl.Caption + STR$(f.Visible) + STR$(f2.Visible) + STR$(f3.Visible) + STR$(n2) + STR$(n3) + ";"
END SUB
