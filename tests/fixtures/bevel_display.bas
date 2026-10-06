' QBEVEL and QDIGDISPLAY without their include libraries (the manual's
' own examples don't $INCLUDE them): RapidR's built-ins, as QBevel.inc and
' QDigDisplay.inc behave — Shape / Style set the bevels (bsBox, bsFrame)
' or draw two lines at an edge (bsTopLine … bsRightLine); the display is
' 12 × 24 per character of Display, drawn as soon as it changes. The
' capture's pixels: the top line's light row, the right line's dark
' column, the display's lit segments (cyan) and unlit dither.
DECLARE SUB Change
DECLARE SUB Clicked
CREATE Form AS QFORM
  Caption = "bevels and digits"
  ClientWidth = 358
  ClientHeight = 189
  CREATE Top AS QBEVEL
    Left = 10 : Top = 10 : Width = 100 : Height = 40
    Shape = 2 : Style = 1
    Caption = "TopLine"
  END CREATE
  CREATE Box AS QBEVEL
    Left = 120 : Top = 10 : Width = 100 : Height = 40
    Shape = 1
  END CREATE
  CREATE Edge AS QBEVEL
    Left = 230 : Top = 10 : Width = 100 : Height = 40
    Shape = 5
    OnClick = Clicked
  END CREATE
  CREATE Clock AS QDIGDISPLAY
    Left = 10 : Top = 70
    Display = "12:34"
    OnClick = Clicked
  END CREATE
  CREATE Btn AS QBUTTON
    Left = 10 : Top = 110
    Caption = "Change"
    OnClick = Change
  END CREATE
  CREATE Lbl AS QLABEL
    Left = 10 : Top = 150 : Width = 340
    Caption = "-"
  END CREATE
END CREATE
SUB Change
  Box.Style = 1
  Lbl.Caption = Lbl.Caption + STR$(Top.BevelOuter) + STR$(Box.BevelOuter) + STR$(Box.BevelInner)
  Box.Shape = 6
  Lbl.Caption = Lbl.Caption + STR$(Box.BevelOuter) + STR$(Box.BevelInner) + " " + Clock.Display + STR$(Clock.Width) + STR$(Clock.Height)
  Clock.Width = 100
  Lbl.Caption = Lbl.Caption + STR$(Clock.Width) + " " + HEX$(Clock.Pixel(13, 15)) + "|" + HEX$(Clock.Pixel(1, 7)) + "|" + HEX$(Clock.Pixel(1, 8))
END SUB
SUB Clicked
  Lbl.Caption = Lbl.Caption + "c"
END SUB
Form.ShowModal
