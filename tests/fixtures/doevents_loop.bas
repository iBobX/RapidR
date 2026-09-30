' A main program waiting in a DOEVENTS loop for a timer (RapidQ's way to
' keep a form alive while busy): the timer's events run meanwhile — on the
' web too, where the loop used to freeze the page.
DECLARE SUB Tick
DIM Ticks AS INTEGER
CREATE Form AS QFORM
  Caption = "doevents"
  CREATE Lbl AS QLABEL
    Width = 200
    Caption = "waiting"
  END CREATE
  CREATE Tmr AS QTIMER
    Interval = 50
    OnTimer = Tick
  END CREATE
END CREATE
Form.Show
SUB Tick
  Ticks = Ticks + 1
END SUB
t = TIMER
WHILE Ticks < 3 AND TIMER - t < 5
  DOEVENTS
WEND
Tmr.Enabled = False
First = Ticks >= 3
' Enabled again, it ticks again.
Tmr.Enabled = True
t = TIMER
WHILE Ticks < 6 AND TIMER - t < 5
  DOEVENTS
WEND
Tmr.Enabled = False
Lbl.Caption = "ticked " + STR$(First) + " " + STR$(Ticks >= 6)
Form.ShowModal
