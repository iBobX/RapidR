' FOR loops as the interpreter's ForTest / ForStep instructions run them
' (docs/vm-fast-design.md): the step's sign tested every turn, an INTEGER
' counter stepping by a fraction (it holds Doubles, as before), a negative
' and a computed step, counters local and global, EXIT FOR, the counter
' after the loop, an end beyond 2^53, comparisons and branches fused with
' their operands. The answers are the interpreter's before (2.122) and
' native builds'.
DIM gi AS INTEGER, gn AS INTEGER, gx AS DOUBLE

SUB Loops
  DIM i AS INTEGER, n AS INTEGER, x AS DOUBLE, s AS DOUBLE
  n = 0
  FOR i = 1 TO 10 STEP 3: n = n + i: NEXT
  PRINT "step3 "; n; " after "; i
  n = 0
  FOR i = 10 TO 1 STEP -2: n = n + i: NEXT
  PRINT "down "; n; " after "; i
  s = 0
  FOR i = 1 TO 3 STEP 0.5: s = s + i: NEXT
  PRINT "half "; s; " after "; i
  s = 0
  FOR x = 0 TO 1 STEP 0.1: s = s + x: NEXT
  PRINT "tenths "; s; " after "; x
  n = 0
  FOR i = 5 TO 1: n = n + 1: NEXT
  PRINT "never "; n; " after "; i
  n = 0
  FOR i = 1 TO 100
    IF i * i > 50 THEN EXIT FOR
    n = n + 1
  NEXT
  PRINT "exit "; n; " at "; i
  n = 0
  FOR i = 1 TO 4
    FOR x = i TO 1 STEP -1: n = n + 1: NEXT
  NEXT
  PRINT "nested "; n
  s = 2.5
  n = 0
  FOR i = 1 TO s: n = n + 1: NEXT
  PRINT "end 2.5 "; n
  n = 0
  FOR i = -3 TO 3
    IF i < 0 THEN
      n = n - 1
    ELSEIF i > 0 THEN
      n = n + 10
    END IF
  NEXT
  PRINT "signs "; n
  ' (beyond 2^53 a Double's steps are 2: the counter still gets there)
  x = 9007199254740992#
  n = 0
  FOR s = x - 4 TO x + 4 STEP 2: n = n + 1: NEXT
  PRINT "big "; n
END SUB

Loops
gn = 0
FOR gi = 1 TO 10: gn = gn + gi: NEXT
PRINT "global "; gn; " after "; gi
gn = 0
FOR gi = 20 TO 0 STEP -5: gn = gn + 1: NEXT
PRINT "gdown "; gn; " after "; gi
gx = 0
FOR gi = 0 TO 2 STEP 0.25: gx = gx + gi: NEXT
PRINT "gquarter "; gx; " after "; gi
gn = 0
DO WHILE gn < 7: gn = gn + 2: LOOP
PRINT "while "; gn
gn = 0
DO: gn = gn + 3: LOOP UNTIL gn >= 10
PRINT "until "; gn
gn = 1
WHILE gn <= 100: gn = gn * 3: WEND
PRINT "wend "; gn
