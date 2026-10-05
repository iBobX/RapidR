' RapidQ syntax from its examples: STRUCT … END STRUCT (a TYPE), "" inside
' a string is no quote — RC.EXE reads `"[:"":>"` as two strings side by
' side, worth the first ("[:") — ?/??/??? type suffixes on numbers, and a
' `_` continuation inside a $ESCAPECHARS string.
STRUCT Pt
  x AS INTEGER
  y AS INTEGER
END STRUCT
DIM p AS Pt
p.x = 3 : p.y = 4
PRINT p.x + p.y
a$ = "[:"":>"
PRINT a$; LEN(a$)
PRINT """"; LEN(""""); "|"; LEN("")
w = 0?? + 1& + 2?
PRINT w
$ESCAPECHARS ON
PRINT "a, _
  b\t!"
