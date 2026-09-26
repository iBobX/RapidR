' Arrays of components (DIM lbl(1 TO 3) AS QLABEL): one component per element,
' ids lbl(1)… (also 2-D and local arrays); properties through lbl(i).

DIM lbl(1 TO 3) AS QLABEL
DIM grid(1, 2) AS QEDIT
DIM i AS INTEGER
FOR i = 1 TO 3
  lbl(i).Caption = "L" + STR$(i)
  lbl(i).Top = i * 20
NEXT i
grid(1, 2).Text = "cell"
PRINT lbl(2).Caption; lbl(3).Top; " "; grid(1, 2).Text; " "; lbl(1)
SUB Show(n AS INTEGER)
  DIM loc(2) AS QBUTTON
  loc(n).Caption = "b" + STR$(n)
  PRINT loc(n).Caption; loc(n)
END SUB
Show 2
