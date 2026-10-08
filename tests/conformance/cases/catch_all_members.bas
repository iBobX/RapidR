' Members a component doesn't have, which RapidR's desktop runtime once
' answered for any component (its generic dispatch: Click, SetParent,
' AddItem, Rect, Repaint …): RC.EXE (RapidQ 2006, run in the Windows VM:
' docs/rapidq-ground-truth.md) stops with "Member X not part of class Y"
' on each, and so does RapidR. Each line below was one probe (RC stops at
' the first error): RC said the same for each.
$APPTYPE CONSOLE
CREATE Form AS QFORM
  CREATE Btn AS QBUTTON
  END CREATE
  CREATE Ed AS QEDIT
  END CREATE
  CREATE LV AS QLISTVIEW
  END CREATE
  CREATE Lst AS QLISTBOX
  END CREATE
  CREATE Cb AS QCOMBOBOX
  END CREATE
  CREATE Grid AS QSTRINGGRID
  END CREATE
  CREATE DX AS QDXSCREEN
  END CREATE
END CREATE
CREATE Menu AS QPOPUPMENU
END CREATE
DIM Bmp AS QBITMAP
Btn.Click
Form.Click
Btn.SetParent
Form.SetParent
Ed.AddItems
LV.AddItem
LV.DeleteItem
LV.Rect
LV.Line
Menu.AddItem
Lst.Rect
Lst.SetPixel
Lst.Ellipse
Lst.DrawText
Cb.Rect
Grid.Rect
DX.Rect
DX.DrawText
Bmp.Rect
Bmp.SetPixel
Bmp.Ellipse
Bmp.DrawText
Bmp.Clear
