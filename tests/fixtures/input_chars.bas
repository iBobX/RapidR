' INPUT$(n): waits for n keys (not echoed) and returns them the moment the
' n-th is pressed — here in the program's window (and in a console
' program's terminal); timers and repaints go on while it waits. The timer's
' handler runs during the wait (an interpreted build serves it as it serves
' ShowModal): its first tick comes right away (-1), not once a key has come.
DECLARE SUB Tick
DIM N AS INTEGER, F AS DOUBLE, D0 AS DOUBLE
CREATE Form AS QFORM
  Caption = "input$"
  CREATE Lbl AS QLABEL
    Width = 300
    Caption = "waiting"
  END CREATE
  CREATE T AS QTIMER
    Interval = 20: OnTimer = Tick
  END CREATE
END CREATE
SUB Tick
  N = N + 1
  IF F = 0 THEN F = TIMER
END SUB
Form.Show
DIM A AS STRING
D0 = TIMER
A = INPUT$(3)
T.Enabled = 0
Lbl.Caption = "[" + A + "]" + STR$(N >= 3 AND F > D0 AND F - D0 < 0.5)
Form.ShowModal
