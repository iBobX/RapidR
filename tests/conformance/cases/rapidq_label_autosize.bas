' A QLABEL's AutoSize, as RapidQ has it — the .expected is RC.EXE's output
' with the text measured in RapidR Sans, a pixel wider than RapidQ's bitmap
' font where readability needs it (r, &, ...: "Password:" 50 x 13, RC.EXE 49;
' the CHANGELOG's "Text in RapidQ's default font was cramped")
' (docs/rapidq-ground-truth.md; rapidr_value::autosize): True by default; a
' new label is 65 x 17 until its Caption changes, then its text's size
' (MS Sans Serif 8, & not counted); a Width set
' after the Caption sticks until the next Caption / font / WordWrap change
' or AutoSize turned on; right alignment keeps the right edge; WordWrap
' wraps at the current Width and takes the widest line; a font change of
' the label or of its parent (any of its properties, its colour too)
' resizes it, setting what it already is doesn't.
CREATE F AS QFORM
  Width = 400: Height = 300
  CREATE L0 AS QLABEL
  END CREATE
  CREATE L1 AS QLABEL
    Caption = "Password:": Left = 10: Top = 40
  END CREATE
  CREATE L2 AS QLABEL
    Caption = "Pass&word:": Left = 8: Top = 40: Width = 64
  END CREATE
  CREATE L3 AS QLABEL
    Width = 64: Caption = "Pass&word:": Left = 8
  END CREATE
  CREATE L4 AS QLABEL
    Caption = "Right": Left = 240: Top = 12: Width = 120: Alignment = 1: AutoSize = 0
  END CREATE
  CREATE L5 AS QLABEL
    Caption = "Right": Left = 240: Top = 12: Alignment = 1
  END CREATE
  CREATE L6 AS QLABEL
    Caption = "The quick brown fox jumps over the lazy dog. 0123456789": Left = 10: Top = 214
  END CREATE
  CREATE L7 AS QLABEL
    Caption = "Word wrap: the quick brown fox jumps over the lazy dog.": Left = 10: Top = 232: Width = 200: WordWrap = 1
  END CREATE
  CREATE L8 AS QLABEL
    Caption = "": Left = 10
  END CREATE
  CREATE L9 AS QLABEL
    Caption = "Two" + CHR$(13) + CHR$(10) + "lines here": Left = 10
  END CREATE
  CREATE P AS QPANEL
    Left = 150: Top = 10: Width = 200: Height = 100
    CREATE C AS QLABEL
      Caption = "Gamma": Left = 5: Top = 5: Width = 90
    END CREATE
  END CREATE
END CREATE
PRINT "L0 "; L0.Width; " "; L0.Height; " auto "; L0.AutoSize; " cap ["; L0.Caption; "]"
PRINT "L1 "; L1.Width; " "; L1.Height; " L "; L1.Left
PRINT "L2 "; L2.Width; " "; L2.Height
PRINT "L3 "; L3.Width; " "; L3.Height
PRINT "L4 "; L4.Width; " "; L4.Height; " L "; L4.Left; " auto "; L4.AutoSize
PRINT "L5 "; L5.Width; " "; L5.Height; " L "; L5.Left
PRINT "L6 "; L6.Width; " "; L6.Height
PRINT "L7 "; L7.Width; " "; L7.Height
PRINT "L8 "; L8.Width; " "; L8.Height
PRINT "L9 "; L9.Width; " "; L9.Height
L1.Width = 20
PRINT "L1w "; L1.Width; " "; L1.Height
L1.Caption = "Password:"
PRINT "L1same "; L1.Width; " "; L1.Height
L1.Caption = "Pass"
PRINT "L1cap "; L1.Width; " "; L1.Height
L1.Width = 100: L1.Height = 50
L1.AutoSize = 1
PRINT "L1auto1 "; L1.Width; " "; L1.Height
L1.AutoSize = 0
L1.Width = 100: L1.Height = 50
L1.AutoSize = 1
PRINT "L1auto01 "; L1.Width; " "; L1.Height
L5.Caption = "Right aligned longer"
PRINT "L5 "; L5.Width; " "; L5.Height; " L "; L5.Left
L4.Caption = "Right aligned longer text"
PRINT "L4 "; L4.Width; " "; L4.Height; " L "; L4.Left
L4.AutoSize = 1
PRINT "L4a "; L4.Width; " "; L4.Height; " L "; L4.Left
L7.Caption = "Word wrap: the quick brown fox jumps over the lazy dog. More words."
PRINT "L7 "; L7.Width; " "; L7.Height
L7.Width = 80
PRINT "L7w "; L7.Width; " "; L7.Height
L8.Caption = "&"
PRINT "L8& "; L8.Width; " "; L8.Height
L8.Caption = "&&"
PRINT "L8&& "; L8.Width; " "; L8.Height
L8.Caption = "A" + CHR$(9) + "B"
PRINT "L8tab "; L8.Width; " "; L8.Height
L8.Left = 300: L8.Alignment = 2
L8.Caption = "Centred text"
PRINT "L8c "; L8.Left; " "; L8.Width; " "; L8.Height
L8.WordWrap = 1
PRINT "L8ww "; L8.Left; " "; L8.Width; " "; L8.Height
L8.Width = 30
L8.Caption = "Centred text again"
PRINT "L8ww2 "; L8.Left; " "; L8.Width; " "; L8.Height
L8.WordWrap = 0
PRINT "L8ww3 "; L8.Left; " "; L8.Width; " "; L8.Height
L2.Font.Size = 8
L2.Font.Name = "MS Sans Serif"
PRINT "L2same "; L2.Width; " "; L2.Height
L2.Font.Color = &HFF
PRINT "L2color "; L2.Width; " "; L2.Height
L3.Width = 70
F.Font.Color = &HFF0000
PRINT "F "; L3.Width; " "; L3.Height; " C "; C.Width; " "; C.Height
L3.Width = 70: C.Width = 90
P.Font.Name = "MS Sans Serif"
PRINT "P "; L3.Width; " C "; C.Width
L3.Visible = 0
L3.Caption = "Hidden one"
PRINT "L3h "; L3.Width; " "; L3.Height
DIM N AS QLABEL
N.Caption = "No parent"
PRINT "N "; N.Width; " "; N.Height
N.Parent = F
PRINT "N "; N.Width; " "; N.Height
