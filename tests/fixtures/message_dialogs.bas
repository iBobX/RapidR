' MESSAGEBOX / MESSAGEDLG / SHOWMESSAGE under a GUI test's hooks
' (RAPIDR_CAPTURE, RAPIDR_TEST_EVENTS): SHOWMESSAGE prints its text and the
' program goes on, as if OK was pressed; MESSAGEDLG shows its dialog
' (mtConfirmation, mbYes OR mbNo OR mbCancel) and waits for it — nobody
' answers, so the test ends with it open (captured with the form) and the
' label still says what was set before it.
CREATE Form AS QFORM
  Caption = "messages"
  Width = 300
  Height = 150
  CREATE Lbl AS QLABEL
    Left = 10 : Top = 10 : Width = 280 : Caption = "start"
  END CREATE
END CREATE
Form.Show
SHOWMESSAGE "Hello from SHOWMESSAGE"
Lbl.Caption = "shown"
DIM Answer AS INTEGER
Answer = MESSAGEDLG("Save the changes to the file" + CHR$(13) + CHR$(10) + "before closing?", 3, 1 OR 2 OR 8, 0)
Lbl.Caption = "answered " + STR$(Answer)
Form.ShowModal
