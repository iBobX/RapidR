' & is RapidQ's +: two numbers add (`5 & 3` is 8), two strings join, at +'s
' own precedence — and && is the same operator (the second & before an
' operand is dropped, as `&x` is x): `IF x = 3 && x = 3` reads `x = (3 + x) = 3`,
' which is never true. The .expected is RC.EXE's output
' (docs/rapidq-ground-truth.md). A string with a number (`"a" & 1`) is
' RapidQ's mix-up (it returns the right-hand operand); RapidR joins the text
' there, so those lines aren't here.
$APPTYPE CONSOLE
DIM a AS INTEGER, b AS INTEGER, c AS INTEGER
DIM x AS INTEGER
DIM s AS STRING

a = 5
b = 3
PRINT 5 & 3
PRINT "5" & "3"
PRINT a & b
c = a & b
PRINT c
PRINT 1 && 2
PRINT 1 &&& 2
PRINT 1 & & 2
PRINT && 2
PRINT "a" && "b"
PRINT "a" & "b" & "c"
PRINT "a" && "b" && "c"
PRINT (1 = 1) && (2 = 2)
PRINT 5 & -3
PRINT 1.5 & 2.25
PRINT &H10 & 1
PRINT STR$(5) & STR$(3)
s = STR$(a) & STR$(b)
PRINT s
' (+'s precedence: the same level as + and -, left to right)
PRINT 1 + 2 & 3 + 4
PRINT 10 - 3 & 2
PRINT 2 * 3 & 4 * 5
PRINT 2 & 3 * 4
PRINT 8 MOD 3 & 1
PRINT 1 < 2 & 1
' (a condition with && never holds for x = 3: `x = (3 && x) = 3`)
x = 3
IF x = 3 && x = 3 THEN
  PRINT "yes"
ELSE
  PRINT "no"
END IF
IF x = 3 & x = 3 THEN
  PRINT "yes"
ELSE
  PRINT "no"
END IF
IF x = 3 AND x = 3 THEN
  PRINT "yes"
ELSE
  PRINT "no"
END IF
IF (x = 3) && (x = 3) THEN
  PRINT "yes"
ELSE
  PRINT "no"
END IF
IF (x = 4) && (x = 4) THEN
  PRINT "yes"
ELSE
  PRINT "no"
END IF
x = 0
IF x = 0 && x = 0 THEN
  PRINT "yes"
ELSE
  PRINT "no"
END IF
