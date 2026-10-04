' Accessibility (docs/desktop-host-plan.md §6, Stage 12): what a screen
' reader is told about a form, the same on the desktop (the UI kernel's
' tree) and in the browser (ARIA); tests/web_a11y.mjs compares the two and
' presses the keys. Names from a caption, a Hint, AccessibleName (RapidR's)
' or the QLABEL beside or above a control; AccessibleDescription; a check
' box, radio buttons in a group box, a gauge, an up-down, a panel that's
' disabled (and so its button), a memo Tab passes by (TabStop), a status
' bar (a polite live region); Tab by TabOrder, Alt + a caption's & letter,
' Enter for the Default button and Escape for the Cancel one.
DECLARE SUB OkClick
DECLARE SUB CancelClick
DECLARE SUB RememberClick
CREATE Form AS QFORM
  Caption = "Sign in": Width = 400: Height = 360
  CREATE LblName AS QLABEL
    Caption = "&Name:": Left = 8: Top = 12: Width = 64
  END CREATE
  CREATE EdName AS QEDIT
    Left = 80: Top = 8: Width = 150: Text = "Ann"
  END CREATE
  CREATE LblPass AS QLABEL
    Caption = "Pass&word:": Left = 8: Top = 40: Width = 64
  END CREATE
  CREATE EdPass AS QEDIT
    Left = 80: Top = 36: Width = 150: PasswordChar = "*": Text = "abc"
  END CREATE
  CREATE ChkRemember AS QCHECKBOX
    Caption = "&Remember me": Left = 8: Top = 66: Width = 150: OnClick = RememberClick
  END CREATE
  CREATE GrpShift AS QGROUPBOX
    Caption = "Shift": Left = 8: Top = 92: Width = 170: Height = 56
    CREATE RadDay AS QRADIOBUTTON
      Caption = "&Day": Left = 8: Top = 20: Width = 60: Checked = 1
    END CREATE
    CREATE RadNight AS QRADIOBUTTON
      Caption = "N&ight": Left = 80: Top = 20: Width = 70
    END CREATE
  END CREATE
  CREATE Upload AS QGAUGE
    Left = 8: Top = 156: Width = 170: Height = 18: Position = 30: Max = 60
    AccessibleName = "Upload": AccessibleDescription = "Sending the file"
  END CREATE
  CREATE Copies AS QUPDOWN
    Left = 8: Top = 182: Width = 40: Height = 24: Position = 2: Hint = "Copies to print"
  END CREATE
  CREATE PnlMore AS QPANEL
    Caption = "": Left = 60: Top = 182: Width = 120: Height = 30: Enabled = 0
    CREATE BtnMore AS QBUTTON
      Caption = "&More": Left = 2: Top = 2: Width = 80
    END CREATE
  END CREATE
  CREATE LblNotes AS QLABEL
    Caption = "Notes": Left = 200: Top = 66: Width = 80
  END CREATE
  CREATE MemNotes AS QMEMO
    Left = 200: Top = 84: Width = 180: Height = 64: TabStop = 0
  END CREATE
  CREATE BtnOK AS QBUTTON
    Caption = "OK": Left = 200: Top = 182: Default = 1: Hint = "Signs you in": OnClick = OkClick
  END CREATE
  CREATE BtnCancel AS QBUTTON
    Caption = "Cancel": Left = 290: Top = 182: Cancel = 1: TabOrder = 1: OnClick = CancelClick
  END CREATE
  CREATE Lbl AS QLABEL
    Left = 8: Top = 230: Width = 380
  END CREATE
  CREATE SB AS QSTATUSBAR
    SimpleText = "Ready"
  END CREATE
END CREATE
Lbl.Caption = "ready"

SUB OkClick
  Lbl.Caption = Lbl.Caption + " ok"
  SB.SimpleText = "Signed in as " + EdName.Text
END SUB

SUB CancelClick
  Lbl.Caption = Lbl.Caption + " cancel"
END SUB

SUB RememberClick
  IF ChkRemember.Checked THEN Lbl.Caption = Lbl.Caption + " on" ELSE Lbl.Caption = Lbl.Caption + " off"
END SUB

Form.ShowModal
