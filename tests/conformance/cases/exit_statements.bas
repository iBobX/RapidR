' EXIT FOR/WHILE/DO/SUB/FUNCTION leave the right construct.
SUB FindFirstEven()
  DIM k AS INTEGER
  FOR k = 1 TO 10
    IF k MOD 2 = 0 THEN
      PRINT "sub:" + STR$(k)
      EXIT SUB
    END IF
  NEXT k
  PRINT "sub:not reached"
END SUB

FUNCTION Clamp(n AS INTEGER) AS INTEGER
  IF n > 10 THEN
    Clamp = 10
    EXIT FUNCTION
  END IF
  Clamp = n * 2
END FUNCTION

FindFirstEven
PRINT "fn:" + STR$(Clamp(50)) + "," + STR$(Clamp(3))

DIM i AS INTEGER
DIM j AS INTEGER
FOR i = 1 TO 3
  j = 0
  WHILE j < 5
    j = j + 1
    IF j = 2 THEN EXIT FOR
  WEND
NEXT i
PRINT "nested:" + STR$(i) + "," + STR$(j)

j = 0
DO
  j = j + 1
  IF j = 3 THEN EXIT DO
LOOP
PRINT "do:" + STR$(j)
