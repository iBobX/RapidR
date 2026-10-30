' A timer whose handler takes longer than its Interval never runs inside
' itself: its next tick comes once the handler has ended (RapidQ's QTIMER is
' Windows' SetTimer — at most one WM_TIMER waits, and only the message loop
' delivers it). The handler draws as an emulator's frame does: the canvas's
' Repaint inside it runs OnPaint at once, before the next statement.
$APPTYPE CONSOLE
DECLARE SUB Tick
DECLARE SUB Paint

DIM busy AS INTEGER, overlaps AS INTEGER, ticks AS INTEGER, inside AS INTEGER

CREATE Form AS QFORM
  Width = 200
  Height = 150
  CREATE Screen AS QCANVAS
    Width = 160
    Height = 100
    OnPaint = Paint
  END CREATE
  CREATE Clock AS QTIMER
    Interval = 10
    OnTimer = Tick
  END CREATE
END CREATE

SUB Paint
  IF busy THEN inside = inside + 1
  Screen.FillRect 0, 0, 160, 100, &H202020
  Screen.TextOut 4, 4, "tick " + STR$(ticks), &HFFFF, &H202020
END SUB

SUB Tick
  IF busy THEN overlaps = overlaps + 1
  busy = 1
  ticks = ticks + 1
  ' 30 ms of work: three Intervals
  DIM t AS DOUBLE
  t = TIMER
  WHILE TIMER - t < 0.03
    Screen.Pset 1, 1, &HFF
  WEND
  Screen.Repaint
  busy = 0
  IF ticks = 5 THEN
    Clock.Enabled = 0
    Form.Close
  END IF
END SUB

Form.ShowModal
PRINT "ticks="; ticks
PRINT "overlaps="; overlaps
PRINT "paints inside a tick="; inside
