' Type suffixes declare types (manual: ? BYTE, ?? WORD, % SHORT, & LONG,
' ! SINGLE, # DOUBLE, $ STRING), undeclared or DIM'd without AS
q% = 40000
PRINT q%
b? = 300
w?? = 70000
PRINT b?, w??
DIM n&, s%(3)
n& = 2.5
s%(1) = 32768
PRINT n&, s%(1)
x# = 1.5
PRINT x#
FUNCTION Half% (v%)
  Half% = v% / 2
END FUNCTION
PRINT Half%(7)
DIM k AS LONG
k% = 70000
PRINT k
FOR i? = 254 TO 255
  PRINT i?;
NEXT
PRINT
