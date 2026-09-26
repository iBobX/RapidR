' Function pointers (RapidQ manual: BIND, CALLFUNC, CODEPTR): a pointer is
' 0 when unset, and CALLFUNC calls SUBs and FUNCTIONs through it.
SUB Hello (n AS INTEGER)
  PRINT "hello"; n
END SUB
FUNCTION Twice (n AS INTEGER) AS INTEGER
  Twice = n * 2
END FUNCTION
DIM p AS INTEGER, q AS INTEGER
PRINT "unset:"; p
BIND p TO Hello
BIND q TO Twice
CALLFUNC(p, 7)
PRINT CALLFUNC(q, 21)
r = CODEPTR(Twice)
PRINT CALLFUNC(r, 5) + 1; " "; (r > 0)
' A pointer to a TYPE's method: the instance is its first argument.
TYPE TAcc
  Total AS INTEGER
  SUB AddUp (n AS INTEGER)
    Total = Total + n
  END SUB
END TYPE
DIM acc AS TAcc
m = CODEPTR(acc.AddUp)
CALLFUNC(m, acc, 5)
BIND q TO TAcc.AddUp
CALLFUNC(q, acc, 2)
PRINT "total"; acc.Total
