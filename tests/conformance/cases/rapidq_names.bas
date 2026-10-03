' Names as RapidQ keeps them apart: `i%` and `i$` are two variables; a
' variable spelled like a function with another suffix (`day&` in FUNCTION
' Day) isn't the function's result; a FOR loop's undeclared variable in a
' SUB is the SUB's own. And REPLACE$ writes over a string at a position
' (REPLACE$("Hello", "J", 1) is "Jello"; find-and-replace is REPLACESUBSTR$).
DIM i% AS INTEGER, i$ AS STRING
i$ = "abc"
FOR i% = 1 TO 3
  PRINT i%; MID$(i$, i%, 1);
NEXT
PRINT
k$ = "s": k% = 7: PRINT k$; k%
FUNCTION Day(d$) AS INTEGER
  DEFINT day& = VAL(d$)
  Day = day& + 100
END FUNCTION
PRINT Day("5")
SUB Inner
  FOR i = 1 TO 2
  NEXT
END SUB
n = 0
FOR i = 1 TO 4
  n = n + 1
  Inner
  IF n > 20 THEN EXIT FOR
NEXT
PRINT "n"; n
PRINT REPLACE$("Hello", "J", 1); " "; REPLACE$("abcdef", "XY", 2); " "; REPLACE$("abc", "XYZ", 3); " "; REPLACE$("abc", "Z", 9)
a$ = SPACE$(5)
a$ = REPLACE$(a$, "5", 3)
PRINT "["; a$; "] "; REPLACESUBSTR$("a-b-c", "-", "+")
