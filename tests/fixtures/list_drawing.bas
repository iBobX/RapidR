' RapidQ's drawing methods on lists and grids, as RapidQ shows them (an
' RC.EXE-built program with this layout, seen in the Windows VM): in an
' owner-drawn list box's OnDrawItem, a combo box's and a grid's OnDrawCell,
' Rectangle, Circle, Line, TextOut and Paint — a flood fill up to the
' border colour — draw on the item or cell; on a list box that isn't
' owner-drawn the drawing stays on the list until it paints rows again
' (selecting an item paints only its row); on a combo box that isn't
' owner-drawn nothing shows. Checked natively and interpreted by
' tests/native_gui_events.mjs; in the browser by tests/web_gui_parity.mjs,
' whose captures must be byte-identical to the desktop's (1x and 2x).
$INCLUDE "RAPIDQ.INC"
DECLARE SUB DrawItem (Index AS INTEGER, State AS INTEGER, R AS QRECT)
DECLARE SUB DrawCombo (Index AS INTEGER, State AS INTEGER, R AS QRECT)
DECLARE SUB DrawCell (Col AS INTEGER, Row AS INTEGER, State AS INTEGER, R AS QRECT)
DECLARE SUB DrawLater
DECLARE SUB Select2
CREATE Form AS QFORM
  Caption = "listdraw"
  ClientWidth = 600
  ClientHeight = 360
  CREATE L1 AS QLISTBOX
    Left = 10
    Top = 10
    Width = 200
    Height = 130
    Style = lbOwnerDrawFixed
    ItemHeight = 30
    OnDrawItem = DrawItem
    AddItems "one", "two", "three"
  END CREATE
  CREATE L2 AS QLISTBOX
    Left = 220
    Top = 10
    Width = 180
    Height = 130
    AddItems "plain one", "plain two", "plain three", "plain four"
  END CREATE
  CREATE C1 AS QCOMBOBOX
    Left = 410
    Top = 10
    Width = 180
    Style = csOwnerDrawFixed
    ItemHeight = 26
    OnDrawItem = DrawCombo
    AddItems "c one", "c two"
    ItemIndex = 0
  END CREATE
  CREATE C2 AS QCOMBOBOX
    Left = 410
    Top = 60
    Width = 180
    AddItems "plain combo", "second"
    ItemIndex = 0
  END CREATE
  CREATE G AS QSTRINGGRID
    Left = 10
    Top = 150
    Width = 330
    Height = 150
    OnDrawCell = DrawCell
  END CREATE
  CREATE Btn AS QBUTTON
    Left = 360
    Top = 150
    Caption = "Draw"
    OnClick = DrawLater
  END CREATE
  CREATE Sel AS QBUTTON
    Left = 360
    Top = 190
    Caption = "Select"
    OnClick = Select2
  END CREATE
  CREATE Lbl AS QLABEL
    Left = 360
    Top = 230
    Caption = "-"
  END CREATE
END CREATE

SUB DrawItem (Index AS INTEGER, State AS INTEGER, R AS QRECT)
  L1.FillRect(R.Left, R.Top, R.Right, R.Bottom, &HFFFFFF)
  L1.Rectangle(R.Left + 2, R.Top + 2, R.Left + 40, R.Bottom - 2, &HFF)
  L1.Paint(R.Left + 10, R.Top + 10, &HFF0000, &HFF)
  L1.Circle(R.Left + 50, R.Top + 2, R.Left + 80, R.Bottom - 2, &H008000, &H00FFFF)
  L1.Line(R.Left + 90, R.Top + 2, R.Left + 120, R.Bottom - 2, &H0)
  L1.TextOut(R.Left + 130, R.Top + 8, L1.Item(Index), &H0, -1)
END SUB

SUB DrawCombo (Index AS INTEGER, State AS INTEGER, R AS QRECT)
  C1.FillRect(R.Left, R.Top, R.Right, R.Bottom, &HFFFFFF)
  C1.Rectangle(R.Left + 2, R.Top + 2, R.Left + 30, R.Bottom - 2, &HFF)
  C1.Paint(R.Left + 8, R.Top + 8, &HFF0000, &HFF)
  C1.TextOut(R.Left + 40, R.Top + 4, C1.Item(Index), &H0, -1)
END SUB

SUB DrawCell (Col AS INTEGER, Row AS INTEGER, State AS INTEGER, R AS QRECT)
  IF Col = 1 AND Row = 1 THEN
    G.FillRect(R.Left, R.Top, R.Right, R.Bottom, &HFFFFFF)
    G.Rectangle(R.Left + 2, R.Top + 2, R.Right - 2, R.Bottom - 2, &HFF)
    G.Paint(R.Left + 6, R.Top + 6, &HFF0000, &HFF)
  END IF
END SUB

SUB DrawLater
  L2.FillRect(0, 0, 80, 40, &HFF)
  L2.Circle(90, 0, 130, 40, &H008000, &H00FFFF)
  L2.Line(0, 50, 170, 120, &H0)
  L2.Paint(150, 100, &HFF00FF, &H0)
  C2.FillRect(2, 2, 40, 18, &HFF)
  C2.Line(0, 0, 180, 24, &H0)
  Lbl.Caption = "drawn"
END SUB

SUB Select2
  L2.ItemIndex = 1
  Lbl.Caption = Lbl.Caption + " selected" + STR$(L2.ItemIndex)
END SUB

Form.ShowModal
