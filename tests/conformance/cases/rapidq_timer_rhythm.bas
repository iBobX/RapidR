' A QTIMER keeps its beat whatever its handler takes (RC.EXE, Windows 11,
' 2026-11-07, a probe that spun on the performance counter: Interval 20 with
' a 24 ms handler gave gaps between one handler's start and the next of 31
' ms, not 55; a 50 ms handler gave 50, each tick running the moment the one
' before ended; Interval 100 with a 24 ms handler gave 109 to 116). The
' system's timer ticks every Interval (in steps of 15.6 ms); a tick that
' fell due while the handler ran waits for it and runs the moment it ends;
' the next one is a period from where the last was *due*, not from where its
' handler ended. Here Interval 100 (109.4 ms) with a 50 ms handler: the
' gaps are about 109, never 109 + 50.
$APPTYPE CONSOLE
DECLARE SUB Tick
DIM Last AS DOUBLE, Ticks AS INTEGER, Gaps AS INTEGER, Quick AS INTEGER

CREATE Form AS QFORM
  Width = 200
  Height = 100
END CREATE

CREATE Clock AS QTIMER
  Interval = 100
  OnTimer = Tick
END CREATE

SUB Tick
  DIM Began AS DOUBLE, Gap AS DOUBLE
  Began = TIMER
  IF Ticks > 0 THEN
    Gap = (Began - Last) * 1000
    Gaps = Gaps + 1
    IF Gap <= 125 THEN Quick = Quick + 1
  END IF
  Last = Began
  Ticks = Ticks + 1
  ' 50 ms of work
  WHILE TIMER - Began < 0.05
  WEND
  IF Ticks = 13 THEN
    Clock.Enabled = 0
    Form.Close
  END IF
END SUB

Form.ShowModal
PRINT "ticks="; Ticks
IF Quick * 4 >= Gaps * 3 THEN
  PRINT "the beat is kept"
ELSE
  PRINT "the beat drifts: gaps of an Interval and a handler"
END IF
