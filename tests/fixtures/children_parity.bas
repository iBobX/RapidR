' A form built the same way on every runtime (tests/gui_parity_cases.mjs
' "children_parity"): the desktop and the web share one children index
' (rapidr_value::children) and one scroll-bar engine (rapidr_value::
' scrollbars::engine), so their captures must be the same.
'
' Overlapping buttons parented last first, one moved to a panel and back,
' one made later: each is drawn over the ones made before it (a parent's
' children come in the order they were made). A scroll box whose labels
' reach past its edge, placed by many writes in a row (its bars worked out
' once, before the read): its ranges and client area, and its bars drawn.
' The user's resize (the test hook) lays the form out again: the anchored
' button follows its right edge.
DECLARE SUB Report
CREATE Form AS QFORM
  Caption = "children": Width = 380: Height = 300
  CREATE Holder AS QSCROLLBOX
    Left = 200: Top = 10: Width = 150: Height = 120
  END CREATE
  CREATE Other AS QPANEL
    Left = 10: Top = 170: Width = 80: Height = 40
  END CREATE
  CREATE Lbl AS QLABEL
    Left = 10: Top = 220: Width = 350
  END CREATE
  CREATE Btn AS QBUTTON
    Left = 290: Top = 160: Width = 75: Caption = "Report"
    Anchors = 4 + 2
    OnClick = Report
  END CREATE
END CREATE
DIM B(3) AS QBUTTON
DIM L(5) AS QLABEL
DIM Late AS QBUTTON
DIM i AS INTEGER

FOR i = 3 TO 0 STEP -1
  B(i).Caption = "B" + STR$(i)
  B(i).Left = 10 + i * 30: B(i).Top = 20 + i * 25: B(i).Width = 80: B(i).Height = 40
  B(i).Parent = Form
NEXT
B(1).Parent = Other
B(1).Parent = Form
Late.Caption = "Late": Late.Left = 25: Late.Top = 32: Late.Width = 80: Late.Height = 40
Late.Parent = Form
FOR i = 0 TO 5
  L(i).Parent = Holder
  L(i).Caption = "L" + STR$(i): L(i).AutoSize = 0
  L(i).Left = 10 + i * 40: L(i).Top = 10 + i * 30: L(i).Width = 36: L(i).Height = 18
NEXT
Lbl.Caption = "box" + STR$(Holder.HorzRange) + STR$(Holder.VertRange) + STR$(Holder.ClientWidth) + STR$(Holder.ClientHeight)
Form.ShowModal

SUB Report
  Lbl.Caption = Lbl.Caption + "|" + STR$(Form.ClientWidth) + STR$(Btn.Left) + STR$(Holder.HorzRange) + STR$(Holder.VertRange)
END SUB
