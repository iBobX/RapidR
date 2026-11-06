' skip-on: win32, linux — macOS's libdispatch plays the driver (dll_async_writes_handler_windows: a Windows timer queue)
' What a driver writes into the program's memory after its call returned is
' there when an event's handler starts, before the handler calls anything:
' in a TYPE's fields and in a string variable handed over by VARPTR (as
' winmm's MIDIHDR and its buffer when MM_MIM_LONGDATA comes:
' docs/windows-dll-calls.md §7). dispatch_after_f calls uname 0.2 s later on
' a thread of its own; the timer's handler reads both 0.5 s later.
$APPTYPE CONSOLE
TYPE TUtsName
    sysname AS STRING * 256
    rest AS STRING * 1024
END TYPE

DECLARE FUNCTION dlsym LIB "libSystem.B.dylib" ALIAS "dlsym" (BYVAL handle AS LONG, BYVAL symbol AS STRING) AS LONG
DECLARE FUNCTION dispatch_time LIB "libSystem.B.dylib" ALIAS "dispatch_time" (BYVAL base AS INT64, BYVAL delta AS INT64) AS INT64
DECLARE FUNCTION dispatch_get_global_queue LIB "libSystem.B.dylib" ALIAS "dispatch_get_global_queue" (BYVAL id AS LONG, BYVAL flags AS LONG) AS LONG
DECLARE SUB DispatchToType LIB "libSystem.B.dylib" ALIAS "dispatch_after_f" (BYVAL when AS INT64, BYVAL queue AS LONG, u AS TUtsName, BYVAL work AS LONG)
DECLARE SUB DispatchTo LIB "libSystem.B.dylib" ALIAS "dispatch_after_f" (BYVAL when AS INT64, BYVAL queue AS LONG, BYVAL context AS LONG, BYVAL work AS LONG)
DECLARE SUB Tick

DIM u AS TUtsName
DIM buf AS STRING
DIM seen AS STRING
DIM queue AS LONG, work AS LONG

CREATE Form AS QFORM
    Width = 200
    Height = 120
    CREATE Clock AS QTIMER
        Interval = 500
        OnTimer = Tick
    END CREATE
END CREATE

SUB Tick
    ' (nothing called before these reads)
    seen = "TYPE: [" + RTRIM$(u.sysname) + "]  variable: [" + LEFT$(buf, INSTR(buf, CHR$(0)) - 1) + "]"
    Clock.Enabled = 0
    Form.Close
END SUB

work = dlsym(-2, "uname")
queue = dispatch_get_global_queue(0, 0)
buf = STRING$(1300, 0)
DispatchToType dispatch_time(0, 200000000), queue, u, work
DispatchTo dispatch_time(0, 200000000), queue, VARPTR(buf), work
Form.ShowModal
PRINT seen
