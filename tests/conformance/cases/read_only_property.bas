' A property RapidQ's manual lists as read-only (R) can't be assigned:
' RapidQ's "Property X of Y is read-only." — in a CREATE or by name.
CREATE Form AS QFORM
  CREATE List AS QLISTBOX
    ItemCount = 2
  END CREATE
END CREATE
List.ItemIndex = 0
List.SelCount = 1
Form.Handle = 5
