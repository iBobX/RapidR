' A message box over a form: SHOWMESSAGE's dialog.
' rapidr-env: RAPIDR_TEST_MESSAGE_DIALOG=OK RAPIDR_TEST_DIALOG_HOLD=4000
CREATE Form AS QFORM
  Caption = "Message": Left = 40: Top = 40: Width = 260: Height = 120
  CREATE Lbl AS QLABEL
    Caption = "Behind the message": Left = 10: Top = 10
  END CREATE
END CREATE
Form.Show
SHOWMESSAGE "The file has been saved."
