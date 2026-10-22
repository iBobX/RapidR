' FILEEXISTS gives 1 for a file and 0 otherwise (RC.EXE, 2026-10-22,
' docs/rapidq-ground-truth.md) — so NOT FILEEXISTS(f) is -2 for a file, which
' is true: `IF NOT FILEEXISTS(f) THEN` runs its THEN for a file too, in
' RapidQ as here. A folder and "" are 0. The .expected is RC.EXE's output.
F$ = "tests/conformance/cases/rapidq_fileexists.bas"
M$ = "tests/conformance/cases/no_such_file.txt"
PRINT "exists "; FILEEXISTS(F$)
PRINT "missing "; FILEEXISTS(M$)
PRINT "not exists "; NOT FILEEXISTS(F$)
PRINT "not missing "; NOT FILEEXISTS(M$)
IF NOT FILEEXISTS(F$) THEN
  PRINT "IF NOT (exists): then"
ELSE
  PRINT "IF NOT (exists): else"
END IF
IF NOT FILEEXISTS(M$) THEN
  PRINT "IF NOT (missing): then"
ELSE
  PRINT "IF NOT (missing): else"
END IF
IF FILEEXISTS(F$) = 1 THEN PRINT "= 1"
IF FILEEXISTS(F$) = -1 THEN PRINT "= -1"
IF FILEEXISTS(F$) = 0 THEN PRINT "= 0 (exists)"
IF FILEEXISTS(M$) = 0 THEN PRINT "= 0 (missing)"
PRINT "empty "; FILEEXISTS("")
PRINT "folder "; FILEEXISTS("tests")
DIM r AS INTEGER
r = FILEEXISTS(F$)
PRINT "stored "; r; " str "; STR$(FILEEXISTS(F$))
