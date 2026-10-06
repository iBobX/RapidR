' Members the language registry once listed as the desktop's only, answered
' alike by every runtime (the interpreter, native builds, the web): Click
' runs OnClick; SetParent; a status bar's Clear (RC.EXE: panels added after
' it start again at Panel(0)); a memory stream's Clear (RC.EXE: Size and
' Position 0); a list view's AddItem / DeleteItem; a popup menu's AddItem;
' an edit's AddItems; a QIMAGE's Clear / Cls; RapidR's canvas names (Rect,
' SetPixel, Ellipse, DrawText) on a QBITMAP, read back by Pixel; drawing on
' a list that isn't owner-drawn draws nothing.
$INCLUDE "RAPIDQ.INC"
DECLARE SUB Clicked
DECLARE SUB Checked
SUB Clicked
  PRINT "clicked"
END SUB
SUB Checked
  PRINT "checked"
END SUB
CREATE Form AS QFORM
  CREATE Btn AS QBUTTON
    OnClick = Clicked
  END CREATE
  CREATE Chk AS QCHECKBOX
    OnClick = Checked
  END CREATE
  CREATE Pnl AS QPANEL
  END CREATE
  CREATE SB AS QSTATUSBAR
  END CREATE
  CREATE LV AS QLISTVIEW
  END CREATE
  CREATE Ed AS QEDIT
    Text = "ab"
  END CREATE
  CREATE Img AS QIMAGE
    Width = 20
    Height = 20
  END CREATE
  CREATE Lst AS QLISTBOX
    AddItems "one", "two"
  END CREATE
END CREATE
CREATE Menu AS QPOPUPMENU
  CREATE First AS QMENUITEM
    Caption = "first"
  END CREATE
END CREATE
DIM Second AS QMENUITEM
Second.Caption = "second"

' Click and SetParent
Btn.Click
Chk.Click
PRINT "checked:"; Chk.Checked
Btn.SetParent(Pnl)
PRINT "parent set"

' a status bar's Clear
SB.AddPanels "one", "two"
PRINT "panels"; SB.PanelCount; " "; SB.Panel(1).Caption
SB.Clear
PRINT "cleared"; SB.PanelCount
SB.AddPanels "three"
PRINT "panels"; SB.PanelCount; " "; SB.Panel(0).Caption

' a memory stream's Clear
DIM M AS QMEMORYSTREAM
M.WriteStr("ABCDEFGHIJ", 10)
M.Position = 4
M.Clear
PRINT "size"; M.Size; " pos"; M.Position
M.WriteStr("XY", 2)
PRINT "size"; M.Size; " pos"; M.Position

' a list view's AddItem / DeleteItem
LV.AddItem "a", "b", "c"
PRINT "items"; LV.ItemCount
LV.DeleteItem 1
PRINT "items"; LV.ItemCount; " "; LV.Item(1).Caption

' a popup menu's AddItem
Menu.AddItem Second
PRINT "menu items"; Menu.ItemCount

' an edit's AddItems
Ed.AddItems "c", "d"
PRINT Ed.Text

' a QIMAGE's Clear and Cls
Img.FillRect(0, 0, 20, 20, &HFF)
PRINT "image"; Img.Pixel(5, 5)
Img.Clear
PRINT "cleared"; Img.Pixel(5, 5)
Img.FillRect(0, 0, 20, 20, &HFF00)
Img.Cls
PRINT "cls"; Img.Pixel(5, 5)

' RapidR's canvas names on a QBITMAP
DIM Bmp AS QBITMAP
Bmp.Width = 40
Bmp.Height = 30
Bmp.FillRect(0, 0, 40, 30, &HFFFFFF)
Bmp.Rect(2, 2, 12, 12, &HFF)
Bmp.SetPixel(20, 5, &HFF00)
Bmp.Ellipse(24, 2, 38, 16, &HFF0000, &HFF0000)
Bmp.DrawText("x", 2, 16, &H0)
PRINT "rect"; Bmp.Pixel(2, 7); " inside"; Bmp.Pixel(7, 7)
PRINT "pixel"; Bmp.Pixel(20, 5)
PRINT "ellipse"; Bmp.Pixel(31, 9)
dark = 0
FOR y = 16 TO 29
  FOR x = 2 TO 12
    IF Bmp.Pixel(x, y) <> &HFFFFFF THEN dark = 1
  NEXT
NEXT
PRINT "text"; dark

' drawing on a list that isn't owner-drawn: nothing
Lst.FillRect(0, 0, 50, 20, &HFF)
Lst.Rect(0, 0, 50, 20, &HFF)
Lst.DrawText("x", 1, 1)
PRINT "list"; Lst.ItemCount; " "; Lst.Item(0)
