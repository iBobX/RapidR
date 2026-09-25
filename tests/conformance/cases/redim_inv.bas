' REDIM keeps the data (RapidQ manual, REDIM; VB's REDIM PRESERVE too),
' creates the array when there was no DIM, works in SUBs; INV is the
' modular inverse (manual, operators: 3 INV 26 = 9).
DIM B(2) AS INTEGER
B(1) = 11 : B(2) = 22
REDIM B(5) AS INTEGER
B(5) = 55
PRINT B(1); B(2); B(5); UBOUND(B)
REDIM PRESERVE B(1) AS INTEGER
PRINT B(1); UBOUND(B)
REDIM Fresh(1 TO 3) AS STRING
Fresh(3) = "three"
PRINT Fresh(3); LBOUND(Fresh)
SUB Grow
  DIM L(1) AS INTEGER
  L(1) = 7
  REDIM L(4) AS INTEGER
  PRINT L(1); UBOUND(L)
END SUB
Grow
PRINT 3 INV 26; " "; 2 INV 4; " "; 10 - 3 INV 26
