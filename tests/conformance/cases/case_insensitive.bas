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
