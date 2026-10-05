' Storing a number into a typed variable as RapidQ does it — the .expected
' is RC.EXE's output (docs/rapidq-ground-truth.md): a store truncates toward
' zero (variables, array elements, TYPE fields, FOR's start, INC / DEC, SWAP,
' READ), beyond 32 bits it is -2147483648, BYTE / WORD / SHORT then wrap,
' DWORD is 32-bit signed; a BYVAL parameter rounds half to even; a FUNCTION
' returns what it was given, unconverted.
DIM i AS INTEGER
DIM l AS LONG
DIM w AS WORD
DIM b AS BYTE
DIM sh AS SHORT
DIM dw AS DWORD
i = 2.5: PRINT "a "; i;
i = 3.5: PRINT " "; i;
i = -2.5: PRINT " "; i;
i = 2.7: PRINT " "; i;
i = -2.7: PRINT " "; i
l = 1.5: PRINT "b "; l;
l = 0.5: PRINT " "; l;
l = 3000000000: PRINT " "; l;
i = 1E10: PRINT " "; i
b = 300: PRINT "c "; b;
b = -1: PRINT " "; b;
b = 2.5: PRINT " "; b;
b = 300.7: PRINT " "; b
w = 70000: PRINT "d "; w;
w = -1.5: PRINT " "; w;
sh = 40000: PRINT " "; sh;
sh = 1E10: PRINT " "; sh
dw = -1: PRINT "e "; dw;
dw = 4294967295: PRINT " "; dw;
dw = 3000000000: PRINT " "; dw;
dw = 2.7: PRINT " "; dw
i = 2147483647
i = i + 1
PRINT "f "; i
DIM a(3) AS INTEGER
a(1) = 2.7: a(2) = -2.7: a(3) = 2.5
PRINT "g "; a(1); " "; a(2); " "; a(3)
TYPE TR
  n AS INTEGER
  b AS BYTE
END TYPE
DIM r AS TR
r.n = 2.7: r.b = 300.7
PRINT "h "; r.n; " "; r.b
FOR i = 1.5 TO 3
  PRINT "i "; i
NEXT
i = 5
INC i, 2.7
PRINT "j "; i;
DEC i, 0.5
PRINT " "; i
DIM d AS DOUBLE
i = 0: d = 2.7
SWAP i, d
PRINT "k "; i; " "; d
READ i
PRINT "l "; i
DATA 3.7
SUB P (n AS INTEGER)
  PRINT " "; n;
END SUB
PRINT "m";
P 2.5: P 3.5: P -2.5: P 2.7: P -2.7: P 1E10
PRINT
FUNCTION F (x AS DOUBLE) AS INTEGER
  F = x
END FUNCTION
FUNCTION G (x AS DOUBLE) AS SINGLE
  RESULT = x
END FUNCTION
PRINT "n "; F(2.7); " "; F(-2.7); " "; G(0.1)
i = F(2.7)
PRINT "o "; i
