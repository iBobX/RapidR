' A PRINT right before the ELSE of a single-line IF ends without a new line,
' as RapidQ's compiler does it — the .expected is RC.EXE's output
' (docs/rapidq-ground-truth.md); the ELSE's own PRINT, a block IF's and a
' single-line IF's without ELSE end their lines.
IF 1 THEN PRINT "a" ELSE PRINT "b"
PRINT "c"
IF 0 THEN PRINT "d" ELSE PRINT "e"
PRINT "f"
IF 1 THEN PRINT "g"
PRINT "h"
IF 1 THEN PRINT "i"; ELSE PRINT "j"
PRINT "k"
IF 1 THEN PRINT "l": PRINT "m" ELSE PRINT "n"
PRINT "o"
IF 0 THEN PRINT "p" ELSE PRINT "q": PRINT "r"
PRINT "s"
IF 1 THEN
  PRINT "t"
ELSE
  PRINT "u"
END IF
PRINT "v"
IF 1 THEN PRINT ELSE PRINT "w"
PRINT "x"
PRINT ,"lead"; 1, 2
PRINT "end"
