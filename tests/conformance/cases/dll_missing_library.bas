' A DLL that isn't there: the error names it (on Windows "can't find the
' DLL", elsewhere it can't be loaded or the web can't load any) and the
' program stops at the call, not before it starts as RapidQ's do
DECLARE FUNCTION Nothing LIB "nosuchlib" ALIAS "Nothing" (BYVAL n AS LONG) AS LONG
PRINT "before"
x = Nothing(1)
PRINT "not reached"
