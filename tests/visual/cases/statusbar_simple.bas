' A status bar with simple text, under a group box and a scroll box.
CREATE Form AS QFORM
  Caption = "Status": Left = 40: Top = 40: Width = 320: Height = 180
  CREATE Grp AS QGROUPBOX
    Caption = "Group": Left = 10: Top = 6: Width = 140: Height = 90
  END CREATE
  CREATE Box AS QSCROLLBOX
    Left = 160: Top = 10: Width = 140: Height = 90
    CREATE Inner AS QBUTTON
      Caption = "Far": Left = 150: Top = 120
    END CREATE
  END CREATE
  CREATE SB AS QSTATUSBAR
    SimpleText = "Ready"
  END CREATE
END CREATE
Form.ShowModal
