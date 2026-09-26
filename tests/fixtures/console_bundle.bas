' A console program for tests/web_bundle_console.mjs: PRINT, COLOR, LOCATE,
' a partial line, and a form shown afterwards (the console then docks).
$INCLUDE "RAPIDQ.INC"
PRINT "Hello from RapidR"
COLOR 14, 1
PRINT "yellow on blue"
COLOR
PRINT "sum ="; 2 + 3
PRINT "partial ";
PRINT "line"
LOCATE 1, 7
PRINT "RAPIDR"
