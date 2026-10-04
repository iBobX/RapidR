' QCOLORDIALOG (RAPIDQ2.INC's QColorDialog): Color (0), Style
' (cdNoFullOpen = 2 by default), Colors(1 TO 16) (the constructor's, set
' with Colors(i) = c), Caption; Execute's answers — OK with a colour (1,
' Color set), Cancel (0, Color kept). The desktop's RAPIDR_TEST_COLOR_DIALOG
' answers "red, then Cancel"; the browser's page dialog is clicked so.
DECLARE SUB PickIt
DECLARE SUB CancelIt
DIM Dlg AS QCOLORDIALOG
CREATE Form AS QFORM
  Caption = "colour dialog"
  Width = 320
  Height = 160
  CREATE B1 AS QBUTTON
    Left = 5 : Top = 5 : Caption = "Pick" : OnClick = PickIt
  END CREATE
  CREATE B2 AS QBUTTON
    Left = 90 : Top = 5 : Caption = "Cancel" : OnClick = CancelIt
  END CREATE
  CREATE Lbl AS QLABEL
    Left = 5 : Top = 40 : Width = 300 : Caption = "-"
  END CREATE
  CREATE Lbl2 AS QLABEL
    Left = 5 : Top = 65 : Width = 300 : Caption = "-"
  END CREATE
END CREATE
Lbl.Caption = HEX$(Dlg.Color) + "|" + HEX$(Dlg.Style) + "|" + HEX$(Dlg.Colors(3)) + "|" + HEX$(Dlg.Colors(16))
SUB PickIt
  Dlg.Colors(2) = &H123456
  Dlg.Caption = "Pick a colour"
  IF Dlg.Execute THEN Lbl2.Caption = "ok " + HEX$(Dlg.Color) ELSE Lbl2.Caption = "cancel"
  Lbl2.Caption = Lbl2.Caption + " " + HEX$(Dlg.Colors(2))
END SUB
SUB CancelIt
  IF Dlg.Execute THEN Lbl2.Caption = Lbl2.Caption + "|ok" ELSE Lbl2.Caption = Lbl2.Caption + "|cancel " + HEX$(Dlg.Color)
END SUB
Form.ShowModal
