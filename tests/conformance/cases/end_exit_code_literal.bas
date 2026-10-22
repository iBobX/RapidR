' END n with a literal: after another statement on the line (`PRINT "x": END
' 3`), and as the single statement of a one-line IF.
DIM n AS INTEGER
n = 5
IF n = 4 THEN END 9
PRINT "x": END 3
PRINT "not reached"
