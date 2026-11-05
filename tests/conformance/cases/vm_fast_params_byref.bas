' Typed parameters converted on entry in place (the interpreter's
' ConvLocal / ConvArgLocal): a BYVAL INTEGER rounds half to even, a DOUBLE
' takes an INTEGER; a FUNCTION's result as it is; a typed variable a BYREF
' parameter writes text into keeps working as before (the fused
' instructions' generic path); recursion and BYREF write-back. The answers
' are the interpreter's before (2.122) and native builds'.
FUNCTION Half(n AS INTEGER) AS INTEGER
  Result = n / 2
END FUNCTION

FUNCTION Scale(x AS DOUBLE, k AS INTEGER) AS DOUBLE
  Result = x * k
END FUNCTION

FUNCTION Fact(n AS INTEGER) AS DOUBLE
  IF n <= 1 THEN
    Result = 1
  ELSE
    Result = n * Fact(n - 1)
  END IF
END FUNCTION

SUB Bump(BYREF v AS INTEGER, BYVAL by AS INTEGER)
  v = v + by
END SUB

SUB SetText(BYREF t)
  t = "7x"
END SUB

FUNCTION Clamp(x AS DOUBLE, lo AS DOUBLE, hi AS DOUBLE) AS DOUBLE
  IF x < lo THEN
    Clamp = lo
  ELSEIF x > hi THEN
    Clamp = hi
  ELSE
    Clamp = x
  END IF
END FUNCTION

DIM n AS INTEGER, d AS DOUBLE, i AS INTEGER
PRINT "half "; Half(5); " "; Half(2.5); " "; Half(3.5); " "; Half(-2.5); " "; Half(7.7)
PRINT "scale "; Scale(3, 2.5); " "; Scale(0.1, 3); " "; Scale(1E300, 10)
PRINT "fact "; Fact(10); " "; Fact(20)
n = 5: Bump n, 3: PRINT "bump "; n
n = 2147483647: Bump n, 1: PRINT "bump over "; n
n = 3: SetText n: PRINT "byref text "; n; " "; n + 1; " "; n * 2
d = 0
FOR i = 1 TO 1000: d = d + Clamp((i MOD 30) - 10.0, 0.0, 15.0): NEXT
PRINT "clamp "; d
