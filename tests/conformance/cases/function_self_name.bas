' A FUNCTION's own name inside it, as RapidQ's compiler (RC.EXE) has it:
' without parameters the name is a call of itself (G = G + 1 recurses);
' RESULT reads and sets the result; a call with arguments recurses.
DIM n AS INTEGER
FUNCTION G AS INTEGER
  n = n + 1
  IF n > 3 THEN G = 100 : EXIT FUNCTION
  G = 5
  G = G + 1
END FUNCTION

FUNCTION H(x AS INTEGER) AS INTEGER
  Result = 1
  Result = Result + x
  H = Result * 10
END FUNCTION

FUNCTION K(x AS INTEGER) AS INTEGER
  IF x <= 0 THEN K = 0 : EXIT FUNCTION
  K = K(x - 1) + x
END FUNCTION

PRINT G; " "; n
PRINT H(2)
PRINT K(4)
