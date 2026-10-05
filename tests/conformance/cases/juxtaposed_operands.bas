' Operands side by side with no operator between (`A B OR C`, `-9(COS(x))`),
' as RapidQ's compiler (RC.EXE) reads them: its operator stack runs on, the
' value is the operand stack's bottom, the other operands are still worked
' out (a FUNCTION among them is called). Every line checked against RC.EXE.
$APPTYPE CONSOLE
CONST A = 1
CONST B = 2
CONST C = 4
DIM x AS INTEGER
DIM y AS DOUBLE
DIM n AS INTEGER
DIM s AS STRING
DIM i AS INTEGER
DIM arr(10) AS INTEGER
SUB Show(v AS INTEGER)
  PRINT "sub "; v
END SUB
SUB Place(px AS DOUBLE, py AS DOUBLE, pz AS DOUBLE)
  IF px < -9.84 AND px > -9.85 THEN PRINT "place -9.84.. "; py; " "; pz
END SUB
FUNCTION Side(v AS INTEGER) AS INTEGER
  n = n + 1
  Side = v
END FUNCTION
x = A B OR C
PRINT "assign "; x
PRINT "print "; A B OR C
Show(A A OR B OR C)
y = -9(COS(3.14159265))
PRINT "neg "; y
PRINT "num "; 2(3)
Place((-9-(SIN(1))), (-9(COS(3.14159265))), -4)
PRINT "p1 "; -2(3)
PRINT "p2 "; 10 - 2 3
PRINT "p3 "; 2 * 3 + 4 5
PRINT "p4 "; 1 + 2 3 * 4
PRINT "p5 "; (2 3) + 1
PRINT "p6 "; 2 + (3 4)
PRINT "p7 "; 5 - (3 4) * 2
PRINT "p8 "; SIN(0) 5
PRINT "p9 "; 2 ^ 3 4
PRINT "p10 "; NOT 0 5
PRINT "p11 "; 3 - -2(4)
PRINT "p12 "; 1 = 1 2
PRINT "p13 "; 7 2 - 1
PRINT "p14 "; 2 * 3 4 * 5
PRINT "p15 "; 9 - 2 * 3 4
PRINT "str "; "a" "b"
s = "x" + "y" "z"
PRINT "concat "; s
PRINT "call "; 1 Side(5); " calls "; n
FOR i = 1 TO 3 5
  PRINT "for "; i
NEXT
IF 0 1 THEN PRINT "if then" ELSE PRINT "if else"
arr(2) = 7
arr(3) = 8
PRINT "index "; arr(2 3)
PRINT "arg "; MID$("abcdef", 2 3, 2)
