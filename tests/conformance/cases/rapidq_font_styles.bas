' A component's font styles as RapidQ keeps them — the .expected is RC.EXE's
' output (docs/rapidq-ground-truth.md): Font.AddStyles / DelStyles inside
' and outside a CREATE, styles read back 1 or 0 whatever was set, numbers
' past fsStrikeOut ignored; a QFONT's the same; the first font change of a
' component ends Delphi's ParentFont (the form's later style isn't its).
CREATE Form AS QFORM
  CREATE L AS QLABEL
    Caption = "x"
    Font.AddStyles(0)
  END CREATE
  CREATE L2 AS QLABEL
  END CREATE
END CREATE
PRINT L.Font.Bold; " "; L.Font.Underline
L.Font.AddStyles(2, 1)
PRINT L.Font.Bold; " "; L.Font.Italic; " "; L.Font.Underline
WITH L
  .Font.DelStyles(0)
END WITH
PRINT L.Font.Bold; " "; L.Font.Underline
PRINT L2.Font.Bold; " "; L2.Font.Italic
L2.Font.Bold = 1
PRINT L2.Font.Bold
L2.Font.Bold = -1
PRINT L2.Font.Bold
L2.Font.Bold = 0
PRINT L2.Font.Bold
L2.Font.Underline = 5
PRINT L2.Font.Underline
L2.Font.AddStyles(0, 3)
PRINT L2.Font.Bold; " "; L2.Font.StrikeOut
L2.Font.DelStyles(3)
PRINT L2.Font.Bold; " "; L2.Font.StrikeOut
L2.Font.AddStyles(7)
PRINT L2.Font.Bold; L2.Font.Italic; L2.Font.Underline; L2.Font.StrikeOut
DIM F AS QFONT
F.AddStyles(0, 3)
PRINT F.Bold; F.StrikeOut
F.Italic = -1
PRINT F.Italic
F.DelStyles(0, 1)
PRINT F.Bold; F.Italic
Form.Font.AddStyles(1)
PRINT Form.Font.Italic; L2.Font.Italic
