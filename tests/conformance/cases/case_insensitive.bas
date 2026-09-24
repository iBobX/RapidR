' xfail: vm — VM resolves variable/SUB names case-sensitively (silent miscompile)
' BASIC identifiers are case-insensitive.
DIM Total AS INTEGER
total = 5
PRINT "global:" + STR$(TOTAL)
SUB Show()
  DIM Count AS INTEGER
  count = 7
  PRINT "local:" + STR$(COUNT)
END SUB
show
SHOW()
CALL Show
DIM Arr(3) AS INTEGER
arr(1) = 9
PRINT "array:" + STR$(ARR(1))
