' Waits and events: a timer ticking while the main form is modal, and a
' button handler that opens a second modal form which a timer closes —
' the handler then continues. Checked natively and interpreted by
' tests/native_gui_events.mjs (the interpreter serves ShowModal's wait itself).
DECLARE SUB Tick
DECLARE SUB OpenSecond
DECLARE SUB CloseSecond
DIM Log AS STRING
CREATE Form AS QFORM
  Caption = "Main": Width = 300: Height = 150
  CREATE Btn AS QBUTTON
    Caption = "Open": OnClick = OpenSecond
  END CREATE
  CREATE Lbl AS QLABEL
    Top = 40: Width = 280
  END CREATE
  CREATE Lbl2 AS QLABEL
    Top = 70: Width = 280
  END CREATE
END CREATE
CREATE Form2 AS QFORM
  Caption = "Second": Width = 200: Height = 100
END CREATE
CREATE T AS QTIMER
  Interval = 50: Enabled = 1: OnTimer = Tick
END CREATE
CREATE T2 AS QTIMER
  Interval = 100: Enabled = 0: OnTimer = CloseSecond
END CREATE

SUB Tick
  Lbl2.Caption = "ticking"
END SUB

SUB OpenSecond
  Log = Log + "open;"
  T2.Enabled = 1
  Form2.ShowModal
  Log = Log + "closed;"
  Lbl.Caption = Log
END SUB

SUB CloseSecond
  T2.Enabled = 0
  Log = Log + "timer-close;"
  Form2.Close
END SUB

Form.ShowModal
