' skip-on: win32, linux, web — the C library on macOS (dll_byref_assigned_windows: kernel32, as RC.EXE ran it)
' A BYREF variable the DLL writes, assigned the function's result in the
' same statement: the assignment comes last, so the variable holds the
' result, as RapidQ's (docs/windows-dll-calls.md §7). modf(7.5, ip)
' writes 7 into ip and returns 0.5.
DECLARE FUNCTION modf LIB "libSystem.B.dylib" ALIAS "modf" (BYVAL x AS DOUBLE, BYREF ip AS DOUBLE) AS DOUBLE
DIM f AS DOUBLE
DIM ip AS DOUBLE
f = modf(3.25, ip)
PRINT "f ="; f; " ip ="; ip
ip = modf(7.5, ip)
PRINT "ip = modf(7.5, ip) gives"; ip
