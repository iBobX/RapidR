' The main program ends after Form.Show (no ShowModal): the program ends
' there, as RapidQ's does (RC.EXE: docs/rapidq-ground-truth.md) — OnShow
' runs inside Show, the form goes, the timer never ticks
' (tests/web_main_end.mjs; the desktop's: the main_ends_after_show case).
DIM ticks AS INTEGER
SUB Shown
  PRINT "onshow"
END SUB
SUB Tick
  ticks = ticks + 1
  PRINT "tick"
END SUB
CREATE Form AS QFORM
  Caption = "Main end"
  CREATE lbl AS QLABEL
    Caption = "label"
  END CREATE
  CREATE T AS QTIMER
    Interval = 50
    OnTimer = Tick
  END CREATE
  OnShow = Shown
END CREATE
Form.Show
PRINT "after show"
