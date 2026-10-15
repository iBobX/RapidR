' RAPIDQ.INC's QBColor: an INTEGER array QBColor(0 TO 15) of QBasic's 16
' colours as &HBBGGRR numbers, assignable (a store truncated as an
' INTEGER's). Included twice it is declared once (the file guards itself).
' The .expected is RC.EXE's output (docs/rapidq-ground-truth.md).
$APPTYPE CONSOLE
$INCLUDE "RAPIDQ.INC"
$INCLUDE "RAPIDQ.INC"
DIM I AS INTEGER
FOR I = 0 TO 15
  PRINT I; " "; QBColor(I); " &H"; HEX$(QBColor(I))
NEXT
PRINT "bounds "; LBOUND(QBColor); " "; UBOUND(QBColor)
QBColor(3) = 70000
PRINT "set "; QBColor(3)
QBColor(4) = 2.7
PRINT "int "; QBColor(4)
PRINT "red "; QBColor(12) = clRed
