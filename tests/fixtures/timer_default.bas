' A QTIMER is enabled by default (manual: Enabled True): with only
' Interval and OnTimer set, it ticks.
$INCLUDE "RAPIDQ.INC"
SUB Tick
  lbl.Caption = "ticking"
END SUB
CREATE Form AS QFORM
  Caption = "Timer default"
  CREATE lbl AS QLABEL
    Caption = "idle"
  END CREATE
  CREATE T AS QTIMER
    Interval = 50
    OnTimer = Tick
  END CREATE
END CREATE
Form.ShowModal
