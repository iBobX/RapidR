' A program's own sets fire RapidQ's events, inside the set (RC.EXE,
' 2026-10-16, DEBUG-2): an edit's Text that changes its OnChange, a check
' box's Checked its OnClick, a radio button turned on its OnClick, a scroll
' bar's Position its OnChange; the same value again, a combo box's or list
' box's ItemIndex, a track bar's Position fire nothing. (Before the form is
' shown: the rich edit's OnChange once.)
$APPTYPE CONSOLE
DECLARE SUB EChange
DECLARE SUB REChange
DECLARE SUB CKClick
DECLARE SUB RB1Click
DECLARE SUB RB2Click
DECLARE SUB SBChange
DECLARE SUB TBChange
DECLARE SUB CBChange
DECLARE SUB LBClick

CREATE Form AS QFORM
  CREATE E AS QEDIT
    OnChange = EChange
  END CREATE
  CREATE RE AS QRICHEDIT
    OnChange = REChange
  END CREATE
  CREATE CK AS QCHECKBOX
    OnClick = CKClick
  END CREATE
  CREATE RB1 AS QRADIOBUTTON
    OnClick = RB1Click
  END CREATE
  CREATE RB2 AS QRADIOBUTTON
    OnClick = RB2Click
  END CREATE
  CREATE SB AS QSCROLLBAR
    OnChange = SBChange
  END CREATE
  CREATE TB AS QTRACKBAR
    OnChange = TBChange
  END CREATE
  CREATE CB AS QCOMBOBOX
    AddItems "zero", "one"
    OnChange = CBChange
  END CREATE
  CREATE LB AS QLISTBOX
    AddItems "zero", "one"
    OnClick = LBClick
  END CREATE
END CREATE

SUB EChange
  PRINT "  E.OnChange "; E.Text
END SUB
SUB REChange
  PRINT "  RE.OnChange"
END SUB
SUB CKClick
  PRINT "  CK.OnClick"; CK.Checked
END SUB
SUB RB1Click
  PRINT "  RB1.OnClick"
END SUB
SUB RB2Click
  PRINT "  RB2.OnClick"
END SUB
SUB SBChange
  PRINT "  SB.OnChange"; SB.Position
END SUB
SUB TBChange
  PRINT "  TB.OnChange"
END SUB
SUB CBChange
  PRINT "  CB.OnChange"
END SUB
SUB LBClick
  PRINT "  LB.OnClick"
END SUB

PRINT "E.Text = abc": E.Text = "abc": PRINT "."
PRINT "again": E.Text = "abc": PRINT "."
PRINT "E.Text = empty": E.Text = "": PRINT "."
PRINT "RE.Text = xyz": RE.Text = "xyz": PRINT "."
PRINT "again": RE.Text = "xyz": PRINT "."
PRINT "CK.Checked = 1": CK.Checked = 1: PRINT "."
PRINT "again": CK.Checked = 1: PRINT "."
PRINT "CK.Checked = 0": CK.Checked = 0: PRINT "."
PRINT "RB1.Checked = 1": RB1.Checked = 1: PRINT "."
PRINT "RB2.Checked = 1": RB2.Checked = 1: PRINT "."
PRINT "RB2.Checked = 0": RB2.Checked = 0: PRINT "."
PRINT "SB.Position = 10": SB.Position = 10: PRINT "."
PRINT "again": SB.Position = 10: PRINT "."
PRINT "TB.Position = 5": TB.Position = 5: PRINT "."
PRINT "CB.ItemIndex = 1": CB.ItemIndex = 1: PRINT "."
PRINT "LB.ItemIndex = 1": LB.ItemIndex = 1: PRINT "."
PRINT "done"
