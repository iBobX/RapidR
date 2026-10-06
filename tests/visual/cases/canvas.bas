' A canvas drawn in its OnPaint (lines, rectangles, circles, text in the
' canvas's default font) and a splitter between two panels.
DECLARE SUB Paint
CREATE Form AS QFORM
  Caption = "Canvas": Left = 40: Top = 40: Width = 360: Height = 240
  CREATE Cv AS QCANVAS
    Left = 10: Top = 10: Width = 200: Height = 120: OnPaint = Paint
  END CREATE
  CREATE PLeft AS QPANEL
    Caption = "Left": Left = 10: Top = 140: Width = 120: Height = 50
  END CREATE
  CREATE Spl AS QSPLITTER
    Left = 130: Top = 140: Width = 5: Height = 50
  END CREATE
  CREATE PRight AS QPANEL
    Caption = "Right": Left = 135: Top = 140: Width = 120: Height = 50
  END CREATE
END CREATE
SUB Paint
  Cv.FillRect 0, 0, 200, 120, &HFFFFFF
  Cv.Rectangle 4, 4, 196, 116, &H0
  Cv.Line 4, 4, 196, 116, &HFF
  Cv.Circle 20, 20, 80, 80, &HFF0000, &HFFFF00
  Cv.RoundRect 100, 20, 180, 60, 10, 10, &H8000
  Cv.TextOut 100, 70, "TextOut", &H0, &HFFFFFF
  Cv.TextOut 100, 90, "Pset:", &H800000, &HFFFFFF
  Cv.Pset 140, 96, &HFF
END SUB
Form.ShowModal
