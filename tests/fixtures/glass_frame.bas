' QGLASSFRAME (UtilMind's TGlassy, built into RapidQ's runtime): its glass
' colour (TransparentColor) over what's under it at 100 - Transparency
' percent — the form's face under the default black glass (60: a 40 %
' shade), a cyan panel under red glass at 50; Moveable: dragged with the
' left button, its form follows; its clicks.
DECLARE SUB Clicked
CREATE Form AS QFORM
  Caption = "glass frame"
  Left = 100 : Top = 100
  Width = 360
  Height = 220
  CREATE G AS QGLASSFRAME
    Left = 10 : Top = 10
    OnClick = Clicked
  END CREATE
  CREATE P AS QPANEL
    Left = 130 : Top = 10 : Width = 120 : Height = 110
    Color = &HFFFF00
    BevelOuter = 0
    CREATE R AS QGLASSFRAME
      Left = 10 : Top = 10 : Width = 60 : Height = 60
      TransparentColor = &HFF
      Transparency = 50
      Moveable = 0
    END CREATE
  END CREATE
  CREATE Lbl AS QLABEL
    Left = 10 : Top = 140 : Width = 340
    Caption = "-"
  END CREATE
END CREATE
SUB Clicked
  Lbl.Caption = Lbl.Caption + " click" + STR$(Form.Left) + STR$(Form.Top)
END SUB
Form.ShowModal
