' A radio button the program turns on turns off the other radio buttons of
' its parent, before its OnClick (which reads them off already), in a CREATE
' block too, before and after its form shows; a radio button of another
' parent keeps its own, and one moved to a new parent is that parent's from
' then on. RC.EXE (2026-11-05, RUNTIME-BUGS; RAPIDR-ISSUES #7): the expected
' output is RapidQ's, byte for byte.
$APPTYPE CONSOLE
DECLARE SUB AClick
DECLARE SUB BClick
DECLARE SUB CClick
CREATE Form AS QFORM
  CREATE A AS QRADIOBUTTON
    Caption = "A"
    Checked = 1
    OnClick = AClick
  END CREATE
  CREATE B AS QRADIOBUTTON
    Caption = "B"
    Top = 24
    OnClick = BClick
  END CREATE
  CREATE P AS QPANEL
    Top = 60
    CREATE C AS QRADIOBUTTON
      Caption = "C"
      Checked = 1
      OnClick = CClick
    END CREATE
    CREATE D AS QRADIOBUTTON
      Caption = "D"
      Top = 20
      Checked = 1
    END CREATE
  END CREATE
END CREATE
SUB AClick
  PRINT "  A.OnClick"; A.Checked; B.Checked
END SUB
SUB BClick
  PRINT "  B.OnClick"; A.Checked; B.Checked
END SUB
SUB CClick
  PRINT "  C.OnClick"; C.Checked; D.Checked
END SUB
PRINT "start"; A.Checked; B.Checked; C.Checked; D.Checked
B.Checked = 1
PRINT "B=1"; A.Checked; B.Checked; C.Checked; D.Checked
A.Checked = 1
PRINT "A=1"; A.Checked; B.Checked; C.Checked; D.Checked
A.Checked = 0
PRINT "A=0"; A.Checked; B.Checked; C.Checked; D.Checked
C.Checked = 1
PRINT "C=1"; A.Checked; B.Checked; C.Checked; D.Checked
B.Checked = 1
PRINT "B=1"; A.Checked; B.Checked; C.Checked; D.Checked
Form.Show
PRINT "shown"; A.Checked; B.Checked; C.Checked; D.Checked
A.Checked = 1
PRINT "A=1"; A.Checked; B.Checked; C.Checked; D.Checked
D.Checked = 1
PRINT "D=1"; A.Checked; B.Checked; C.Checked; D.Checked
C.Parent = Form
PRINT "C to Form"; A.Checked; B.Checked; C.Checked; D.Checked
C.Checked = 1
PRINT "C=1"; A.Checked; B.Checked; C.Checked; D.Checked
Form.Close
PRINT "done"
