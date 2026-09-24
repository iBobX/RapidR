' FOR/STEP, WHILE, DO/LOOP, SELECT CASE, EXIT FOR, IF/ELSEIF — core control flow.
DIM i AS INTEGER
DIM s AS STRING
s = ""
FOR i = 1 TO 5
  s = s + STR$(i)
NEXT i
PRINT "for:" + s
s = ""
FOR i = 10 TO 1 STEP -3
  s = s + STR$(i) + ","
NEXT i
PRINT "step:" + s
i = 0
WHILE i < 3
  i = i + 1
WEND
PRINT "while:" + STR$(i)
i = 0
DO
  i = i + 2
LOOP UNTIL i >= 7
PRINT "do-until:" + STR$(i)
FOR i = 1 TO 100
  IF i = 4 THEN EXIT FOR
NEXT i
PRINT "exit-for:" + STR$(i)
FOR i = 1 TO 4
  IF i = 1 THEN
    PRINT "if:one"
  ELSEIF i = 2 THEN
    PRINT "if:two"
  ELSE
    PRINT "if:other"
  END IF
NEXT i
IF i > 100 THEN PRINT "single:yes" ELSE PRINT "single:no"
FOR i = 1 TO 3
  SELECT CASE i
    CASE 1
      PRINT "case:1"
    CASE 2, 3
      PRINT "case:2or3"
    CASE ELSE
      PRINT "case:else"
  END SELECT
NEXT i
