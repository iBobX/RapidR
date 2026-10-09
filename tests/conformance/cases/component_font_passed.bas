' (RapidR's) A component's own Font passed where a QFONT goes: QFONTDIALOG's
' SetFont / GetFont and `X.Font = Y.Font`. RC.EXE refuses these at compile
' time ("Wrong type L.FONT"), so RapidQ programs pass a QFONT
' (rapidq_font_dialog_fonts); RapidR takes the component's font too.
DIM D AS QFONTDIALOG
CREATE Form AS QFORM
  CREATE L AS QLABEL
    Caption = "x"
  END CREATE
  CREATE P AS QPANEL
    Font.Name = "Arial"
    Font.Size = 20
    Font.Color = &H00FF00
    CREATE Inner AS QLABEL
    END CREATE
  END CREATE
  CREATE L2 AS QLABEL
  END CREATE
END CREATE
D.Name = "Courier New": D.Size = 14: D.Color = &HFF0000
D.AddStyles(1, 3)
D.SetFont(L.Font)
PRINT L.Font.Name; "|"; L.Font.Size; "|"; L.Font.Color; "|"; L.Font.Italic; "|"; L.Font.Strikeout; "|"; L.Font.Underline
' (the panel's font, Inner's through ParentFont)
D.GetFont(Inner.Font)
PRINT D.Name; "|"; D.Size; "|"; D.Color; "|"; D.FontItalic
L2.Font = L.Font
PRINT L2.Font.Name; "|"; L2.Font.Size; "|"; L2.Font.Italic
' (a QFONT as before)
DIM F AS QFONT
D.SetFont(F)
PRINT F.Name; "|"; F.Italic
