' RapidQ's `Obj.Method = a, b, …`: the method called, its first argument
' `= a` — a number as itself, text as "" — and `x = = 5` alike (RC.EXE;
' RapidQ's games/qmorp/QMORP.BAS draws `fenetrejeu.fillrect = 20, 20, …`).
$APPTYPE CONSOLE
SUB Foo (a AS INTEGER, b AS INTEGER)
  PRINT "foo "; a; " "; b
END SUB
TYPE TT EXTENDS QOBJECT
  SUB Bar (a AS INTEGER, b AS INTEGER)
    PRINT "bar "; a; " "; b
  END SUB
  SUB Baz (s AS STRING)
    PRINT "baz ["; s; "]"
  END SUB
END TYPE
DIM v AS TT
v.Bar = 3, 4
v.Bar = 3 + 1, 4
v.Bar 5 = 6, 7
v.Bar = 2 * 3, 4
v.Bar = -3, 4
v.Bar = 1 = 1, 4
v.Baz = "hi"
Foo = 3, 4
DIM B AS QBITMAP
B.Width = 40: B.Height = 40
B.FillRect = 10, 10, 20, 20, &HFF
PRINT HEX$(B.Pixel(5, 15)); " "; HEX$(B.Pixel(15, 15)); " "; HEX$(B.Pixel(0, 15))
DIM L AS QSTRINGLIST
L.AddItems = "a", "b", "c"
PRINT L.ItemCount; " ["; L.Item(0); "] "; L.Item(2)
L.AddItems = "x"
PRINT L.ItemCount; " ["; L.Item(3); "]"
x = = 5
PRINT "x "; x
s$ = = "hi"
PRINT "s ["; s$; "]"
