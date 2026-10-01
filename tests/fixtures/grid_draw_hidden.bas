' OnDrawCell fires when the grid is painted on screen, as in RapidQ — not
' while its form is still hidden: a program creates what its handler reads
' (here Marks, after the grid) before showing the form.
DECLARE SUB DrawCell (Col%, Row%, State%, R AS QRECT)
DIM Drawn AS INTEGER
CREATE Form AS QFORM
  Caption = "grid draw"
  CREATE G AS QSTRINGGRID
    ColCount = 2
    RowCount = 2
    OnDrawCell = DrawCell
    Cell(0, 0) = "x"
  END CREATE
  CREATE Lbl AS QLABEL
    Top = 150 : Width = 300
  END CREATE
END CREATE
DIM Marks(1, 1) AS STRING
Marks(1, 1) = "m"
Before = Drawn
Form.ShowModal
SUB DrawCell (Col%, Row%, State_, R AS QRECT)
  Drawn = Drawn + 1
  Lbl.Caption = "before=" + STR$(Before) + " mark=" + Marks(1, 1)
END SUB
