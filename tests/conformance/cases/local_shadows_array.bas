' A local that shares a global array's name inside one routine must not
' change what the name means in the routines after it
DIM Names$(1 TO 3)
Names$(1) = "one"
Names$(2) = "two"
Names$(3) = "three"

FUNCTION Shadow (k AS INTEGER) AS INTEGER
  DIM Names$ AS INTEGER
  Names$ = k * 10
  Shadow = Names$
END FUNCTION

SUB Show (k AS INTEGER)
  PRINT Names$(k)
END SUB

PRINT Shadow(4)
Show 2
PRINT Names$(3)
