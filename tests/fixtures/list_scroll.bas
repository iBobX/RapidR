' A QLISTBOX scrolls to what the program picks, as Windows' list box does:
' ItemIndex = i shows that item (the least scrolling), TopIndex = i makes it
' the first row shown (never scrolling past the end of the list). Four lists
' of 50 items, 7 rows and a bit showing, each set by the program before the
' window first shows; the last is a message list: items added, the newest
' one picked (and, on the Send button, three more). The window as captured
' shows the picked item in each (the selected row, blue under the classic
' theme), checked natively and interpreted by tests/native_gui_events.mjs
' and in the browser by tests/web_gui_parity.mjs.
DECLARE SUB Send
DIM I AS INTEGER
CREATE Form AS QFORM
  Caption = "List scroll": Width = 520: Height = 220
  CREATE L1 AS QLISTBOX
    Left = 10: Top = 10: Width = 120: Height = 100
  END CREATE
  CREATE L2 AS QLISTBOX
    Left = 135: Top = 10: Width = 120: Height = 100
  END CREATE
  CREATE L3 AS QLISTBOX
    Left = 260: Top = 10: Width = 120: Height = 100
  END CREATE
  CREATE L4 AS QLISTBOX
    Left = 385: Top = 10: Width = 120: Height = 100
  END CREATE
  CREATE C1 AS QLABEL
    Left = 10: Top = 115: Width = 120: Caption = "ItemIndex = 49"
  END CREATE
  CREATE C2 AS QLABEL
    Left = 135: Top = 115: Width = 120: Caption = "TopIndex = 49"
  END CREATE
  CREATE C3 AS QLABEL
    Left = 260: Top = 115: Width = 120: Caption = "TopIndex = 10"
  END CREATE
  CREATE C4 AS QLABEL
    Left = 385: Top = 115: Width = 120: Caption = "messages"
  END CREATE
  CREATE SendBtn AS QBUTTON
    Left = 385: Top = 135: Width = 120: Caption = "Send three"
    OnClick = Send
  END CREATE
END CREATE

FOR I = 1 TO 50
  L1.AddItems "Item " + STR$(I)
  L2.AddItems "Item " + STR$(I)
  L3.AddItems "Item " + STR$(I)
NEXT I

' (the last item picked: shown at the bottom)
L1.ItemIndex = L1.ItemCount - 1

' (item 46 picked, then the last item made the top: the list shows its
' last page, which has item 46 on it, and no further)
L2.ItemIndex = 45
L2.TopIndex = L2.ItemCount - 1

' (item 13 picked, then item 11 made the top: it is the first row)
L3.ItemIndex = 12
L3.TopIndex = 10

' (a message list: each new message added, then picked)
FOR I = 1 TO 50
  L4.AddItems "Message " + STR$(I)
  L4.ItemIndex = L4.ItemCount - 1
NEXT I

SUB Send
  DIM K AS INTEGER
  FOR K = 1 TO 3
    L4.AddItems "Message " + STR$(L4.ItemCount + 1)
    L4.ItemIndex = L4.ItemCount - 1
  NEXT K
END SUB

Form.ShowModal
