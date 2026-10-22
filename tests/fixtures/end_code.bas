' RapidR's END n: on the desktop the program ends with exit code 3 (the
' conformance case end_exit_code_literal checks it); in a browser a program
' has no exit code, so it just ends — "after" is never printed, and the 3 is
' ignored (tests/web_end_code.mjs).
PRINT "before"
END 3
PRINT "after"
