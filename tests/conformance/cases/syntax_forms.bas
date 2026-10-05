DIM x AS INTEGER
x = 5
IF x > 3 THEN: PRINT "a": END IF
IF x > 9 THEN: PRINT "no": ELSE: PRINT "b": END IF
IF x = 5 THEN :
  PRINT "c"
  PRINT "d"
END IF
IF x < 9 THEN PRINT "e" : ' comment here
IF x => 5 AND x =< 5 THEN PRINT "f"
CONST H1 = &H1&
CONST H2 = &HFFFF&
PRINT H1; " "; H2; " "; &HFF & "x"
FUNCTION Feb (o AS INTEGER) AS INTEGER
    IF o = 0 THEN :Feb = 29
        ELSE :Feb = 28
    END IF
END FUNCTION
PRINT Feb(0); Feb(1)
DIM n AS INTEGER
DIM C& AS LONG
n = 4
SELECT CASE n
' (RC.EXE reads `CASE 4, 7<TAB>C& = -2` as the list 4, 7: the assignment is
' an operand side by side with the 7 — tests/conformance/cases/
' juxtaposed_operands.bas — so the case has no body; RapidQ's own
' dayfunction.bas example has this line)
CASE 4, 7	C& = -2
CASE ELSE C& = 5
END SELECT
PRINT C&
270 FOR I = 1 TO 3
280 PRINT I;
310 NEXT I
PRINT
10 WHILE n < 6: n = n + 1
20 WEND
PRINT n
IF n = 6 THEN PRINT "one-line" END IF
IF n = 1 THEN PRINT "no" ELSE PRINT "else" END IF
900 READ da, db
910 PRINT da + db
920 DATA 3, 4
