' INP / OUT (hardware ports) are a run-time error on every system, as
' RapidQ's own are on every Windows since 2000 — the program runs up to
' the statement (docs/windows-dll-calls.md §2)
PRINT "start"
x = INP(&H3C9)
PRINT "not reached"
