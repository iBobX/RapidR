' Many place writes in a row on forms with scroll bars, then reads (the
' SPEED-AUDIT's fix: the bars are worked out once, before the next read,
' not after every write): what the program reads must be what it read when
' every write worked them out at once — the .expected is the interpreter's
' output from before the change. A form scrolled and then emptied (its
' position kept in range moves the components), an aligned panel beside
' components that come and go past the edge, a scroll box inside a form.
CREATE F AS QFORM
  Width = 400: Height = 300
END CREATE
CREATE G AS QFORM
  Width = 420: Height = 320
  CREATE P AS QPANEL
    Align = 5
  END CREATE
END CREATE
CREATE H AS QFORM
  Width = 500: Height = 400
  CREATE SBox AS QSCROLLBOX
    Left = 10: Top = 10: Width = 300: Height = 200
  END CREATE
END CREATE
DIM B(9) AS QBUTTON
DIM C(5) AS QBUTTON
DIM D(5) AS QLABEL
DIM i AS INTEGER, cw AS INTEGER, ch AS INTEGER, s AS STRING

FOR i = 0 TO 9
  B(i).Parent = F: B(i).Left = i * 30: B(i).Top = i * 20: B(i).Width = 25: B(i).Height = 18
NEXT
cw = F.ClientWidth: ch = F.ClientHeight
' past the right and the bottom: both bars
FOR i = 0 TO 9
  B(i).Left = 100 + i * 60
  B(i).Top = 50 + i * 40
NEXT
PRINT "bars "; cw - F.ClientWidth; " "; ch - F.ClientHeight; " "; F.HorzRange; " "; F.VertRange
' scrolled to the end, then everything back inside: the positions come
' back in range and the components move with them
F.HorzPosition = 500: F.VertPosition = 400
PRINT "scrolled "; F.HorzPosition; " "; F.VertPosition; " "; B(0).Left; " "; B(0).Top
FOR i = 0 TO 9
  B(i).Left = B(i).Left - 50
  B(i).Width = 20
  B(i).Top = B(i).Top - 30
NEXT
s = ""
FOR i = 0 TO 9
  s = s + STR$(B(i).Left) + STR$(B(i).Top)
NEXT
PRINT "moved"; s
PRINT "after "; F.HorzPosition; " "; F.VertPosition; " "; F.HorzRange; " "; F.VertRange; " "; cw - F.ClientWidth; " "; ch - F.ClientHeight
FOR i = 0 TO 9
  B(i).Left = 5 + i: B(i).Top = 5 + i
NEXT
PRINT "inside "; F.HorzPosition; " "; F.VertPosition; " "; F.HorzRange; " "; F.VertRange; " "; cw - F.ClientWidth; " "; ch - F.ClientHeight

' an alClient panel shares the form with components past its edge: the
' bars take room from it
G.Show
FOR i = 0 TO 5
  C(i).Parent = G: C(i).Left = 10 + i * 20: C(i).Top = 10: C(i).Width = 18: C(i).Height = 18
NEXT
PRINT "panel "; P.Width; " "; P.Height; " "; G.ClientWidth; " "; G.ClientHeight
FOR i = 0 TO 5
  C(i).Left = 300 + i * 100
NEXT
PRINT "wide "; P.Width; " "; P.Height; " "; G.ClientWidth; " "; G.ClientHeight; " "; G.HorzRange
FOR i = 0 TO 5
  C(i).Top = 250 + i * 60
NEXT
PRINT "tall "; P.Width; " "; P.Height; " "; G.ClientWidth; " "; G.ClientHeight; " "; G.VertRange
FOR i = 0 TO 5
  C(i).Left = 10: C(i).Top = 10
NEXT
PRINT "back "; P.Width; " "; P.Height; " "; G.ClientWidth; " "; G.ClientHeight
G.Close

' a scroll box's own components
FOR i = 0 TO 5
  D(i).Parent = SBox: D(i).Caption = "x": D(i).AutoSize = 0
  D(i).Left = i * 70: D(i).Top = i * 50: D(i).Width = 60: D(i).Height = 20
NEXT
PRINT "box "; SBox.ClientWidth; " "; SBox.ClientHeight; " "; SBox.HorzRange; " "; SBox.VertRange
SBox.VertPosition = 100
FOR i = 0 TO 5
  D(i).Height = 10
NEXT
s = ""
FOR i = 0 TO 5
  s = s + STR$(D(i).Top)
NEXT
PRINT "box after "; SBox.VertPosition; " "; SBox.VertRange; s
D(5).Visible = 0: D(4).Visible = 0
PRINT "hidden "; SBox.ClientWidth; " "; SBox.ClientHeight; " "; SBox.HorzRange; " "; SBox.VertRange; " "; SBox.VertPosition
