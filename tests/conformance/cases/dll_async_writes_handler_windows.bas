' skip-on: darwin, linux, web — a Windows timer queue plays the driver (dll_async_writes_handler_macos: libdispatch)
' What a driver writes into the program's memory after its call returned is
' there when an event's handler starts, before the handler calls anything:
' in a TYPE's fields and in a string variable handed over by VARPTR (as
' winmm's MIDIHDR and its buffer when MM_MIM_LONGDATA comes:
' docs/windows-dll-calls.md §7). CreateTimerQueueTimer calls
' GetStartupInfoA 0.2 s later on a thread of Windows' pool (its first LONG:
' the structure's size, 104 on 64-bit Windows); the timer's handler reads
' both 0.5 s later.
$APPTYPE CONSOLE
TYPE TStartupInfo
    cb AS LONG
    rest AS STRING * 100
END TYPE

DECLARE FUNCTION GetModuleHandle LIB "kernel32" ALIAS "GetModuleHandleA" (BYVAL name AS STRING) AS LONG
DECLARE FUNCTION GetProcAddress LIB "kernel32" ALIAS "GetProcAddress" (BYVAL module AS LONG, BYVAL name AS STRING) AS LONG
DECLARE FUNCTION TimerToType LIB "kernel32" ALIAS "CreateTimerQueueTimer" (BYREF timer AS LONG, BYVAL queue AS LONG, BYVAL callback AS LONG, si AS TStartupInfo, BYVAL due AS LONG, BYVAL period AS LONG, BYVAL flags AS LONG) AS LONG
DECLARE FUNCTION TimerTo LIB "kernel32" ALIAS "CreateTimerQueueTimer" (BYREF timer AS LONG, BYVAL queue AS LONG, BYVAL callback AS LONG, BYVAL parameter AS LONG, BYVAL due AS LONG, BYVAL period AS LONG, BYVAL flags AS LONG) AS LONG
DECLARE SUB Tick

CONST WT_EXECUTEONLYONCE = 8
DIM si AS TStartupInfo
DIM buf AS STRING
DIM seen AS STRING
DIM timer1 AS LONG, timer2 AS LONG, work AS LONG

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
    seen = "TYPE:" + STR$(si.cb) + "  variable:" + STR$(ASC(buf))
    Clock.Enabled = 0
    Form.Close
END SUB

work = GetProcAddress(GetModuleHandle("kernel32"), "GetStartupInfoA")
buf = STRING$(120, 0)
TimerToType(timer1, 0, work, si, 200, 0, WT_EXECUTEONLYONCE)
TimerTo(timer2, 0, work, VARPTR(buf), 200, 0, WT_EXECUTEONLYONCE)
Form.ShowModal
PRINT seen
