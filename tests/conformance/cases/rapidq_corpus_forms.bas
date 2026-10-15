' Forms RapidQ's example programs use that RC.EXE takes (the .expected is
' RC.EXE's output): `&` before a name is the name (CallBack_*.inc's
' `.WriteNum(&ForwardTo, 4)`), `DEFINT R()` an array REDIM sizes later
' (RC.EXE: R(0)), a `$` inside a name (`Encrypted$YY`, a variable of its
' own), a DECLARE inside a block declares for the whole program, a
' DECLAREd FUNCTION's parameters have the DECLARE's names (the header's
' are other, undeclared names), SizeOf(Short) with $TYPECHECK ON, and a
' variable named Open passed to a SUB without parentheses.
$APPTYPE CONSOLE
DECLARE FUNCTION Twice (a AS LONG) AS LONG
FUNCTION Twice (b AS LONG) AS LONG
  Result = a * 2 + b
END FUNCTION
SUB Show (p AS INTEGER, q AS INTEGER)
  PRINT p; "/"; q
END SUB
DIM x AS LONG
x = 5
PRINT &x + 1
DIM s AS STRING
s = "abc"
PRINT &s
DefInt R()
PRINT UBOUND(R); LBOUND(R)
REDIM R(3) AS INTEGER
R(2) = 9
PRINT R(2); UBOUND(R)
DIM E$YY AS STRING
E$YY = "hi"
PRINT E$YY; LEN(E$YY); "["; E$; "]"
PRINT Twice(3)
IF x = 5 THEN
  DECLARE FUNCTION Half (n AS LONG) AS LONG
END IF
PRINT Half(8)
DIM Open AS INTEGER
Open = 7
Show Open, 1
Show 2, Open
$TYPECHECK ON
DIM sz AS INTEGER
sz = SizeOf(Short) + SizeOf(Integer)
PRINT sz
$TYPECHECK OFF
FUNCTION Half (m AS LONG) AS LONG
  Result = n / 2
END FUNCTION
