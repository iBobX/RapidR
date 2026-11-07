' A classic program's QTIMER counts its Interval in the system timer's steps
' of 15.625 ms, rounded up (RC.EXE, Windows 11, 2026-11-07: Interval 20 ticked
' every 33 ms on average, 30 times a second, not 50). The same program as a
' RapidR program ticks 50 times (timer_default_rate.rr): a classic program
' keeps the classic beat, a RapidR program's RTimer ticks every Interval
' exactly. Both run for a second from the first tick; the count is said in a
' range (a busy machine).
$APPTYPE CONSOLE
DECLARE SUB Tick
DIM Ticks AS INTEGER, Began AS DOUBLE

CREATE Form AS QFORM
  Width = 200
  Height = 100
END CREATE

CREATE Clock AS QTIMER
  Interval = 20
  OnTimer = Tick
END CREATE

SUB Tick
  IF Began = 0 THEN Began = TIMER
  Ticks = Ticks + 1
  IF TIMER - Began >= 1 THEN
    Clock.Enabled = 0
    Form.Close
  END IF
END SUB

Form.ShowModal
IF Ticks >= 24 AND Ticks <= 38 THEN
  PRINT "Interval 20 in Windows' steps of 15.6 ms: about 32 a second"
ELSE
  PRINT "ticks: "; Ticks
END IF
