' skip-on: win32, linux, web — macOS's libdispatch plays the driver (dll_async_writes_windows: a Windows timer queue)
' A DLL that writes into the program's memory AFTER its call returned (a
' driver filling winmm's MIDIHDR and buffer, an OVERLAPPED's result): the
' program sees what it wrote after a wait, and a later DLL call that has
' nothing to do with that memory doesn't write the program's old copy over
' it (docs/windows-dll-calls.md §7). Here dispatch_after_f calls uname 0.2 s
' later on a thread of its own, into a TYPE and into a string's characters.
TYPE TUtsName
    sysname AS STRING * 256
    rest AS STRING * 1024
END TYPE

DECLARE FUNCTION dlsym LIB "libSystem.B.dylib" ALIAS "dlsym" (BYVAL handle AS LONG, BYVAL symbol AS STRING) AS LONG
DECLARE FUNCTION dispatch_time LIB "libSystem.B.dylib" ALIAS "dispatch_time" (BYVAL base AS INT64, BYVAL delta AS INT64) AS INT64
DECLARE FUNCTION dispatch_get_global_queue LIB "libSystem.B.dylib" ALIAS "dispatch_get_global_queue" (BYVAL id AS LONG, BYVAL flags AS LONG) AS LONG
DECLARE SUB DispatchToType LIB "libSystem.B.dylib" ALIAS "dispatch_after_f" (BYVAL when AS INT64, BYVAL queue AS LONG, u AS TUtsName, BYVAL work AS LONG)
DECLARE SUB DispatchTo LIB "libSystem.B.dylib" ALIAS "dispatch_after_f" (BYVAL when AS INT64, BYVAL queue AS LONG, BYVAL context AS LONG, BYVAL work AS LONG)
DECLARE FUNCTION labs LIB "libSystem.B.dylib" ALIAS "labs" (BYVAL n AS LONG) AS LONG
DECLARE FUNCTION strlen LIB "libSystem.B.dylib" ALIAS "strlen" (BYVAL s AS LONG) AS LONG

DIM u AS TUtsName
DIM buf AS STRING
DIM queue AS LONG, work AS LONG, n AS LONG
DIM t AS DOUBLE

' (RTLD_DEFAULT is -2)
work = dlsym(-2, "uname")
queue = dispatch_get_global_queue(0, 0)

' 1. Into a TYPE, read after SLEEP.
DispatchToType dispatch_time(0, 200000000), queue, u, work
PRINT "right after the call: ["; u.sysname; "]"
SLEEP 0.6
PRINT "after SLEEP: ["; u.sysname; "]"

' 2. Into a string's characters, then a DLL call that isn't about them.
buf = STRING$(1300, 0)
DispatchTo dispatch_time(0, 200000000), queue, VARPTR(buf), work
t = TIMER
DO
LOOP UNTIL TIMER - t > 0.6
n = labs(-5)
' (strlen reads the characters the driver wrote, not a copy from before)
PRINT "strlen:"; strlen(VARPTR(buf))
PRINT "the variable: ["; LEFT$(buf, INSTR(buf, CHR$(0)) - 1); "]"
