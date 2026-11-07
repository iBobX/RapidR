' w_timer: when does a QTIMER tick again? Its handler works Busy ms (spinning
' on QueryPerformanceCounter) and the program records the start and end of
' each handler. For each (Interval, Busy) it writes the mean gap between
' starts, the gap between one handler's end and the next one's start, and the
' first gaps, in ms. A tick every Interval from the *start* of the last one
' (the system's timer, one message waiting) gives gaps near Interval whatever
' Busy is (until Busy is longer); a tick Interval after the *end* gives
' gaps of Busy + Interval.
$INCLUDE "RAPIDQ.INC"
TYPE TLI
  Lo AS LONG
  Hi AS LONG
END TYPE
DECLARE FUNCTION QPC LIB "kernel32" ALIAS "QueryPerformanceCounter" (T AS TLI) AS LONG
DECLARE FUNCTION QPF LIB "kernel32" ALIAS "QueryPerformanceFrequency" (T AS TLI) AS LONG
DECLARE SUB Tick

DIM Rep AS QFILESTREAM
DIM Freq AS DOUBLE
DIM Busy AS INTEGER
DIM N AS INTEGER
DIM Want AS INTEGER
DIM StartAt(0 TO 400) AS DOUBLE
DIM EndAt(0 TO 400) AS DOUBLE

CREATE Form AS QFORM
  Caption = "w_timer"
  Width = 200
  Height = 100
END CREATE

CREATE Clock AS QTIMER
  Enabled = 0
  Interval = 20
  OnTimer = Tick
END CREATE

FUNCTION Stamp AS DOUBLE
  DIM T AS TLI
  DIM R AS LONG
  DIM D AS DOUBLE
  R = QPC(T)
  D = T.Lo
  IF D < 0 THEN D = D + 4294967296
  Stamp = D * 1000 / Freq
END FUNCTION

SUB Tick
  IF N > Want THEN EXIT SUB
  StartAt(N) = Stamp
  DIM Limit AS DOUBLE
  Limit = StartAt(N) + Busy
  WHILE Stamp < Limit
  WEND
  EndAt(N) = Stamp
  N = N + 1
END SUB

SUB RunOne (Interval AS INTEGER, BusyMs AS INTEGER, Ticks AS INTEGER)
  DIM I AS INTEGER
  DIM Gap AS DOUBLE
  DIM Sum AS DOUBLE
  DIM Idle AS DOUBLE
  DIM Gaps AS STRING
  Busy = BusyMs
  N = 0
  Want = Ticks
  Clock.Interval = Interval
  Clock.Enabled = 1
  WHILE N < Ticks
    DOEVENTS
  WEND
  Clock.Enabled = 0
  Sum = 0
  Idle = 0
  Gaps = ""
  FOR I = 1 TO Ticks - 1
    Gap = StartAt(I) - StartAt(I - 1)
    Sum = Sum + Gap
    Idle = Idle + (StartAt(I) - EndAt(I - 1))
    IF I <= 12 THEN Gaps = Gaps + " " + STR$(INT(Gap * 10 + 0.5) / 10)
  NEXT
  Rep.WriteLine "Interval=" + STR$(Interval) + " Busy=" + STR$(BusyMs) + ": mean gap " + STR$(INT(Sum / (Ticks - 1) * 100 + 0.5) / 100) + " ms, mean end-to-next-start " + STR$(INT(Idle / (Ticks - 1) * 100 + 0.5) / 100) + " ms, gaps:" + Gaps
END SUB

DIM T0 AS TLI
DIM R0 AS LONG
R0 = QPF(T0)
Freq = T0.Lo
Rep.Open("w_timer.txt", fmCreate)
Rep.WriteLine "QueryPerformanceFrequency=" + STR$(Freq)
Form.Show
RunOne 20, 0, 60
RunOne 20, 24, 60
RunOne 20, 50, 40
RunOne 100, 24, 30
RunOne 100, 130, 20
RunOne 50, 24, 40
Rep.Close
