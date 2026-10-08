' RapidR Studio's Output on a console program (tests/studio_flows.mjs
' run-ansi): CLS clears, COLOR colours, LOCATE positions, as a terminal
' shows them: no escape sequence left in the text.
PRINT "one"
PRINT "two"
CLS
PRINT "first line"
COLOR 14, 1
PRINT "yellow on blue"
COLOR
LOCATE 1, 7
PRINT "LINE"
PRINT "row"; CSRLIN
