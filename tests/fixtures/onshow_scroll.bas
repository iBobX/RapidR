' OnShow on both runtimes: it fires as ShowModal shows the form, and what
' it adds counts for AutoScroll (four panels past the right edge: a
' horizontal bar, as in Delphi; the canvas below the client: a vertical one).
DECLARE SUB Showed
DECLARE SUB Clicked
DIM p(3) AS QPANEL
CREATE f AS QFORM
  ClientWidth = 350 : ClientHeight = 300 : OnShow = Showed
  CREATE c AS QCANVAS
    Left = 5 : Top = 5 : Width = 270 : Height = 310
  END CREATE
  CREATE btn AS QBUTTON
    Left = 290 : Top = 270 : Width = 40 : Height = 20 : OnClick = Clicked
  END CREATE
  CREATE lbl AS QLABEL
    Left = 290 : Top = 250 : Width = 40
  END CREATE
END CREATE
f.ShowModal

SUB Showed
  DIM i AS INTEGER
  lbl.Caption = lbl.Caption + "show;"
  FOR i = 0 TO 3
    p(i).Left = 320 : p(i).Top = i * 32 + 15 : p(i).Width = 23 : p(i).Height = 23 : p(i).Parent = f
  NEXT
END SUB

SUB Clicked
  lbl.Caption = lbl.Caption + STR$(f.ClientWidth) + "x" + STR$(f.ClientHeight)
END SUB
