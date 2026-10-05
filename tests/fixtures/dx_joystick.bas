' QDXJOYSTICK (docs/directx-plan.md §2.4, stage D6): RapidQ's way — Update,
' then IsLeft / IsRight / IsUp / IsDown and Button(n) as Update read them —
' then RapidR's additions (Name, Connected, X, Y, Buttons, POV) and its
' events (OnButtonDown / OnButtonUp with the button's number, OnMove), the
' handlers bound at run time; a joystick unplugged. The gamepad is the
' tests' script (the case's `joystick`: RAPIDR_TEST_JOYSTICK on the
' desktop, the page's RAPIDR_TEST_JOYSTICK in the browser), a step a read.
' Checked natively and interpreted by tests/native_gui_events.mjs, in the
' browser by tests/web_gui_parity.mjs.
$INCLUDE "RAPIDQ.INC"
DECLARE SUB Poll
DECLARE SUB Watch
DECLARE SUB Report
DECLARE SUB Down(Button AS INTEGER)
DECLARE SUB Up(Button AS INTEGER)
DECLARE SUB Moved
DIM log AS STRING
DIM events AS STRING
DIM t AS DOUBLE
DIM J AS QDXJOYSTICK
DIM Pad AS QDXJOYSTICK
CREATE Form AS QFORM
  Caption = "dx joystick"
  Width = 320
  CREATE b1 AS QBUTTON
    Left = 10
    Top = 10
    Caption = "Poll"
    OnClick = Poll
  END CREATE
  CREATE b2 AS QBUTTON
    Left = 90
    Top = 10
    Caption = "Watch"
    OnClick = Watch
  END CREATE
  CREATE b3 AS QBUTTON
    Left = 170
    Top = 10
    Caption = "Report"
    OnClick = Report
  END CREATE
  CREATE lbl AS QLABEL
    Left = 10
    Top = 50
    Width = 300
    Caption = "-"
  END CREATE
END CREATE

SUB Poll
  DIM i AS INTEGER
  FOR i = 1 TO 3
    J.Update
    log = log + STR$(J.IsLeft) + STR$(J.IsRight) + STR$(J.IsUp) + STR$(J.IsDown) + STR$(J.Button(1)) + STR$(J.Button(2)) + " "
  NEXT
  log = log + J.Name + "," + STR$(J.Connected) + "," + STR$(J.X) + "," + STR$(J.Y) + "," + STR$(J.Buttons) + "," + STR$(J.POV) + " "
END SUB

SUB Watch
  ' (from now on the runtime looks for Pad's events while the program waits)
  Pad.OnButtonDown = Down
  Pad.OnButtonUp = Up
  Pad.OnMove = Moved
END SUB

SUB Down(Button AS INTEGER)
  events = events + "down" + STR$(Button) + " "
END SUB

SUB Up(Button AS INTEGER)
  events = events + "up" + STR$(Button) + " "
END SUB

SUB Moved
  events = events + "move" + STR$(Pad.X) + " "
END SUB

SUB Report
  t = TIMER
  WHILE LEN(events) < 26 AND TIMER - t < 3
    DOEVENTS
  WEND
  lbl.Caption = log + "|" + events + "|" + STR$(Pad.Connected)
END SUB

Form.ShowModal
