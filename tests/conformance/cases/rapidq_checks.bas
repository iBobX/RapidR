' RapidQ's compile-time checks, in its compiler's words: argument counts,
' a name DIMmed twice in one scope (i% and i$ are two names), RESULT
' outside a FUNCTION. A DECLARE SUB's own signature is accepted too.
DECLARE SUB Tick ()
SUB Tick (Sender AS LONG)
END SUB
FUNCTION Pair$ (a AS LONG, b AS LONG)
  Pair$ = STR$(a) + STR$(b)
END FUNCTION
DIM i% AS INTEGER, i$ AS STRING
DIM total AS LONG
DIM total AS STRING
Tick
PRINT Pair$(1, 2, 3)
PRINT Pair$(7)
SUB Show
  Result = 1
END SUB
