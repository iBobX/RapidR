' Inside a FUNCTION, Result is the FUNCTION's own value (RapidQ's RESULT),
' whatever else is named Result: a parameter (its argument can't be read
' there), a DIM of the FUNCTION's, a global. A SUB's parameter named Result
' is an ordinary parameter. RC.EXE's output (Windows 11 VM, 2026-10-22).
$APPTYPE CONSOLE
DIM Result AS INTEGER
FUNCTION Joined (Id AS STRING, Result AS STRING) AS STRING
    Joined = Id + "/" + Result
END FUNCTION
FUNCTION SetsIt (Id AS STRING, Result AS STRING) AS STRING
    Result = Id + "!"
END FUNCTION
FUNCTION Twice (Result AS INTEGER) AS INTEGER
    Twice = Result * 2
END FUNCTION
FUNCTION Seen (Id AS STRING, Result AS STRING) AS STRING
    Seen = "x"
    PRINT "seen "; Result
END FUNCTION
FUNCTION WithLocal (Id AS STRING) AS STRING
    DIM Result AS STRING
    Result = "local"
    WithLocal = "z"
    PRINT "local "; Result
END FUNCTION
FUNCTION WithGlobal (A AS INTEGER) AS INTEGER
    WithGlobal = A + 1
    PRINT "global "; Result
END FUNCTION
SUB Shown (A AS STRING, Result AS STRING)
    PRINT "sub "; A; Result
END SUB
Result = 40
PRINT "joined "; Joined("a", "b")
PRINT "sets "; SetsIt("a", "b")
PRINT "twice "; Twice(21)
DIM s AS STRING
s = Seen("a", "b")
PRINT "seen gave "; s
s = WithLocal("a")
PRINT "local gave "; s
PRINT "global gave "; WithGlobal(1); " "; Result
Shown "a", "b"
