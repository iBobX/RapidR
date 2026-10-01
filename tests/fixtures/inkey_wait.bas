' INKEY$: a "press a key" loop (RapidQ's manual: DO: LOOP UNTIL INKEY$ <>
' "") — the keys pressed in the program's window (and in a console
' program's terminal); an arrow is CHR$(27) + its scan code, as RapidQ's
' (chapter 6.5).
CREATE Form AS QFORM
  Caption = "inkey"
  CREATE Ed AS QEDIT
    Width = 100
  END CREATE
  CREATE Lbl AS QLABEL
    Top = 40 : Width = 300
    Caption = "waiting"
  END CREATE
END CREATE
Form.Show
DIM K AS STRING, Got AS STRING, N AS INTEGER
t = TIMER
DO
  K = INKEY$
  IF K <> "" THEN
    Got = Got + STR$(ASC(K)) + "/" + STR$(LEN(K)) + " "
    N = N + 1
  END IF
  DOEVENTS
LOOP UNTIL N = 2 OR TIMER - t > 8
Lbl.Caption = Got
Form.ShowModal
