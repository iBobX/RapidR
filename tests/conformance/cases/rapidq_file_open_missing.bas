' Opening a file that isn't there stops the program, as RapidQ's does — the
' .expected is RC.EXE's output: "Exception EFOpenError … Cannot open file
' nofile.txt." after "before" (RapidQ's asteroids high-score file).
DIM F AS QFILESTREAM
PRINT "before"
F.Open("nofile.txt", 0)
PRINT "after "; F.Size
