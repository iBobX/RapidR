' The web's command line: the page's query string, one argument per
' `&`-separated part, decoded (tests/web_bundle_console.mjs opens
' index.html?a&b%20c&x%3D1). COMMAND$(0) is the page.
$APPTYPE CONSOLE
DIM i AS INTEGER
PRINT "count="; CommandCount
FOR i = 1 TO CommandCount + 1
  PRINT "arg"; i; "=["; COMMAND$(i); "]"
NEXT
PRINT "bare=["; COMMAND$; "]"
IF INSTR(COMMAND$(0), "index.html") > 0 THEN PRINT "self ok"
