' A form's AutoScroll, a QHEADER's Align, a QSTATUSBAR's SimplePanel, as
' RapidQ has them — the .expected is RC.EXE's output
' (docs/rapidq-ground-truth.md): AutoScroll reads 1 and a component past
' the client area takes 17 pixels off it for a scroll bar (gone with
' AutoScroll = 0, back with 1); a header is aligned at the top (Align 1,
' 17 high; its width once the form shows); SimpleText leaves SimplePanel 0.
CREATE F AS QFORM
  Width = 380: Height = 290
  CREATE L AS QLABEL
    Caption = "x": Left = 10: Top = 10: Width = 200: Height = 30: AutoSize = 0
  END CREATE
END CREATE
CREATE G AS QFORM
  Width = 380: Height = 200
  CREATE H AS QHEADER
  END CREATE
  CREATE SB AS QSTATUSBAR
    SimpleText = "Ready"
  END CREATE
END CREATE
cw = F.ClientWidth: ch = F.ClientHeight
PRINT "auto "; F.AutoScroll
L.Top = 400
PRINT "below "; cw - F.ClientWidth; " "; ch - F.ClientHeight
F.AutoScroll = 0
PRINT "off "; F.AutoScroll; " "; cw - F.ClientWidth; " "; ch - F.ClientHeight
F.AutoScroll = 1
PRINT "on "; cw - F.ClientWidth; " "; ch - F.ClientHeight
L.Top = 10: L.Left = 1000
PRINT "right "; cw - F.ClientWidth; " "; ch - F.ClientHeight
L.Left = 10
PRINT "back "; cw - F.ClientWidth; " "; ch - F.ClientHeight
PRINT "header "; H.Align; " "; H.Left; " "; H.Top; " "; H.Height
PRINT "status "; SB.Align; " "; SB.SimplePanel; " "; SB.SimpleText; " "; SB.Height
SB.SimplePanel = 1
PRINT "simple "; SB.SimplePanel
