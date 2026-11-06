' skip-on: darwin, linux, web — a Windows timer queue plays the driver (dll_async_writes_macos: libdispatch)
' A DLL that writes into the program's memory AFTER its call returned (a
' driver filling winmm's MIDIHDR and buffer, an OVERLAPPED's result): the
' program sees what it wrote after a wait, and a later DLL call that has
' nothing to do with that memory doesn't write the program's old copy over
' it (docs/windows-dll-calls.md §7). Here CreateTimerQueueTimer calls
' GetStartupInfoA 0.2 s later on a thread of Windows' pool, into a TYPE and
' into a string's characters (its first LONG: the structure's size, 104 on
' 64-bit Windows).
TYPE TStartupInfo
    cb AS LONG
    rest AS STRING * 100
END TYPE

DECLARE FUNCTION GetModuleHandle LIB "kernel32" ALIAS "GetModuleHandleA" (BYVAL name AS STRING) AS LONG
DECLARE FUNCTION GetProcAddress LIB "kernel32" ALIAS "GetProcAddress" (BYVAL module AS LONG, BYVAL name AS STRING) AS LONG
DECLARE FUNCTION TimerToType LIB "kernel32" ALIAS "CreateTimerQueueTimer" (BYREF timer AS LONG, BYVAL queue AS LONG, BYVAL callback AS LONG, si AS TStartupInfo, BYVAL due AS LONG, BYVAL period AS LONG, BYVAL flags AS LONG) AS LONG
DECLARE FUNCTION TimerTo LIB "kernel32" ALIAS "CreateTimerQueueTimer" (BYREF timer AS LONG, BYVAL queue AS LONG, BYVAL callback AS LONG, BYVAL parameter AS LONG, BYVAL due AS LONG, BYVAL period AS LONG, BYVAL flags AS LONG) AS LONG
DECLARE FUNCTION GetTickCount LIB "kernel32" ALIAS "GetTickCount" () AS LONG
DECLARE FUNCTION lstrlen LIB "kernel32" ALIAS "lstrlenA" (BYVAL s AS LONG) AS LONG

CONST WT_EXECUTEONLYONCE = 8
DIM si AS TStartupInfo
DIM buf AS STRING
DIM timer1 AS LONG, timer2 AS LONG, work AS LONG, n AS LONG
DIM t AS DOUBLE

work = GetProcAddress(GetModuleHandle("kernel32"), "GetStartupInfoA")

' 1. Into a TYPE, read after SLEEP.
PRINT "queued:"; TimerToType(timer1, 0, work, si, 200, 0, WT_EXECUTEONLYONCE)
PRINT "right after the call:"; si.cb
SLEEP 0.6
PRINT "after SLEEP:"; si.cb

' 2. Into a string's characters, then a DLL call that isn't about them.
buf = STRING$(120, 0)
PRINT "queued:"; TimerTo(timer2, 0, work, VARPTR(buf), 200, 0, WT_EXECUTEONLYONCE)
t = TIMER
DO
LOOP UNTIL TIMER - t > 0.6
n = GetTickCount
' (the characters the timer's thread wrote — 104 is "h", then a NUL —,
' not a copy from before)
PRINT "lstrlen:"; lstrlen(VARPTR(buf))
PRINT "the variable:"; ASC(buf)
