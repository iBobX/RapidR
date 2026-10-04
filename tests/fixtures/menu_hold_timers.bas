' Timers while a native menu holds the window system (docs/desktop-host-plan.md,
' "Timers during native menu tracking"): `form.__hold_600` makes the kernel's
' headless host hold its next pump 600 ms, as a menu the user keeps open
' would, ticking through the same tracking tick. The timer keeps firing
' meanwhile (as RapidQ's WM_TIMERs do during a menu); one of its handlers,
' run inside the hold, asks for what needs the pump: DOEVENTS returns at
' once, Popup's menu waits for the hold to end, ShowModal and a MESSAGEDLG
' with a choice answer as dismissed (mrCancel, mrNo) without showing.
DECLARE SUB Tick
DECLARE SUB Before
DECLARE SUB After
DIM N AS INTEGER
DIM N0 AS INTEGER
DIM Log AS STRING
CREATE Form AS QFORM
  Caption = "Hold": Width = 320: Height = 160
  CREATE Lbl AS QLABEL
    Width = 300
  END CREATE
  CREATE B1 AS QBUTTON
    Top = 40: Caption = "Before": OnClick = Before
  END CREATE
  CREATE B2 AS QBUTTON
    Top = 70: Caption = "After": OnClick = After
  END CREATE
  CREATE Pop AS QPOPUPMENU
    CREATE P1 AS QMENUITEM
      Caption = "One"
    END CREATE
  END CREATE
END CREATE
CREATE Dlg AS QFORM
  Caption = "Dlg": Width = 200: Height = 100
END CREATE
CREATE T AS QTIMER
  Interval = 40: Enabled = 1: OnTimer = Tick
END CREATE

SUB Tick
  N = N + 1
  ' (the fifth tick after Before: inside the hold)
  IF N0 > 0 AND N = N0 + 5 THEN
    DOEVENTS
    Log = Log + "de;"
    Pop.Popup(Form.Left + 20, Form.Top + 40)
    Log = Log + "pop;"
    Log = Log + "modal" + STR$(Dlg.ShowModal) + ";"
    Log = Log + "ask" + STR$(MESSAGEDLG("Sure?", 3, 1 OR 2, 0)) + ";"
  END IF
END SUB

SUB Before
  N0 = N
END SUB

SUB After
  ' (-1: it ticked during the hold, as often as without one)
  Lbl.Caption = Log + "|" + STR$(N - N0 >= 10)
END SUB

Form.ShowModal
