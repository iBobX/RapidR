' END in an event handler: nothing after it runs, the forms close, the timer stops (tests/web_end_timer.mjs)
$INCLUDE "RAPIDQ.INC"
DIM ticks AS INTEGER
SUB Tick
  ticks = ticks + 1
  lbl.Caption = "ticks" + STR$(ticks)
END SUB
SUB Quit
  PRINT "before end"
  END
  PRINT "after end"
END SUB
CREATE Form AS QFORM
  Caption = "End test"
  CREATE lbl AS QLABEL
    Caption = "start"
  END CREATE
  CREATE Btn AS QBUTTON
    Top = 30
    Caption = "End"
    OnClick = Quit
  END CREATE
  CREATE T AS QTIMER
    Interval = 100
    OnTimer = Tick
  END CREATE
END CREATE
PRINT "main"
Form.ShowModal
