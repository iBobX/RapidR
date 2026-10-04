' QFORM.WindowState (RapidQ: wsNormal 0, wsMinimized 1, wsMaximized 2,
' read / write): maximized, the form takes the work area (the page's
' viewport on the web) — Width / Height / ClientWidth read it, bigger
' than its own 300 x 200 — and OnResize fires as for a user's resize;
' restored, its own bounds come back; minimized, nothing is resized (no
' OnResize) and it stays Visible. The OnResize count is read by a later
' click (an interpreted program runs a handler's events after it).
DECLARE SUB MaxIt
DECLARE SUB RestoreIt
DECLARE SUB MinIt
DECLARE SUB Report
DECLARE SUB Resized
DIM Resizes AS INTEGER
DIM L AS INTEGER
DIM T AS INTEGER
CREATE Form AS QFORM
  Caption = "window state"
  Left = 100
  Top = 80
  Width = 300
  Height = 200
  OnResize = Resized
  CREATE B1 AS QBUTTON
    Left = 5 : Top = 5 : Width = 60 : Caption = "Max" : OnClick = MaxIt
  END CREATE
  CREATE B2 AS QBUTTON
    Left = 70 : Top = 5 : Width = 60 : Caption = "Restore" : OnClick = RestoreIt
  END CREATE
  CREATE B3 AS QBUTTON
    Left = 135 : Top = 5 : Width = 60 : Caption = "Min" : OnClick = MinIt
  END CREATE
  CREATE B4 AS QBUTTON
    Left = 200 : Top = 5 : Width = 60 : Caption = "Report" : OnClick = Report
  END CREATE
  CREATE Lbl AS QLABEL
    Left = 5 : Top = 40 : Width = 280 : Caption = "-"
  END CREATE
  CREATE Lbl2 AS QLABEL
    Left = 5 : Top = 65 : Width = 280 : Caption = "-"
  END CREATE
  CREATE Lbl3 AS QLABEL
    Left = 5 : Top = 90 : Width = 280 : Caption = ""
  END CREATE
END CREATE
SUB Resized
  Resizes = Resizes + 1
END SUB
SUB MaxIt
  L = Form.Left
  T = Form.Top
  Resizes = 0
  Form.WindowState = 2
  Lbl.Caption = STR$(Form.WindowState) + " " + STR$(Form.Width > 300) + STR$(Form.Height > 200) + STR$(Form.ClientWidth > 280)
END SUB
SUB RestoreIt
  Form.WindowState = 0
  Lbl.Caption = Lbl.Caption + "|" + STR$(Form.WindowState) + " " + STR$(Form.Width) + "x" + STR$(Form.Height) + " " + STR$(Form.Left = L) + STR$(Form.Top = T)
END SUB
SUB MinIt
  Form.WindowState = 1
  Lbl2.Caption = STR$(Form.WindowState) + " " + STR$(Form.Width) + " " + STR$(Form.Visible)
  Form.WindowState = 0
  Lbl2.Caption = Lbl2.Caption + "|" + STR$(Form.WindowState) + " " + STR$(Form.Width)
END SUB
SUB Report
  Lbl3.Caption = Lbl3.Caption + STR$(Resizes) + ";"
END SUB
Lbl.Caption = STR$(Form.WindowState)
Form.ShowModal
