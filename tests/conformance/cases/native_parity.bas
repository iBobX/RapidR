' Constructs from the RapidQ example corpus that both backends must treat
' alike (native builds used to reject them).

' A number written with a leading dot, also next to a WITH member.
TYPE TPoint
  x AS DOUBLE
  y AS DOUBLE
END TYPE
DIM p AS TPoint
WITH p
  .x = .5
  .y = .25 + .x
END WITH
PRINT p.x; " "; p.y; " "; .75 * 2

' A variable named like a builtin.
RGB$ = "#" + MID$("00FF8040", 7, 2)
PRINT RGB$

' An array declared with a type suffix, and REDIM of a module-level array
' inside a SUB.
DEFSTR Rider$(1 TO 3) = {"a", "b", "c"}
PRINT Rider$(2)
REDIM Names(1 TO 2) AS STRING
SUB Grow(n)
  REDIM Names(1 TO n) AS STRING
  Names(n) = "last"
END SUB
Grow 4
PRINT UBOUND(Names); " "; Names(4)

' A SUB written inside another SUB is a routine of the program.
SUB Outer
  PRINT "outer"
  SUB Inner
    PRINT "inner"
  END SUB
END SUB
Outer
Inner

' A method named Init doesn't replace the TYPE's own setup.
TYPE TCounter
  n AS INTEGER
  SUB Init(start AS INTEGER)
    This.n = start
  END SUB
END TYPE
DIM c AS TCounter
c.Init 41
c.n = c.n + 1
PRINT c.n

' Arguments fitted to the parameters: extra ones dropped, missing ones empty.
FUNCTION Pair$(a, b)
  Pair$ = "[" + STR$(a) + "|" + STR$(b) + "]"
END FUNCTION
PRINT Pair$(1, 2, 3); Pair$(7)

' A FUNCTION named with a suffix, called without it.
FUNCTION Greeting$
  Greeting$ = "hi"
END FUNCTION
PRINT Greeting
