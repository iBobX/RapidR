' QFONTDIALOG (RapidQ manual): Name, Size (TFontDialog's font at first:
' the default QFONT, Arial 10), FontCount and FontName(i), GetFont(F)
' before Execute and SetFont(F) after it, AddOptions (fdApplyButton = 15),
' MinFontSize; Execute's answers — OK with a font (1), Cancel (0). The
' desktop's RAPIDR_TEST_FONT_DIALOG answers "Courier New, 14, bold and
' underlined, red, then Cancel"; the browser's page dialog is set so.
DECLARE SUB PickIt
DECLARE SUB CancelIt
DIM Dlg AS QFONTDIALOG
DIM F AS QFONT
CREATE Form AS QFORM
  Caption = "font dialog"
  Width = 380
  Height = 160
  CREATE B1 AS QBUTTON
    Left = 5 : Top = 5 : Caption = "Pick" : OnClick = PickIt
  END CREATE
  CREATE B2 AS QBUTTON
    Left = 90 : Top = 5 : Caption = "Cancel" : OnClick = CancelIt
  END CREATE
  CREATE Lbl AS QLABEL
    Left = 5 : Top = 40 : Width = 370 : Caption = "-"
  END CREATE
  CREATE Lbl2 AS QLABEL
    Left = 5 : Top = 65 : Width = 370 : Caption = "-"
  END CREATE
END CREATE
Lbl.Caption = Dlg.Name + "|" + STR$(Dlg.Size) + "|" + STR$(Dlg.FontCount) + "|" + Dlg.FontName(1)
SUB PickIt
  F.Name = "Times New Roman"
  F.Size = 12
  F.AddStyles(1)
  Dlg.GetFont(F)
  Lbl.Caption = Lbl.Caption + "|" + Dlg.Name + STR$(Dlg.Size)
  Dlg.AddOptions(15)
  Dlg.MinFontSize = 8
  IF Dlg.Execute THEN
    Dlg.SetFont(F)
    Lbl2.Caption = "ok " + F.Name + STR$(F.Size) + " " + STR$(F.Bold) + STR$(F.Italic) + STR$(F.Underline) + " " + HEX$(F.Color)
  ELSE
    Lbl2.Caption = "cancel"
  END IF
END SUB
SUB CancelIt
  IF Dlg.Execute THEN Lbl2.Caption = Lbl2.Caption + "|ok" ELSE Lbl2.Caption = Lbl2.Caption + "|cancel " + Dlg.Name
END SUB
Form.ShowModal
