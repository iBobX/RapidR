' Members the language registry once listed as the desktop's only, answered
' alike by every runtime (the interpreter, native builds, the web): a status
' bar's Clear (RC.EXE: panels added after it start again at Panel(0)); a
' memory stream's Clear (RC.EXE: Size and Position 0); a QIMAGE's Clear /
' Cls (RapidR's: the picture goes); RapidQ's drawing methods on a list box
' and a combo box that aren't owner-drawn (answered; what they show:
' tests/fixtures/list_drawing.bas).
$INCLUDE "RAPIDQ.INC"
CREATE Form AS QFORM
  CREATE SB AS QSTATUSBAR
  END CREATE
  CREATE Img AS QIMAGE
    Width = 20
    Height = 20
  END CREATE
  CREATE Lst AS QLISTBOX
    AddItems "one", "two"
  END CREATE
  CREATE Cb AS QCOMBOBOX
    AddItems "one", "two"
  END CREATE
END CREATE

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

' a QIMAGE's Clear and Cls
Img.FillRect(0, 0, 20, 20, &HFF)
PRINT "image"; Img.Pixel(5, 5)
Img.Clear
PRINT "cleared"; Img.Pixel(5, 5)
Img.FillRect(0, 0, 20, 20, &HFF00)
Img.Cls
PRINT "cls"; Img.Pixel(5, 5)

' drawing on lists that aren't owner-drawn
Lst.FillRect(0, 0, 50, 20, &HFF)
Lst.Circle(0, 0, 20, 20, &HFF, &HFF00)
Lst.Line(0, 0, 50, 20, 0)
Lst.Paint(30, 5, &HFF0000, 0)
Cb.FillRect(0, 0, 50, 20, &HFF)
Cb.Paint(5, 5, &HFF0000, 0)
PRINT "lists"; Lst.ItemCount; Cb.ItemCount; " "; Lst.Item(0)
