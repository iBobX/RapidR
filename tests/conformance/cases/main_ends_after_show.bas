' The main program's end is the program's: after `Form.Show` (no ShowModal,
' no DOEVENTS loop) the form goes with it and its timer never ticks. RC.EXE
' (RapidQ 2006, in the Windows VM: docs/rapidq-ground-truth.md) ran this
' to its end at once: OnShow ran inside Show, then "after show", no tick.
' Its probes writing timestamps to a file said the same: a form shown with
' a 200 ms timer (closing it at the 3rd tick, or never; hiding it; two
' forms) and a timer alone all wrote "start" and "after show" only, and
' the program ended right away.
DIM F AS QFORM
DIM T AS QTIMER
SUB Shown
  PRINT "onshow"
END SUB
SUB Tick
  PRINT "tick"
END SUB
T.Interval = 20
T.OnTimer = Tick
F.OnShow = Shown
F.Show
PRINT "after show"
