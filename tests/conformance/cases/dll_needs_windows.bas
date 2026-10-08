' skip-on: win32 — there the call succeeds (docs/windows-dll-calls.md §1)
' A program that calls Windows itself (DECLARE … LIB "kernel32") compiles
' everywhere and runs on Windows; on macOS, Linux and the web it stops at
' the call with an error naming the function and that it needs Windows
DECLARE FUNCTION GetTickCount LIB "kernel32" ALIAS "GetTickCount" () AS LONG
PRINT "before"
t = GetTickCount
PRINT "not reached"
