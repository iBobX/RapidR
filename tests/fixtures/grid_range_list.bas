' QSTRINGGRID range selection (goRangeSelect, on by default as in Delphi)
' seen through OnDrawCell's State, and a gcsList column's drop-down
' (ColumnList, which OnListDropDown may change: its S comes back) whose
' pick goes through OnSetEditText, and column sizing
' (goColSizing). Driven with the mouse
' by tests/web_ide_grid_draw.mjs.
DECLARE SUB DrawCell (Col%, Row%, State%, Rect AS QRECT)
DECLARE SUB SetText (Col%, Row%, Value$)
DECLARE SUB ListDrop (Col%, Row%, BYREF S AS STRING)
DIM Selected AS INTEGER
CREATE Form AS QFORM
  Caption = "Range": Width = 360: Height = 220
  CREATE Grid AS QSTRINGGRID
    Width = 340: Height = 140
    AddOptions 7   ' goColSizing
    ColumnStyle(1) = 0
    ColumnList(1) = "red" + CHR$(10) + "blue" + CHR$(10) + "green"
    OnDrawCell = DrawCell
    OnSetEditText = SetText
    OnListDropDown = ListDrop
  END CREATE
  CREATE Lbl AS QLABEL
    Top = 150: Width = 340
  END CREATE
  CREATE Info AS QLABEL
    Top = 175: Width = 340
  END CREATE
END CREATE
Form.ShowModal

SUB DrawCell (Col%, Row%, State%, Rect AS QRECT)
  IF Col% = 0 AND Row% = 0 THEN Selected = 0
  IF State% AND 1 THEN Selected = Selected + 1
  Lbl.Caption = "selected" + STR$(Selected)
END SUB

SUB SetText (Col%, Row%, Value$)
  Info.Caption = "set" + STR$(Col%) + "," + STR$(Row%) + " " + Value$ + " " + Grid.Cell(Col%, Row%)
END SUB

SUB ListDrop (Col%, Row%, BYREF S AS STRING)
  IF Col% = 1 THEN S = S + CHR$(10) + "pink"
END SUB
