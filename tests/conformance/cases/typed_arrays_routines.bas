' Typed arrays and routines (rapidr-codegen-rust typed_arrays): main-program
' arrays used in SUBs and FUNCTIONs, a routine's own arrays (a new one each
' call, each DIM), array parameters (the caller's array, REDIM included),
' a parameter of another type or none (the array stays a Value there), a
' routine with GOTO, a local array shadowing a global one, an array that is
' also used whole (it stays a Value), elements passed to parameters. The
' interpreter and native builds print the same.
$APPTYPE CONSOLE
CONST N = 4
DIM Wave(N - 1) AS DOUBLE
DIM Gain(N - 1) AS DOUBLE
DIM Counts(1 TO 3) AS INTEGER
DIM Mixed(2) AS DOUBLE
DIM Bytes(3) AS BYTE
DIM Marks(1) AS DOUBLE
DIM i AS INTEGER, total AS DOUBLE

SUB Fill(a() AS DOUBLE, start AS DOUBLE)
  DIM k AS INTEGER
  FOR k = LBOUND(a) TO UBOUND(a)
    a(k) = start + k / 2
  NEXT
END SUB

FUNCTION Sum(a() AS DOUBLE) AS DOUBLE
  DIM k AS INTEGER, s AS DOUBLE
  FOR k = LBOUND(a) TO UBOUND(a)
    s = s + a(k)
  NEXT
  Sum = s
END FUNCTION

' A parameter passed on to another routine
FUNCTION Twice(a() AS DOUBLE) AS DOUBLE
  Twice = Sum(a) * 2
END FUNCTION

SUB Grow(a() AS DOUBLE, upper AS INTEGER)
  REDIM a(upper) AS DOUBLE
  a(upper) = -1
END SUB

' A routine's own arrays: a new one each call (and each DIM)
FUNCTION Depth(level AS INTEGER) AS INTEGER
  DIM mine(2) AS INTEGER
  mine(0) = mine(0) + level
  IF level < 3 THEN mine(1) = Depth(level + 1)
  Depth = mine(0) * 10 + mine(1)
END FUNCTION

SUB Loops
  DIM k AS INTEGER
  FOR k = 1 TO 3
    DIM acc(1) AS LONG
    acc(0) = acc(0) + k
    PRINT "acc"; acc(0);
  NEXT
  PRINT
END SUB

' A parameter of another type, and one without a type
SUB AsIntegers(a() AS INTEGER)
  a(0) = 2.75
END SUB

SUB Untyped(a())
  a(1) = "7"
END SUB

' The array by its own name and as a parameter: one array
SUB Both(x() AS DOUBLE)
  x(0) = 5
  PRINT "both"; Wave(0); x(0)
  Wave(1) = 6
  PRINT "both"; x(1)
END SUB

' GOTO: its own arrays stay Values; the main program's are still typed
SUB Jumps
  DIM k AS INTEGER, own(2) AS DOUBLE
  k = 0
again:
  own(k) = Gain(k) + 0.5
  k = k + 1
  IF k < 3 THEN GOTO again
  PRINT "jumps"; own(0); own(2)
END SUB

' A local array of a global's name
SUB Shadow
  DIM Gain(1) AS STRING
  Gain(0) = "text"
  PRINT "shadow "; Gain(0)
END SUB

' The main program's array resized in a SUB
SUB Resize
  REDIM Counts(1 TO 5) AS INTEGER
  Counts(5) = 55
END SUB

' A routine with a dotted name, and one called as `Name:`
SUB Draw.Mark(x() AS DOUBLE, v AS DOUBLE)
  x(1) = v
END SUB

SUB Greet
  PRINT "greet"; Marks(1)
END SUB

SUB Show(v AS DOUBLE, r AS INTEGER)
  r = v * 2
  PRINT "show"; v; r
END SUB

Fill Wave, 1
Fill Gain, 0.25
PRINT Sum(Wave); " "; Sum(Gain); " "; Twice(Wave)
FOR i = 0 TO N - 1
  total = total + Wave(i) * Gain(i)
NEXT
PRINT "dot"; total
PRINT "depth"; Depth(1)
Loops
Grow Gain, 6
PRINT UBOUND(Gain); " "; Gain(6); " "; Gain(1); " "; Sum(Gain)
AsIntegers Mixed
Untyped Mixed
PRINT Mixed(0); " "; Mixed(1); " "; Mixed(2)
Both Wave
Jumps
Shadow
PRINT "global "; Gain(0)
Counts(1) = 11
Resize
PRINT UBOUND(Counts); " "; Counts(1); " "; Counts(5)
Show Wave(2), Wave(3)
PRINT "after show"; Wave(3)
Bytes(0) = 200
Bytes(1) = Bytes(0) + Bytes(0)
PRINT Bytes(1); " "; Bytes(0) * 2
Draw.Mark Marks, 9.5
Greet:
