' GOTO/GOSUB mixed with structured code: labels inside FOR/WHILE/DO/IF
' bodies, GOSUB inside IF and loops, EXIT FOR out of a loop with a label,
' GOTO out of SELECT CASE, DIM after a label, a FUNCTION using GOSUB.
FOR i = 1 TO 5
  IF i = 2 THEN GOTO NextI
  IF i = 4 THEN GOSUB Shout
  PRINT "i ="; i
NextI:
NEXT i
n = 0
WHILE n < 10
  n = n + 1
  IF n MOD 2 = 0 THEN GOTO Skip
  IF n > 6 THEN EXIT WHILE
  PRINT "odd"; n
Skip:
WEND
k = 0
DO
  k = k + 1
  SELECT CASE k
    CASE 3
      GOTO DoneDo
  END SELECT
Again:
LOOP UNTIL k > 10
DoneDo:
PRINT "k ="; k
DIM total AS INTEGER
FOR j = 1 TO 3
  GOSUB AddJ
  IF j = 2 THEN EXIT FOR
Mark:
NEXT j
PRINT "total ="; total
PRINT "fact ="; Fact(5)
Counter
END

Shout:
  PRINT "four!"
RETURN

AddJ:
  total = total + j * 10
RETURN

FUNCTION Fact (n AS INTEGER) AS INTEGER
  DIM acc AS INTEGER
  acc = 1
  m = n
Loop1:
  IF m <= 1 THEN GOTO Done
  GOSUB Multiply
  m = m - 1
  GOTO Loop1
Multiply:
  acc = acc * m
  RETURN
Done:
  Fact = acc
END FUNCTION

SUB Counter
  c = 0
Up:
  c = c + 1
  IF c < 3 THEN GOTO Up
  DIM msg AS STRING
  msg = "counted to"
  PRINT msg; c
  RETURN
  PRINT "never"
END SUB
