' A message box's default button — the one Enter presses — opened from a
' button's handler: MESSAGEDLG's DefaultButton (RapidR's fifth argument,
' mrNo = 7) and MESSAGEBOX's MB_DEFBUTTON2 (&H100, as Windows has it).
' Each box gets Enter (and the last Escape) from the program's own
' Application.SendKeys, through the kernel's keyboard as the user's keys:
' Enter answers No, Escape answers as it always did. In a native build the
' box waits inside the handler — the test's next event (Lbl2's click)
' still reaches the program while a box is open, as in the interpreter;
' the test then waits for the label's caption as written (capitals too).
' Checked natively and interpreted by tests/native_gui_events.mjs.
DECLARE SUB Ask
DECLARE SUB Press
DECLARE SUB Noted
DIM Keys AS STRING
DIM Waiting AS INTEGER
CREATE Form AS QFORM
  Caption = "default button": Width = 340: Height = 140
  CREATE Btn AS QBUTTON
    Caption = "Ask": OnClick = Ask
  END CREATE
  CREATE Lbl AS QLABEL
    Top = 40: Width = 320
  END CREATE
  CREATE Lbl2 AS QLABEL
    Top = 70: Width = 320: OnClick = Noted
  END CREATE
END CREATE
CREATE T AS QTIMER
  Interval = 150: Enabled = 0: OnTimer = Press
END CREATE

SUB Press
  T.Enabled = 0
  Application.SendKeys Keys
END SUB

SUB Noted
  IF Waiting THEN Lbl2.Caption = "heard while a box waits" ELSE Lbl2.Caption = "heard after"
END SUB

SUB Ask
  DIM A AS INTEGER
  Waiting = 1
  ' Yes / No, mrNo the default: Enter gives No (7)
  Keys = "{ENTER}": T.Enabled = 1
  A = MESSAGEDLG("This deletes 1,204 rows.", 0, 1 OR 2, 0, 7)
  Lbl.Caption = Lbl.Caption + "Dlg" + STR$(A) + ";"
  ' Yes / No / Cancel by its mb flag (mbNo = 2 is mrCancel's number, and
  ' there is a Cancel: mrCancel wins) — Enter gives Cancel (2)
  Keys = "{ENTER}": T.Enabled = 1
  A = MESSAGEDLG("Save the changes?", 3, 1 OR 2 OR 8, 0, 2)
  Lbl.Caption = Lbl.Caption + "Ync" + STR$(A) + ";"
  ' none given: the first, RapidQ's — Enter gives Yes (6)
  Keys = "{ENTER}": T.Enabled = 1
  A = MESSAGEDLG("Go on?", 3, 1 OR 2, 0)
  Lbl.Caption = Lbl.Caption + "First" + STR$(A) + ";"
  ' MESSAGEBOX, MB_YESNO + MB_DEFBUTTON2: Enter gives IDNO (7)
  Keys = "{ENTER}": T.Enabled = 1
  A = MESSAGEBOX("Delete it?", "Box", 4 OR &H100)
  Lbl.Caption = Lbl.Caption + "Box" + STR$(A) + ";"
  ' Escape keeps its meaning (No, there being no Cancel)
  ' (a longer wait: the test's click comes while a box is open)
  Keys = "{ESC}": T.Interval = 900: T.Enabled = 1
  A = MESSAGEDLG("Escape?", 0, 1 OR 2, 0, 6)
  Lbl.Caption = Lbl.Caption + "Esc" + STR$(A) + ";"
  Waiting = 0
END SUB

Form.ShowModal
