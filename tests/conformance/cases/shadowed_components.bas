' A parameter or local named like a global component hides it in its
' routine (names are case-insensitive: `b` and a QBITMAP `B`); a component
' declared in the routine stays one.
DIM B AS QBITMAP
B.Width = 4
FUNCTION Twice(b AS INTEGER) AS INTEGER
  Twice = b * 2
END FUNCTION
SUB Show
  DIM b AS STRING
  b = "local"
  PRINT b; " "; LEN(b)
END SUB
SUB Own
  DIM L AS QBITMAP
  L.Width = 7
  PRINT L.Width
END SUB
PRINT Twice(21)
Show
Own
PRINT B.Width
