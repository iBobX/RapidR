' QSTATUSBAR's size grip (SizeGrip, True by default, as Delphi's
' TStatusBar): on a sizeable form, docked at its bottom, its bottom-right
' square takes the mouse — no OnMouseDown — and a drag from it resizes the
' window as the user's drag of its border would: Width / Height follow and
' OnResize fires. A press elsewhere on the bar is its OnMouseDown; with
' SizeGrip off the corner is the bar's too.
DECLARE SUB BarDown (Button AS INTEGER, X AS INTEGER, Y AS INTEGER, Shift AS INTEGER)
DECLARE SUB Resized
DECLARE SUB NoGrip
DIM Log AS STRING
CREATE Form AS QFORM
  Caption = "size grip": Width = 320: Height = 240
  OnResize = Resized
  CREATE Bar AS QSTATUSBAR
    SimpleText = "ready"
    OnMouseDown = BarDown
  END CREATE
  CREATE Btn AS QBUTTON
    Left = 5: Top = 5: Caption = "No grip"
    OnClick = NoGrip
  END CREATE
  CREATE Lbl AS QLABEL
    Left = 5: Top = 40: Width = 300: Caption = "-"
  END CREATE
END CREATE
Log = "w" + STR$(Bar.Width) + " g" + STR$(Bar.SizeGrip) + " "
Form.ShowModal

SUB BarDown (Button AS INTEGER, X AS INTEGER, Y AS INTEGER, Shift AS INTEGER)
  Log = Log + "d" + STR$(X) + " ": Lbl.Caption = Log
END SUB
SUB Resized
  Log = Log + "r" + STR$(Form.Width) + "x" + STR$(Form.Height) + " ": Lbl.Caption = Log
END SUB
SUB NoGrip
  Bar.SizeGrip = 0
  Log = Log + "off ": Lbl.Caption = Log
END SUB
