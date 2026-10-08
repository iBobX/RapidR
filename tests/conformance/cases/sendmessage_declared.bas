' skip-on: win32 — there the call is made
' A program that declares its own SendMessage keeps it: RapidQ's built-in of
' that name only stands in where nothing else does (the DECLARE's alias
' is what runs, here the error naming it off Windows)
DECLARE FUNCTION SendMessage LIB "user32" ALIAS "SendMessageW" (BYVAL h AS LONG, BYVAL m AS LONG, BYVAL w AS LONG, BYVAL l AS LONG) AS LONG
PRINT "before"
r = SendMessage(0, 0, 0, 0)
PRINT "not reached"
