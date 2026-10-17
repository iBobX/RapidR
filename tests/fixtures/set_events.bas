' The events a program's own sets fire on a shown form, as RapidQ's (RC.EXE,
' DEBUG-2): an edit's Text (OnChange), a rich edit's Text (OnChange twice
' once shown), a check box's Checked and a radio button turned on (OnClick),
' a scroll bar's Position (OnChange); a combo box's ItemIndex and a track
' bar's Position nothing; the same value again nothing.
' tests/native_gui_events.mjs.
DECLARE SUB SetAll
DECLARE SUB EChange
DECLARE SUB REChange
DECLARE SUB CKClick
DECLARE SUB RBClick
DECLARE SUB SBChange
DECLARE SUB CBChange
DECLARE SUB TBChange
DIM trail AS STRING
CREATE Form AS QFORM
  Caption = "Sets": Width = 400: Height = 260
  CREATE Go AS QBUTTON
    Left = 10: Top = 10: Caption = "Set"
    OnClick = SetAll
  END CREATE
  CREATE E AS QEDIT
    Left = 100: Top = 10
    OnChange = EChange
  END CREATE
  CREATE RE AS QRICHEDIT
    Left = 100: Top = 40: Width = 120: Height = 40
    OnChange = REChange
  END CREATE
  CREATE CK AS QCHECKBOX
    Left = 10: Top = 90: Caption = "ck"
    OnClick = CKClick
  END CREATE
  CREATE RB AS QRADIOBUTTON
    Left = 10: Top = 115: Caption = "rb"
    OnClick = RBClick
  END CREATE
  CREATE SB AS QSCROLLBAR
    Left = 10: Top = 140: Width = 200
    OnChange = SBChange
  END CREATE
  CREATE CB AS QCOMBOBOX
    Left = 230: Top = 10
    AddItems "zero", "one"
    OnChange = CBChange
  END CREATE
  CREATE TB AS QTRACKBAR
    Left = 230: Top = 40: Width = 150
    OnChange = TBChange
  END CREATE
  CREATE Log AS QLABEL
    Left = 10: Top = 200: Width = 380
  END CREATE
END CREATE
SUB EChange
  trail = trail + " e"
END SUB
SUB REChange
  trail = trail + " r"
END SUB
SUB CKClick
  trail = trail + " c"
END SUB
SUB RBClick
  trail = trail + " b"
END SUB
SUB SBChange
  trail = trail + " s"
END SUB
SUB CBChange
  trail = trail + " CB"
END SUB
SUB TBChange
  trail = trail + " TB"
END SUB
SUB SetAll
  E.Text = "abc": E.Text = "abc"
  RE.Text = "x"
  CK.Checked = 1: CK.Checked = 1
  RB.Checked = 1
  SB.Position = 5: SB.Position = 5
  CB.ItemIndex = 1
  TB.Position = 3
  Log.Caption = "log" + trail
END SUB
Form.ShowModal
