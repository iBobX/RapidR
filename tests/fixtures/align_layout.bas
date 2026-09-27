' Align (alTop / alBottom / alLeft / alRight / alClient), laid out as Delphi
' does, checked natively and interpreted by tests/native_gui_events.mjs
' (which also resizes the form as a user would, RAPIDR_TEST_RESIZE, and
' drags the splitter, RAPIDR_TEST_SPLIT).
DECLARE SUB Go
DECLARE SUB Resized
DECLARE SUB Moved
CONST alTop = 1
CONST alLeft = 3
CONST alRight = 4
CONST alClient = 5
CREATE Form AS QFORM
  Caption = "Align": Width = 400: Height = 300
  OnResize = Resized
  CREATE Bar AS QPANEL
    Align = alTop: Height = 40
    CREATE Btn AS QBUTTON
      Left = 5: Top = 5: Caption = "Go": OnClick = Go
    END CREATE
  END CREATE
  CREATE Status AS QSTATUSBAR
    SimpleText = "ready"
  END CREATE
  CREATE Split AS QSPLITTER
    OnMoved = Moved
  END CREATE
  CREATE Tree AS QLISTBOX
    Align = alLeft: Width = 100
  END CREATE
  CREATE Side AS QPANEL
    Align = alRight: Width = 60
  END CREATE
  CREATE Memo AS QRICHEDIT
    Align = alClient
  END CREATE
  CREATE Loose AS QLABEL
    Left = 150: Top = 60: Width = 120: Caption = "free"
  END CREATE
END CREATE
' Laid out before the form is shown.
Loose.Caption = STR$(Memo.Left) + "," + STR$(Memo.Top) + "," + STR$(Memo.Width) + "," + STR$(Memo.Height) + "|" + STR$(Split.Left) + "|" + STR$(Status.Top)

SUB Go
  Side.Visible = 0
  Status.SimpleText = STR$(Memo.Width) + "|" + STR$(Tree.Height) + "|" + STR$(Btn.Left) + "|" + STR$(Bar.Width)
END SUB

SUB Resized
  Bar.Caption = STR$(Form.Width) + "x" + STR$(Form.Height) + "|" + STR$(Memo.Width) + "x" + STR$(Memo.Height) + "|" + STR$(Side.Left) + "|" + STR$(Loose.Left)
END SUB

SUB Moved
  Side.Caption = "moved" + STR$(Tree.Width) + "|" + STR$(Split.Left) + "|" + STR$(Memo.Left)
END SUB

Form.ShowModal
