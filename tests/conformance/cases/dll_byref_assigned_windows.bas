' skip-on: darwin, linux — kernel32 (dll_byref_assigned_macos: the C library)
' A BYREF variable the DLL writes, assigned the function's result in the
' same statement: the assignment comes last, so the variable holds the
' result (RC.EXE, 2026-11-05: GetComputerNameA writes nSize = 15 and returns
' 1; `n = GetComputerName(s, n)` leaves n = 1). docs/windows-dll-calls.md §7.
DECLARE FUNCTION GetComputerName LIB "kernel32" ALIAS "GetComputerNameA" (lpBuffer AS STRING, BYREF nSize AS LONG) AS LONG
DIM n AS LONG
DIM r AS LONG
DIM s AS STRING
s = SPACE$(64)
n = 64
r = GetComputerName(s, n)
PRINT "r ="; r
n = 64
n = GetComputerName(s, n)
PRINT "n = GetComputerName(s, n) gives"; n
