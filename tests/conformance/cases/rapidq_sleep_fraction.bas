' SLEEP .3 sleeps three tenths of a second (a number after a space is a
' number, not a member: RapidQ's MOVETEXT example) — the .expected is RC.EXE's
' output; RapidQ's single-line IF … ELSE PRINT keeps no line end.
DIM t AS DOUBLE
t = TIMER
SLEEP .3
IF TIMER - t >= .25 THEN PRINT "slept" ELSE PRINT "no"
PRINT .5 + 1; " "; -.25
