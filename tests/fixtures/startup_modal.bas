' A ShowModal in the main program waits for its form to close before the
' next statement (as the desktop does, also in the browser); Form.Repaint
' fires the form's OnPaint.
DECLARE SUB DlgClose
DECLARE SUB Paint
DECLARE SUB Again
DECLARE SUB Check
DIM Paints AS INTEGER, Base AS INTEGER
CREATE Dlg AS QFORM
  Caption = "first"
  Width = 200
  Height = 100
  CREATE DlgOk AS QBUTTON
    Caption = "OK"
    OnClick = DlgClose
  END CREATE
END CREATE
CREATE Form AS QFORM
  Caption = "startup modal"
  Width = 300
  Height = 160
  OnPaint = Paint
  CREATE Lbl AS QLABEL
    Top = 5
    Width = 280
    Caption = "-"
  END CREATE
  CREATE Rp AS QBUTTON
    Top = 40
    Caption = "Repaint"
    OnClick = Again
  END CREATE
  CREATE Chk AS QBUTTON
    Top = 70
    Caption = "Check"
    OnClick = Check
  END CREATE
  CREATE Lbl2 AS QLABEL
    Top = 100
    Width = 280
    Caption = "-"
  END CREATE
END CREATE
SUB DlgClose
  Dlg.Close
END SUB
SUB Paint
  Paints = Paints + 1
END SUB
SUB Again
  Base = Paints
  Form.Repaint
END SUB
SUB Check
  Lbl2.Caption = "repainted" + STR$(Paints - Base)
END SUB
Lbl.Caption = "before"
Dlg.ShowModal
Lbl.Caption = Lbl.Caption + " after"
Form.ShowModal
