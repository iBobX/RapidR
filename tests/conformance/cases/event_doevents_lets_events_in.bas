' The window loop's events (here a timer's tick) never run inside a handler
' that is still working: they wait for it to end, back in the loop — unless
' it calls DOEVENTS, which lets them in there and then (RapidQ: DOEVENTS
' "returns control to the operating system to process the events in its
' queue").
$APPTYPE CONSOLE
DECLARE SUB FirstTick
DECLARE SUB SecondTick
DECLARE SUB Work

DIM phase AS INTEGER

CREATE Form AS QFORM
  CREATE First AS QTIMER
    Interval = 20
    OnTimer = FirstTick
  END CREATE
  CREATE Second AS QTIMER
    Interval = 10
    Enabled = 0
    OnTimer = SecondTick
  END CREATE
END CREATE

' 60 ms of work: the second timer is due meanwhile.
SUB Work
  DIM t AS DOUBLE
  t = TIMER
  WHILE TIMER - t < 0.06
  WEND
END SUB

SUB SecondTick
  Second.Enabled = 0
  PRINT "second tick in phase "; phase
  IF phase = 2 THEN Form.Close
END SUB

SUB FirstTick
  First.Enabled = 0
  phase = 1
  Second.Enabled = 1
  Work
  PRINT "first worked"
  DOEVENTS
  PRINT "first after DOEVENTS"
  phase = 2
  Second.Enabled = 1
  Work
  PRINT "first ends"
END SUB

Form.ShowModal
PRINT "done"
