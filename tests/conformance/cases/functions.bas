' xfail: vm, codegen — VM: return-by-name (Fact = ...) returns empty, BYREF ignored; codegen: BYREF param fails to compile
' SUB/FUNCTION calls, recursion, RETURN, return-by-name, BYREF.
FUNCTION Fact(n AS INTEGER) AS INTEGER
  IF n <= 1 THEN
    Fact = 1
  ELSE
    Fact = n * Fact(n - 1)
  END IF
END FUNCTION

FUNCTION Twice(x AS INTEGER) AS INTEGER
  RETURN x * 2
END FUNCTION

SUB Bump(BYREF v AS INTEGER)
  v = v + 1
END SUB

SUB Greet(who AS STRING)
  PRINT "hello " + who
END SUB

DIM k AS INTEGER
k = 5
Bump k
PRINT "fact:" + STR$(Fact(5))
PRINT "twice:" + STR$(Twice(21))
PRINT "byref:" + STR$(k)
Greet "world"
CALL Greet("again")
