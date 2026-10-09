' QFONTDIALOG's GetFont / SetFont with a QFONT, and what a QFONT's and a
' component's fonts read — the .expected is RC.EXE's output
' (docs/rapidq-ground-truth.md): GetFont(F) gives the dialog F's font,
' SetFont(F) gives F the dialog's; colours pass as they are (clWindowText
' stays -2147483640, the dialog's own Color at first); styles read 1 / 0.
DIM D AS QFONTDIALOG
CREATE Form AS QFORM
  CREATE L AS QLABEL
    Caption = "x"
  END CREATE
END CREATE
DIM F AS QFONT
PRINT "F:"; F.Name; "|"; F.Size; "|"; F.Color
PRINT "D:"; D.Name; "|"; D.Size; "|"; D.Color
PRINT "L:"; L.Font.Name; "|"; L.Font.Size; "|"; L.Font.Color; "|"; L.Color
PRINT "Form:"; Form.Font.Color
F.Name = "Times New Roman": F.Size = 9
D.GetFont(F)
PRINT D.Name; "|"; D.Size; "|"; D.Color
D.Name = "Verdana"
D.SetFont(F)
PRINT F.Name; "|"; F.Size; "|"; F.Color
L.Font = F
PRINT L.Font.Name; "|"; L.Font.Size; "|"; L.Font.Italic
F.Color = &HFF
L.Font = F
PRINT "L1:"; L.Font.Color
F.Color = -2147483640
L.Font = F
PRINT "L2:"; L.Font.Color
F.Color = &HFF
D.Color = &HFF0000
D.SetFont(F)
PRINT "F1:"; F.Color
D.Color = &H123456
D.GetFont(F)
PRINT "D1:"; D.Color
D.AddStyles(1)
D.SetFont(F)
PRINT "F2:"; F.Italic; F.Bold
F.Bold = 1
D.GetFont(F)
D.SetFont(F)
PRINT "F3:"; F.Italic; F.Bold
DIM G AS QFONT
G.Italic = 1: G.StrikeOut = 1: G.Bold = 1: G.Underline = 1
PRINT G.Italic; "|"; G.Strikeout; "|"; G.Bold; "|"; G.Underline
G.Italic = 0
PRINT G.Italic
G.Italic = -1
PRINT G.Italic
G.Italic = 5
PRINT G.Italic
L.Font.Italic = 1
PRINT L.Font.Italic
L.Font.Italic = 0
PRINT L.Font.Italic
L.Font.Bold = -1
PRINT L.Font.Bold
