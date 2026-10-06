' A tree view: nodes expanded and collapsed, lines and buttons, a selected
' node with the focus; a disabled one.
CREATE Form AS QFORM
  Caption = "Tree view": Left = 40: Top = 40: Width = 340: Height = 220
  CREATE TV AS QTREEVIEW
    Left = 10: Top = 10: Width = 180: Height = 170
  END CREATE
  CREATE TV2 AS QTREEVIEW
    Left = 200: Top = 10: Width = 120: Height = 80: Enabled = 0
  END CREATE
END CREATE
TV.AddItems "Desktop", "Documents", "Music"
TV.AddChildItems 0, "Pictures", "Videos"
TV.AddChildItems 3, "Letters", "Reports"
TV.Expand 0, 0
TV.ItemIndex = 1
TV2.AddItems "One", "Two"
TV2.AddChildItems 0, "Child"
Form.ShowModal
