' FILEEXISTS with * and ? (RC.EXE, 2026-10-22, docs/rapidq-ground-truth.md):
' RapidQ asks Windows for the first name that matches, and that name must
' be a file — so a pattern a file matches is 1, but `*` and `*.*` meet the
' folder's own "." first and are 0, and a pattern only a folder matches is 0.
' Any case; a ? at the end of the name or before its dot may match nothing.
' On the web the program's files are matched (the page has no folders but
' the program's own). The .expected is RC.EXE's output.
D$ = "tests/conformance/cases/resource_files/"
IF DIREXISTS(D$) = 0 THEN D$ = ""
PRINT "txt "; FILEEXISTS(D$ + "*.txt")
PRINT "none "; FILEEXISTS(D$ + "*.nope")
PRINT "q "; FILEEXISTS(D$ + "h?llo.txt")
PRINT "case "; FILEEXISTS(D$ + "HELLO.T*")
PRINT "short q "; FILEEXISTS(D$ + "hello??.txt")
PRINT "two "; FILEEXISTS(D$ + "two.*")
PRINT "star "; FILEEXISTS(D$ + "*")
PRINT "dot star "; FILEEXISTS(D$ + "*.*")
PRINT "folder only "; FILEEXISTS("tests/conformance/case?")
PRINT "no folder "; FILEEXISTS("tests/no_such_folder/*.txt")
IF FILEEXISTS(D$ + "*.txt") THEN PRINT "a text file is there"
