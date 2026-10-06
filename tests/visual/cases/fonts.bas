' Labels in the default font and in others: sizes, styles, faces, so the
' text's size and width can be compared.
CREATE Form AS QFORM
  Caption = "Fonts": Left = 40: Top = 40: Width = 420: Height = 300
  CREATE L1 AS QLABEL
    Caption = "Default: The quick brown fox jumps over the lazy dog": Left = 8: Top = 8
  END CREATE
  CREATE L2 AS QLABEL
    Caption = "MS Sans Serif 10: The quick brown fox": Left = 8: Top = 28
  END CREATE
  CREATE L3 AS QLABEL
    Caption = "MS Sans Serif 12: The quick brown": Left = 8: Top = 50
  END CREATE
  CREATE L4 AS QLABEL
    Caption = "Bold: The quick brown fox jumps": Left = 8: Top = 76
  END CREATE
  CREATE L5 AS QLABEL
    Caption = "Italic: The quick brown fox jumps": Left = 8: Top = 96
  END CREATE
  CREATE L6 AS QLABEL
    Caption = "Arial 10: The quick brown fox jumps": Left = 8: Top = 116
  END CREATE
  CREATE L7 AS QLABEL
    Caption = "Times New Roman 12: The quick brown": Left = 8: Top = 138
  END CREATE
  CREATE L8 AS QLABEL
    Caption = "Courier New 10: The quick brown fox": Left = 8: Top = 164
  END CREATE
  CREATE L9 AS QLABEL
    Caption = "Underline 8": Left = 8: Top = 186
  END CREATE
  CREATE L10 AS QLABEL
    Caption = "0123456789 ABCDEFGHIJKLMNOPQRSTUVWXYZ abcdefghijklmnopqrstuvwxyz": Left = 8: Top = 206
  END CREATE
  CREATE L11 AS QLABEL
    Left = 8: Top = 226
  END CREATE
END CREATE
DIM f AS QFONT
L11.Caption = "!" + CHR$(34) + "#$%&()*+,-./:;<=>?@[\]^_{|}~ " + CHR$(192) + CHR$(201) + CHR$(206) + CHR$(213) + CHR$(220) + " " + CHR$(224) + CHR$(233) + CHR$(238) + CHR$(245) + CHR$(252) + " " + CHR$(241) + CHR$(231)
f.Name = "MS Sans Serif": f.Size = 10: L2.Font = f
f.Size = 12: L3.Font = f
DIM fb AS QFONT
fb.AddStyles 0: L4.Font = fb
DIM fi AS QFONT
fi.AddStyles 1: L5.Font = fi
DIM fa AS QFONT
fa.Name = "Arial": fa.Size = 10: L6.Font = fa
DIM ft AS QFONT
ft.Name = "Times New Roman": ft.Size = 12: L7.Font = ft
DIM fc AS QFONT
fc.Name = "Courier New": fc.Size = 10: L8.Font = fc
DIM fu AS QFONT
fu.AddStyles 2: L9.Font = fu
Form.ShowModal
