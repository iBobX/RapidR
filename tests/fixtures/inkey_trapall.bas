' $OPTION INKEY$ TRAPALL: INKEY$ also returns Shift, Ctrl, Alt, the lock
' keys and the menu key, as extended keys (CHR$(27) + the scan code: Shift
' is 42); $OPTION INKEY$ DEFAULT leaves them out again.
$OPTION INKEY$ TRAPALL
CREATE Form AS QFORM
  Caption = "trapall"
  CREATE Lbl AS QLABEL
    Width = 300
    Caption = "waiting"
  END CREATE
END CREATE
Form.Show
DIM K AS STRING, Got AS STRING, N AS INTEGER
t = TIMER
DO
  K = INKEY$
  IF K <> "" THEN
    Got = Got + STR$(ASC(K)) + ":" + STR$(ASC(RIGHT$(K, 1))) + " "
    N = N + 1
  END IF
  DOEVENTS
LOOP UNTIL N = 2 OR TIMER - t > 8
Lbl.Caption = Got
Form.ShowModal
