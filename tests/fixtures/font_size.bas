' Font sizes are points, as in RapidQ (Windows: 96 dpi, whole pixels —
' 12 pt is 16 pixels): a label's text and a canvas's TextWidth / TextHeight
' are the same size on every runtime.
DECLARE SUB Clicked
CREATE f AS QFORM
  CREATE l AS QLABEL
    Caption = "Hello" : FontSize = 12 : Top = 60 : Width = 200
  END CREATE
  CREATE c AS QCANVAS
    Top = 90 : Width = 200 : Height = 40
  END CREATE
  CREATE btn AS QBUTTON
    OnClick = Clicked
  END CREATE
  CREATE lbl AS QLABEL
    Top = 30 : Width = 300
  END CREATE
END CREATE
f.ShowModal

SUB Clicked
  c.Font.Size = 12
  lbl.Caption = STR$(c.TextWidth("Hello")) + "x" + STR$(c.TextHeight("Hello"))
  c.Font.Size = 10
  lbl.Caption = lbl.Caption + " " + STR$(c.TextWidth("Hello")) + "x" + STR$(c.TextHeight("Hello"))
END SUB
