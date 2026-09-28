' QSTRINGGRID OnDrawCell(Col, Row, State, Rect, Sender): fired for every
' cell, drawing kept on the cell; reading the grid in the handler doesn't
' fire it again, Repaint does. Checked natively and interpreted by
' tests/native_gui_events.mjs.
DECLARE SUB DrawCell (Col%, Row%, State%, Rect AS QRECT, Sender AS QSTRINGGRID)
DECLARE SUB Again
DIM Calls AS INTEGER
DIM Seen AS STRING
DIM Fixed AS STRING
DIM Round AS INTEGER
Round = 1
CREATE Form AS QFORM
  Caption = "Draw": Width = 360: Height = 220
  CREATE Grid AS QSTRINGGRID
    Width = 340: Height = 140
    OnDrawCell = DrawCell
  END CREATE
  CREATE Btn AS QBUTTON
    Top = 150: Caption = "Again"
    OnClick = Again
  END CREATE
  CREATE Lbl AS QLABEL
    Top = 180: Width = 340
  END CREATE
END CREATE
Grid.Cell(2, 1) = "two"
Form.ShowModal

SUB DrawCell (Col%, Row%, State%, Rect AS QRECT, Sender AS QSTRINGGRID)
  Calls = Calls + 1
  IF Col% = 2 AND Row% = 1 THEN
    Seen = STR$(State%) + "," + STR$(Rect.Left) + "," + STR$(Rect.Top) + "," + STR$(Rect.Right) + "," + STR$(Rect.Bottom) + "," + Sender.Cell(Col%, Row%)
    Sender.Rectangle(Rect.Left + 2, Rect.Top + 2, Rect.Right - 2, Rect.Bottom - 2, &HFF)
  END IF
  IF Col% = 0 AND Row% = 0 THEN Fixed = "fixed" + STR$(State%)
  IF Col% = 1 AND Row% = 1 THEN Fixed = Fixed + " selected" + STR$(State%)
  Lbl.Caption = "round" + STR$(Round) + ":" + STR$(Calls) + "|" + Seen + "|" + Fixed
END SUB

SUB Again
  Calls = 0
  Seen = ""
  Fixed = ""
  Lbl.Caption = "again"
  Round = 2
  Grid.Repaint
END SUB
