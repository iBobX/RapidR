' Timers keep ticking while a dialog waits for the user, as RapidQ's do
' (Windows' dialogs run a modal loop that dispatches WM_TIMER) — in an
' interpreted build as in a native one: the VM serves each dialog's wait
' between instructions, so a timer's handler runs during the dialog, and
' the dialog's answer still comes back to the program when it closes.
' Under the test hooks each dialog stays open RAPIDR_TEST_DIALOG_HOLD ms
' (250), then RAPIDR_TEST_MESSAGE_DIALOG / _FILE_ / _COLOR_ / _FONT_DIALOG
' answer it. Nested: a timer's handler opens a MESSAGEBOX over the
' MESSAGEDLG (the other timer ticks during both), and a timer closes a modal
' form while a box another timer opened over it waits — ShowModal returns
' once the box is answered and its handler is over. Last, two timers fall
' due together and the first one's handler opens a box: the second still
' ticks during it.
'
' Each step logs Ticked(...): -1 when the timer ticked at least 3 times AND
' its first tick came soon after the dialog opened (< 0.15 s) — handlers
' that ran in a burst once the dialog had closed would come ~0.25 s late.
DECLARE SUB Tick
DECLARE SUB Tock
DECLARE SUB Tick3
DECLARE SUB Tock3
DECLARE SUB Opening
DECLARE FUNCTION Ticked (Count AS INTEGER, First AS DOUBLE, Since AS DOUBLE) AS STRING
DIM N AS INTEGER, N0 AS INTEGER, K AS INTEGER, Phase AS INTEGER, R AS INTEGER, Log AS STRING
DIM G AS DOUBLE, D3 AS DOUBLE
' (F / F2: when the timer first ticked since D0 / D2; 0: not yet)
DIM F AS DOUBLE, D0 AS DOUBLE, F2 AS DOUBLE, D2 AS DOUBLE
DIM Od AS QOPENDIALOG
DIM Cd AS QCOLORDIALOG
DIM Fd AS QFONTDIALOG
CREATE Form AS QFORM
  Caption = "dialog timers": Width = 460: Height = 160
  CREATE Lbl AS QLABEL
    Left = 5: Top = 5: Width = 440: Caption = "start"
  END CREATE
  CREATE Lbl2 AS QLABEL
    Left = 5: Top = 35: Width = 440
  END CREATE
END CREATE
CREATE Form2 AS QFORM
  Caption = "Second": Width = 200: Height = 100
END CREATE
' (ticking from the first wait on: Enabled by default)
CREATE T AS QTIMER
  Interval = 20: OnTimer = Tick
END CREATE
CREATE T2 AS QTIMER
  Interval = 30: Enabled = 0: OnTimer = Tock
END CREATE
CREATE T3 AS QTIMER
  Interval = 40: Enabled = 0: OnTimer = Tick3
END CREATE
CREATE T4 AS QTIMER
  Interval = 40: Enabled = 0: OnTimer = Tock3
END CREATE

SUB Tick
  N = N + 1
  IF F = 0 THEN F = TIMER
  IF F2 = 0 THEN F2 = TIMER
  Lbl2.Caption = "tick" + STR$(N)
  ' (the modal form closed by a timer while a box over it waits)
  IF Phase = 2 AND N = N0 + 4 THEN Form2.Close
END SUB

SUB Tock
  DIM M AS INTEGER, A AS INTEGER
  T2.Enabled = 0
  M = N: F2 = 0: D2 = TIMER
  IF Phase = 1 THEN
    ' (over the MESSAGEDLG, a box of its own: T ticks during it)
    A = MESSAGEBOX("Inner?", "Inner", 4)
    Log = Log + "inner" + STR$(A) + Ticked(N - M, F2, D2) + ";"
  ELSE
    A = MESSAGEBOX("Close the second form?", "Second", 1)
    Log = Log + "box" + STR$(A) + Ticked(N - M, F2, D2) + ";"
  END IF
END SUB

SUB Tick3
  K = K + 1
  IF G = 0 THEN G = TIMER
END SUB

SUB Tock3
  DIM M AS INTEGER, A AS INTEGER
  T4.Enabled = 0
  M = K: G = 0: D3 = TIMER
  A = MESSAGEBOX("Go on?", "Both", 1)
  Log = Log + "both" + STR$(A) + Ticked(K - M, G, D3) + ";"
  Lbl.Caption = Log
END SUB

' (a dialog opens now)
SUB Opening
  N0 = N: F = 0: D0 = TIMER
END SUB

FUNCTION Ticked (Count AS INTEGER, First AS DOUBLE, Since AS DOUBLE) AS STRING
  Ticked = STR$(Count >= 3 AND First > Since AND First - Since < 0.15)
END FUNCTION

Form.Show

' MESSAGEDLG (mtConfirmation, Yes / No / Cancel) answered No (mrNo = 7);
' T2's handler opens a MESSAGEBOX (Yes / No) over it, answered Yes (6).
Phase = 1
T2.Enabled = 1
Opening
R = MESSAGEDLG("Save the changes?", 3, 1 OR 2 OR 8, 0)
Log = Log + "dlg" + STR$(R) + Ticked(N - N0, F, D0) + ";"

Opening
SHOWMESSAGE "Saved"
Log = Log + "shown" + Ticked(N - N0, F, D0) + ";"

Opening
R = MSGBOX("Hello")
Log = Log + "msgbox" + STR$(R) + Ticked(N - N0, F, D0) + ";"

Opening
IF Od.Execute THEN Log = Log + "open " + Od.FileName ELSE Log = Log + "cancel"
Log = Log + Ticked(N - N0, F, D0) + ";"

Opening
IF Cd.Execute THEN Log = Log + "color" + HEX$(Cd.Color) ELSE Log = Log + "cancel"
Log = Log + Ticked(N - N0, F, D0) + ";"

Opening
IF Fd.Execute THEN Log = Log + "font " + Fd.Name + STR$(Fd.Size) ELSE Log = Log + "cancel"
Log = Log + Ticked(N - N0, F, D0) + ";"

' A modal form; T2's handler opens a box over it (OK / Cancel, answered OK),
' T closes the form meanwhile: ShowModal returns mrCancel (2) after the box.
Phase = 2
Opening
T2.Enabled = 1
R = Form2.ShowModal
Log = Log + "modal" + STR$(R) + ";"
Lbl.Caption = Log

' Two timers due at once (enabled in turn, then a SLEEP past both: they
' fire as the form's ShowModal starts), T4 first: its handler opens a box
' (answered OK) and T3 still ticks during it — a timer fires once the
' handler of the one before has run, or waits; never queued behind it.
Phase = 3
T4.Enabled = 1
T3.Enabled = 1
SLEEP 0.1
Form.ShowModal
