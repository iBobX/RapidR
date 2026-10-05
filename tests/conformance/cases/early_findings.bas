' The first review's findings (ROADMAP, 2026-09-24), now fixed on both backends; numbers print as RapidQ's PRINT does (RC.EXE: 9 decimals, whole numbers as 32-bit integers) and STR$ (9 digits)
DECLARE SUB Hello (n AS INTEGER)
? "question print"
i = 5: INC i: INC i, 3: DEC i
PRINT i
PRINT REPLACESUBSTR$("aXbX", "X", "-")
PRINT "a", "b", "c"
GOSUB Sub1
Hello 2
PRINT 0.1 + 0.2, 1 / 3, 2 ^ 70, 1E-7
x = 5
PRINT "x:"; x, STR$(x); "|"; STR$(-x)
END
Sub1:
  PRINT "in gosub"
  RETURN
SUB Hello (n AS INTEGER)
  PRINT "hello"; n
END SUB
