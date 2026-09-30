' SLEEP counts seconds (RapidQ's manual: SLEEP 1.5 pauses 1.5 seconds);
' DOEVENTS lets the rest of the program's world run (on the web: the
' browser) and comes back.
t = TIMER
SLEEP 0.3
d = TIMER - t
PRINT (d >= 0.25) AND (d < 3)
FOR i = 1 TO 3
  DOEVENTS
NEXT
PRINT "done"
