' File dialogs: QOPENDIALOG / QSAVEDIALOG (Filter, FilterIndex, FileName)
' and RAPIDQ2.INC's QFILEDIALOG (Mode, MultiSelect, Files(0) the folder
' then the names, SelCount, FileTitle, DefaultExt on a save).
DECLARE SUB OpenIt
DECLARE SUB SaveIt
DECLARE SUB MultiIt
DIM Od AS QOPENDIALOG
DIM Fd AS QFILEDIALOG
CREATE Form AS QFORM
  Caption = "file dialogs"
  Width = 360
  Height = 200
  CREATE B1 AS QBUTTON
    Left = 5 : Top = 5 : Caption = "Open" : OnClick = OpenIt
  END CREATE
  CREATE B2 AS QBUTTON
    Left = 90 : Top = 5 : Caption = "Save" : OnClick = SaveIt
  END CREATE
  CREATE B3 AS QBUTTON
    Left = 175 : Top = 5 : Caption = "Multi" : OnClick = MultiIt
  END CREATE
  CREATE Lbl AS QLABEL
    Left = 5 : Top = 40 : Width = 350 : Caption = "-"
  END CREATE
  CREATE Lbl2 AS QLABEL
    Left = 5 : Top = 65 : Width = 350 : Caption = "-"
  END CREATE
  CREATE Lbl3 AS QLABEL
    Left = 5 : Top = 90 : Width = 350 : Caption = "-"
  END CREATE
END CREATE
SUB OpenIt
  Od.Filter = "Text files|*.txt|All Files|*.*"
  Od.FilterIndex = 2
  IF Od.Execute THEN Lbl.Caption = "open " + Od.FileName ELSE Lbl.Caption = "cancel"
END SUB
SUB SaveIt
  Fd.Mode = 1
  Fd.DefaultExt = "txt"
  IF Fd.Execute THEN Lbl2.Caption = "save " + Fd.FileTitle ELSE Lbl2.Caption = "cancel"
END SUB
SUB MultiIt
  DIM I AS INTEGER, S AS STRING
  Fd.Mode = 0
  Fd.MultiSelect = 1
  IF Fd.Execute THEN
    FOR I = 1 TO Fd.SelCount
      S = S + Fd.Files(I) + " "
    NEXT
    Lbl3.Caption = STR$(Fd.SelCount) + " " + S
  ELSE
    Lbl3.Caption = "cancel"
  END IF
END SUB
Form.ShowModal
