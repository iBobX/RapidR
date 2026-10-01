' INPUT$(n): waits for n keys (not echoed) and returns them the moment the
' n-th is pressed — here in the program's window (and in a console
' program's terminal); timers and repaints go on while it waits.
CREATE Form AS QFORM
  Caption = "input$"
  CREATE Lbl AS QLABEL
    Width = 300
    Caption = "waiting"
  END CREATE
END CREATE
Form.Show
DIM A AS STRING
A = INPUT$(3)
Lbl.Caption = "[" + A + "]"
Form.ShowModal
