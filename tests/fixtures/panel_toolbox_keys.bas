' RTOOLBOX by the keyboard (docs/ide-components.md 3.6): Down to the
' first components (OnSelect each), "l" goes to QLABEL (type-ahead), Left
' to its group and Left again closes it, Down to the next group, Right to
' its first component, End to the last one, Home to "RapidQ", Enter
' closes it (fewer components shown).
DIM log AS STRING

SUB Picked (T AS STRING)
  log = log + " pick:" + T
END SUB

SUB Selected (T AS STRING)
  log = log + " sel:" + T
END SUB

SUB DragStarted (T AS STRING)
  log = log + " drag:" + T
END SUB

SUB Dropped (T AS STRING, Target AS STRING, X AS INTEGER, Y AS INTEGER)
  log = log + " drop:" + T + ">" + Target + "@" + STR$(X) + "," + STR$(Y)
END SUB

SUB Report
  lbl.Caption = log
  info.Caption = STR$(Box.Count) + " " + Box.Item(0) + " " + Box.Selected + " [" + Box.Filter + "]"
END SUB

CREATE Form AS QFORM
  Caption = "Toolbox"
  Width = 640: Height = 480
  CREATE Box AS RTOOLBOX
    Left = 8: Top = 8: Width = 220: Height = 400
    OnPick = Picked
    OnSelect = Selected
    OnDragStart = DragStarted
    OnDragDrop = Dropped
  END CREATE
  CREATE Surface AS QPANEL
    Left = 300: Top = 40: Width = 300: Height = 240
    CREATE Inner AS QPANEL
      Left = 180: Top = 20: Width = 100: Height = 60
    END CREATE
  END CREATE
  CREATE Lbl AS QLABEL
    Left = 240: Top = 290: Width = 380: Height = 60
    WordWrap = 1
  END CREATE
  CREATE Info AS QLABEL
    Left = 240: Top = 360: Width = 380
  END CREATE
  CREATE BReport AS QBUTTON
    Left = 240: Top = 390: Caption = "Report"
    OnClick = Report
  END CREATE
END CREATE
Form.ShowModal
