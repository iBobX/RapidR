' Typed main-program variables and BYVAL parameters: native builds keep
' them as Rust numbers (rapidr-codegen-rust `typed`), the interpreter as
' Values. Both must print exactly the same.
DIM total AS LONG, scale AS DOUBLE, b AS BYTE, i AS INTEGER, j AS INTEGER, passed AS LONG, fr AS SHORT
DECLARE SUB AddUp (n AS LONG, f AS DOUBLE)
DECLARE FUNCTION Half (x AS DOUBLE) AS DOUBLE
DECLARE SUB Bump (BYREF v AS LONG)
DECLARE SUB Own
SUB AddUp (n AS LONG, f AS DOUBLE)
  total = total + n
  scale = scale * f
  n = n * 2.5
  PRINT "in AddUp n="; n; " total="; total
END SUB
FUNCTION Half (x AS DOUBLE) AS DOUBLE
  x = x / 2
  Half = x
END FUNCTION
SUB Bump (BYREF v AS LONG)
  v = v + 1
END SUB
SUB Own
  DIM total AS STRING
  total = "mine"
  PRINT "own "; total
END SUB
scale = 1
FOR i = 1 TO 5
  AddUp i, 1.5
NEXT
PRINT total; " "; scale; " "; Half(7)
b = 250
FOR j = 1 TO 10
  b = b + 1
NEXT
PRINT "byte "; b
Own
PRINT "after own "; total
passed = 41
Bump passed
PRINT "passed "; passed
DIM total AS LONG
PRINT "reset "; total
i = 0
again:
i = i + 1
IF i < 3 THEN GOTO again
PRINT "goto "; i
FOR fr = 1 TO 3
  IF fr = 2 THEN GOSUB sub1
NEXT
PRINT "fr "; fr
DIM got AS LONG
INPUT "number? ", got
PRINT "got "; got + 1
' A typed global read as a property value inside CREATE.
DEFINT bw
bw = 7 * 3
CREATE Pnl AS QPANEL
  Width = bw: Height = got + 1
END CREATE
PRINT "panel "; Pnl.Width; " "; Pnl.Height
END
sub1:
PRINT "gosub at "; fr
RETURN
