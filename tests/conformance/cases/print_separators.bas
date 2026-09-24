' xfail: vm, codegen — any ';' in PRINT suppresses the newline; only a trailing ';' should
' A trailing ';' keeps the cursor on the line; a ';' between items does not.
PRINT "a"; "b"
PRINT "c";
PRINT "d"
PRINT "e"
