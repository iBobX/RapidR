' AutoScroll (QFORM, QSCROLLBOX) as Delphi's TScrollingWinControl: a bar
' shows when the components reach past the client area (ClientWidth /
' ClientHeight lose 17 pixels to it); scrolling moves the components (their
' Left / Top); Position stays within 0..Range - client; an arrow moves
' Increment pixels, the track a page (80); no OnMouseDown on the bars;
' AutoScroll off keeps the program's Range. (L takes its caption's width,
' AutoSize: the box's HorzRange is 200 + 20.)
DECLARE SUB Report
DECLARE SUB BoxDown (Button AS INTEGER, X AS INTEGER, Y AS INTEGER, Shift AS INTEGER)
DIM Downs AS INTEGER
CREATE Form AS QFORM
  Caption = "autoscroll": Width = 320: Height = 240
  CREATE Box AS QSCROLLBOX
    Left = 10: Top = 10: Width = 150: Height = 100
    OnMouseDown = BoxDown
    CREATE L AS QLABEL
      Left = 200: Top = 5: Width = 100: Caption = "right"
    END CREATE
  END CREATE
  CREATE Lbl AS QLABEL
    Left = 10: Top = 120: Width = 280
  END CREATE
  CREATE Btn AS QBUTTON
    Left = 10: Top = 150: Caption = "Report"
    OnClick = Report
  END CREATE
  CREATE Far AS QBUTTON
    Left = 10: Top = 400: Width = 75: Height = 25: Caption = "far"
  END CREATE
END CREATE
Lbl.Caption = STR$(Form.ClientWidth) + STR$(Form.ClientHeight) + STR$(Form.VertRange) + STR$(Form.HorzRange) + STR$(Form.AutoScroll) + "|" + STR$(Box.ClientWidth) + STR$(Box.ClientHeight) + STR$(Box.HorzRange)
Form.VertPosition = 99999
Lbl.Caption = Lbl.Caption + "|" + STR$(Form.VertPosition) + STR$(Far.Top) + STR$(Lbl.Top)
Form.VertPosition = 0
Form.ShowModal

SUB BoxDown (Button AS INTEGER, X AS INTEGER, Y AS INTEGER, Shift AS INTEGER)
  Downs = Downs + 1
END SUB

SUB Report
  Lbl.Caption = Lbl.Caption + "|" + STR$(Box.HorzPosition) + STR$(L.Left) + STR$(Downs)
  Box.AutoScroll = 0
  Box.HorzRange = 100
  Lbl.Caption = Lbl.Caption + "|" + STR$(Box.HorzPosition) + STR$(L.Left) + STR$(Box.ClientHeight)
END SUB
