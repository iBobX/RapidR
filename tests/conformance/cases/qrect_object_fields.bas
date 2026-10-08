' QRECT, QNOTIFYICONDATA and QFONT as fields of an object (TYPE or STRUCT
' EXTENDS QOBJECT): RapidQ's compiler (RC.EXE) takes them there, each
' instance with its own, as a DIMmed one behaves — 0 until set, 32-bit
' integers cut toward zero, a SUB gets the field by reference, the TYPE's
' own code reaches it through This / the TYPE's name, Canvas.CopyRect takes
' it (checked against RC.EXE; RapidQ's direct3d examples' QD3DCloneMesh).
$APPTYPE CONSOLE
TYPE TT EXTENDS QOBJECT
  A AS INTEGER
  R AS QRECT
  B AS INTEGER
  SUB Setr (n AS INTEGER)
    This.R.Left = n
    TT.R.Top = n + 1
  END SUB
  FUNCTION W AS INTEGER
    Result = TT.R.Right - TT.R.Left
  END FUNCTION
END TYPE
STRUCT SS EXTENDS QOBJECT
  N AS QNOTIFYICONDATA
  F AS QFONT
  D AS QRECT
  S AS QRECT
END STRUCT
SUB Widen (X AS QRECT)
  PRINT "in "; X.Left
  X.Left = 9
END SUB
DIM v AS TT
PRINT "L="; v.R.Left; " T="; v.R.Top; " R="; v.R.Right; " B="; v.R.Bottom
v.A = 1: v.B = 2
v.R.Left = 3.7
v.R.Right = -2.5
v.R.Bottom = 2147483648
PRINT v.R.Left; " "; v.R.Right; " "; v.R.Bottom; " "; v.A; " "; v.B
v.R.Top = "12"
PRINT "top "; v.R.Top
DIM u AS TT
u.R.Left = 77
PRINT "u "; u.R.Left; " v "; v.R.Left
v.R.Left = 4
Widen v.R
PRINT "after "; v.R.Left
v.Setr 10
v.R.Right = 25
PRINT v.R.Left; " "; v.R.Top; " "; v.W
DIM s AS SS
PRINT s.N.cbSize; " "; s.N.uID; " "; s.F.Name; " "; s.F.Size
s.N.szTip = "hello"
PRINT s.N.szTip
DIM B AS QBITMAP
DIM C AS QBITMAP
B.Width = 40: B.Height = 40
C.Width = 40: C.Height = 40
C.FillRect 0, 0, 40, 40, &HFF
s.D.Right = 10: s.D.Bottom = 10
s.S.Right = 10: s.S.Bottom = 10
B.CopyRect s.D, C, s.S
PRINT HEX$(B.Pixel(5, 5)); " "; HEX$(B.Pixel(30, 30))
